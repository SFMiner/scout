use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::collections::HashSet;
use tauri::{AppHandle, Manager};
use chrono::Local;
use pulldown_cmark::{Parser, Event};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Config {
    #[serde(rename = "lastProjectPath")]
    last_project_path: Option<String>,
    #[serde(rename = "fontFamily")]
    font_family: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Project {
    title: String,
    author: String,
    #[serde(rename = "chapterOrder")]
    chapter_order: Vec<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Chapter {
    id: u32,
    title: String,
    content: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoadProjectResponse {
    project: Project,
    chapters: Vec<Chapter>,
    path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateProjectResponse {
    project: Project,
    path: String,
}

// Get the config directory for Scout
fn get_config_dir(handle: &AppHandle) -> Result<PathBuf, String> {
    let mut config_dir = handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {}", e))?;
    config_dir.push("Scout");
    Ok(config_dir)
}

// Get the config file path
fn get_config_path(handle: &AppHandle) -> Result<PathBuf, String> {
    let mut path = get_config_dir(handle)?;
    path.push("config.json");
    Ok(path)
}

// Read config from app config directory
#[tauri::command]
fn read_config(handle: AppHandle) -> Result<Config, String> {
    let config_path = get_config_path(&handle)?;

    if !config_path.exists() {
        // Return default config if file doesn't exist
        return Ok(Config {
            last_project_path: None,
            font_family: None,
        });
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))
}

// Write config to app config directory
#[tauri::command]
fn write_config(handle: AppHandle, last_project_path: String) -> Result<(), String> {
    let config_dir = get_config_dir(&handle)?;

    // Create config directory if it doesn't exist
    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let config = Config {
        last_project_path: Some(last_project_path),
        font_family: None,
    };

    let config_path = get_config_path(&handle)?;
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, json)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

// Create a new project at the specified path
#[tauri::command]
fn create_project(path: String, title: String) -> Result<CreateProjectResponse, String> {
    let project_path = PathBuf::from(&path);

    // Ensure path exists
    if !project_path.exists() {
        fs::create_dir_all(&project_path)
            .map_err(|e| format!("Failed to create project directory: {}", e))?;
    }

    // Create chapters subdirectory
    let chapters_dir = project_path.join("chapters");
    fs::create_dir_all(&chapters_dir)
        .map_err(|e| format!("Failed to create chapters directory: {}", e))?;

    // Create project.json
    let project = Project {
        title: title.clone(),
        author: String::new(),
        chapter_order: vec![],
    };

    let project_file = project_path.join("project.json");
    let json = serde_json::to_string_pretty(&project)
        .map_err(|e| format!("Failed to serialize project: {}", e))?;

    fs::write(&project_file, json)
        .map_err(|e| format!("Failed to write project.json: {}", e))?;

    Ok(CreateProjectResponse {
        project,
        path: path.clone(),
    })
}

// Load a project from the specified path
#[tauri::command]
fn load_project(path: String) -> Result<LoadProjectResponse, String> {
    let project_path = PathBuf::from(&path);

    // Validate that project.json exists
    let project_file = project_path.join("project.json");
    if !project_file.exists() {
        return Err("project.json not found in the selected directory".to_string());
    }

    // Read project.json
    let content = fs::read_to_string(&project_file)
        .map_err(|e| format!("Failed to read project.json: {}", e))?;

    let project_data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse project.json: {}", e))?;

    let project = serde_json::from_value::<Project>(project_data.clone())
        .map_err(|e| format!("Failed to parse project: {}", e))?;

    // Load chapter titles from chapterTitles if available
    let chapter_titles = project_data
        .get("chapterTitles")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    // Load chapters from chapters/ directory
    let chapters_dir = project_path.join("chapters");
    let mut chapters: Vec<Chapter> = Vec::new();

    if chapters_dir.exists() {
        let entries = fs::read_dir(&chapters_dir)
            .map_err(|e| format!("Failed to read chapters directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let file_path = entry.path();

            if file_path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(file_name) = file_path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(id) = file_name.parse::<u32>() {
                        let file_content = fs::read_to_string(&file_path)
                            .map_err(|e| format!("Failed to read chapter file: {}", e))?;

                        let content: Option<serde_json::Value> =
                            serde_json::from_str(&file_content).ok();

                        // Get custom title if available, otherwise use default
                        let title = chapter_titles
                            .get(&id.to_string())
                            .and_then(|v| v.as_str())
                            .unwrap_or(&format!("Chapter {}", id))
                            .to_string();

                        chapters.push(Chapter {
                            id,
                            title,
                            content,
                        });
                    }
                }
            }
        }
    }

    // Sort chapters by their position in chapterOrder; unknown IDs go at the end
    let order_map: std::collections::HashMap<u32, usize> = project.chapter_order
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();
    chapters.sort_by_key(|ch| order_map.get(&ch.id).copied().unwrap_or(usize::MAX));

    Ok(LoadProjectResponse {
        project,
        chapters,
        path: path.clone(),
    })
}

// Save a single chapter's content
#[tauri::command]
fn save_chapter(
    project_path: String,
    chapter_id: u32,
    json_content: String,
) -> Result<(), String> {
    let path = PathBuf::from(project_path);
    let chapters_dir = path.join("chapters");

    // Ensure chapters directory exists
    fs::create_dir_all(&chapters_dir)
        .map_err(|e| format!("Failed to create chapters directory: {}", e))?;

    let chapter_file = chapters_dir.join(format!("{}.json", chapter_id));

    // Validate JSON before writing
    serde_json::from_str::<serde_json::Value>(&json_content)
        .map_err(|e| format!("Invalid JSON content: {}", e))?;

    fs::write(&chapter_file, json_content)
        .map_err(|e| format!("Failed to save chapter: {}", e))?;

    Ok(())
}

// Save project metadata (title, author, chapter order).
// Merges into existing project.json to preserve fields the frontend doesn't know about
// (e.g. chapterTitles, exportDir set by other commands).
#[tauri::command]
fn save_project(project_path: String, project_data: serde_json::Value) -> Result<(), String> {
    let path = PathBuf::from(project_path);
    let project_file = path.join("project.json");

    // Read existing data so we don't clobber fields like chapterTitles
    let mut merged: serde_json::Value = if project_file.exists() {
        let content = fs::read_to_string(&project_file)
            .map_err(|e| format!("Failed to read project.json: {}", e))?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Overlay new fields from frontend
    if let (Some(merged_obj), Some(new_obj)) = (merged.as_object_mut(), project_data.as_object()) {
        for (key, value) in new_obj {
            merged_obj.insert(key.clone(), value.clone());
        }
    } else {
        merged = project_data;
    }

    let json = serde_json::to_string_pretty(&merged)
        .map_err(|e| format!("Failed to serialize project: {}", e))?;

    fs::write(&project_file, json)
        .map_err(|e| format!("Failed to write project.json: {}", e))?;

    Ok(())
}

// Convert TipTap JSON content to RTF
// Convert TipTap JSON content to RTF body (without document header/footer)
fn json_to_rtf_content(content: &Option<serde_json::Value>) -> String {
    let mut rtf = String::new();

    if let Some(doc) = content {
        if let Some(nodes) = doc.get("content").and_then(|c| c.as_array()) {
            for node in nodes {
                if let Some(node_type) = node.get("type").and_then(|t| t.as_str()) {
                    match node_type {
                        "paragraph" => {
                            rtf.push_str("{\\pard ");
                            if let Some(node_content) = node.get("content").and_then(|c| c.as_array()) {
                                for item in node_content {
                                    if let Some(marks) = item.get("marks").and_then(|m| m.as_array()) {
                                        let mut is_bold = false;
                                        let mut is_italic = false;
                                        for mark in marks {
                                            if let Some(mark_type) = mark.get("type").and_then(|t| t.as_str()) {
                                                if mark_type == "bold" {
                                                    is_bold = true;
                                                }
                                                if mark_type == "italic" {
                                                    is_italic = true;
                                                }
                                            }
                                        }
                                        if is_bold {
                                            rtf.push_str("\\b ");
                                        }
                                        if is_italic {
                                            rtf.push_str("\\i ");
                                        }
                                    }
                                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                        rtf.push_str(text);
                                    }
                                    rtf.push_str("\\b0\\i0 ");
                                }
                            }
                            rtf.push_str("\\par}\n");
                        }
                        "heading" => {
                            if let Some(level) = node.get("attrs").and_then(|a| a.get("level")).and_then(|l| l.as_u64()) {
                                let font_size = match level {
                                    2 => 32,  // 16pt
                                    3 => 28,  // 14pt
                                    4 => 24,  // 12pt
                                    _ => 20,  // 10pt
                                };
                                rtf.push_str(&format!("{{\\pard \\fs{} \\b ", font_size));
                            } else {
                                rtf.push_str("{\\pard \\fs28 \\b ");
                            }

                            if let Some(node_content) = node.get("content").and_then(|c| c.as_array()) {
                                for item in node_content {
                                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                        rtf.push_str(text);
                                    }
                                }
                            }
                            rtf.push_str("\\b0\\par}\n");
                        }
                        "blockquote" => {
                            rtf.push_str("{\\pard \\li720 ");
                            if let Some(node_content) = node.get("content").and_then(|c| c.as_array()) {
                                for item in node_content {
                                    if let Some(item_content) = item.get("content").and_then(|c| c.as_array()) {
                                        for content_item in item_content {
                                            if let Some(text) = content_item.get("text").and_then(|t| t.as_str()) {
                                                rtf.push_str(text);
                                            }
                                        }
                                    }
                                }
                            }
                            rtf.push_str("\\par}\n");
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    rtf
}

// Get default export directory (parent of project folder)
#[tauri::command]
fn get_default_export_dir(project_path: String) -> Result<String, String> {
    let path = PathBuf::from(&project_path);

    // First check if project.json has exportDir saved
    let project_file = path.join("project.json");
    if project_file.exists() {
        let content = fs::read_to_string(&project_file)
            .map_err(|e| format!("Failed to read project.json: {}", e))?;

        if let Ok(project) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(export_dir) = project.get("exportDir").and_then(|d| d.as_str()) {
                if !export_dir.is_empty() {
                    return Ok(export_dir.to_string());
                }
            }
        }
    }

    // Fall back to parent directory of project path
    if let Some(parent) = path.parent() {
        if let Some(parent_str) = parent.to_str() {
            return Ok(parent_str.to_string());
        }
    }

    Err("Could not determine export directory".to_string())
}

// Update the project's saved export directory
#[tauri::command]
fn update_export_dir(project_path: String, new_export_dir: String) -> Result<(), String> {
    let path = PathBuf::from(&project_path);
    let project_file = path.join("project.json");

    if !project_file.exists() {
        return Err("project.json not found".to_string());
    }

    let content = fs::read_to_string(&project_file)
        .map_err(|e| format!("Failed to read project.json: {}", e))?;

    let mut project: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse project.json: {}", e))?;

    project["exportDir"] = serde_json::json!(new_export_dir);

    let json = serde_json::to_string_pretty(&project)
        .map_err(|e| format!("Failed to serialize project: {}", e))?;

    fs::write(&project_file, json)
        .map_err(|e| format!("Failed to write project.json: {}", e))?;

    Ok(())
}

// Convert plain text to TipTap JSON
fn text_to_tiptap_json(text: &str) -> serde_json::Value {
	let paragraphs: Vec<&str> = text.split("\n\n").collect();
	let mut content = Vec::new();

	for para in paragraphs {
		if !para.trim().is_empty() {
			let para_content = vec![serde_json::json!({
				"type": "text",
				"text": para.trim()
			})];
			content.push(serde_json::json!({
				"type": "paragraph",
				"content": para_content
			}));
		}
	}

	if content.is_empty() {
		content.push(serde_json::json!({
			"type": "paragraph",
			"content": []
		}));
	}

	serde_json::json!({
		"type": "doc",
		"content": content
	})
}

// Convert markdown to TipTap JSON with full formatting support
fn markdown_to_tiptap_json(markdown: &str) -> serde_json::Value {
	let parser = Parser::new(markdown);
	let mut content = Vec::new();
	let mut current_paragraph: Option<Vec<serde_json::Value>> = None;
	let mut in_strong = false;
	let mut in_em = false;
	let mut _in_code = false;
	let mut _in_link = false;
	let mut _link_url = String::new();
	let mut heading_level = 0;
	let mut heading_content: Vec<serde_json::Value> = Vec::new();
	let mut list_stack: Vec<(String, Vec<serde_json::Value>)> = Vec::new(); // (type, items)
	let mut list_item_content: Option<Vec<serde_json::Value>> = None;
	let mut blockquote_content: Vec<serde_json::Value> = Vec::new();
	let mut in_blockquote = false;
	let mut code_block_lang = String::new();
	let mut code_block_content = String::new();
	let mut in_code_block = false;

	for event in parser {
		match event {
			// Block-level events
			Event::Start(tag) => {
				match tag {
					pulldown_cmark::Tag::Heading { level, .. } => {
						heading_level = (level as u8) as u64;
						heading_content.clear();
					}
					pulldown_cmark::Tag::Paragraph => {
						if current_paragraph.is_none() {
							current_paragraph = Some(Vec::new());
						}
					}
					pulldown_cmark::Tag::Strong => {
						in_strong = true;
					}
					pulldown_cmark::Tag::Emphasis => {
						in_em = true;
					}
					pulldown_cmark::Tag::CodeBlock(kind) => {
						in_code_block = true;
						code_block_content.clear();
						code_block_lang = match kind {
							pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
							_ => String::new(),
						};
					}
					pulldown_cmark::Tag::Link { .. } => {
						// Links: extract text as is for now
					}
					pulldown_cmark::Tag::List(ordered) => {
						list_stack.push((
							if ordered.is_some() { "ordered".to_string() } else { "bullet".to_string() },
							Vec::new(),
						));
					}
					pulldown_cmark::Tag::Item => {
						list_item_content = Some(Vec::new());
						// For tight lists, pulldown_cmark emits Text directly inside Item
						// without wrapping it in a Paragraph. Pre-init current_paragraph
						// so those text nodes have somewhere to land.
						if current_paragraph.is_none() {
							current_paragraph = Some(Vec::new());
						}
					}
					pulldown_cmark::Tag::BlockQuote(_) => {
						in_blockquote = true;
						blockquote_content.clear();
					}
					_ => {}
				}
			}
			Event::End(tag) => {
				match tag {
					pulldown_cmark::TagEnd::Heading(_) => {
						if heading_level > 0 {
							content.push(serde_json::json!({
								"type": "heading",
								"attrs": { "level": heading_level },
								"content": heading_content.clone()
							}));
							heading_content.clear();
							heading_level = 0;
						}
					}
					pulldown_cmark::TagEnd::Paragraph => {
						if let Some(para) = current_paragraph.take() {
							if in_blockquote {
								blockquote_content.push(serde_json::json!({
									"type": "paragraph",
									"content": para
								}));
							} else if !list_stack.is_empty() {
								// Part of list item, don't add yet
								if let Some(item) = list_item_content.as_mut() {
									item.push(serde_json::json!({
										"type": "paragraph",
										"content": para
									}));
								}
							} else if !para.is_empty() {
								content.push(serde_json::json!({
									"type": "paragraph",
									"content": para
								}));
							}
						}
					}
					pulldown_cmark::TagEnd::Strong => {
						in_strong = false;
					}
					pulldown_cmark::TagEnd::Emphasis => {
						in_em = false;
					}
					pulldown_cmark::TagEnd::CodeBlock => {
						in_code_block = false;
						content.push(serde_json::json!({
							"type": "codeBlock",
							"attrs": { "language": if code_block_lang.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(code_block_lang.clone()) } },
							"content": [{
								"type": "text",
								"text": code_block_content.clone()
							}]
						}));
						code_block_content.clear();
						code_block_lang.clear();
					}
					pulldown_cmark::TagEnd::Link => {
						// Link end
					}
					pulldown_cmark::TagEnd::List(_) => {
						if let Some((list_type, items)) = list_stack.pop() {
							if !items.is_empty() {
								let node_type = if list_type == "ordered" { "orderedList" } else { "bulletList" };
								content.push(serde_json::json!({
									"type": node_type,
									"content": items
								}));
							}
						}
					}
					pulldown_cmark::TagEnd::Item => {
						// Flush any open paragraph (tight-list text lands here without
						// a wrapping Paragraph event)
						if let Some(para) = current_paragraph.take() {
							if !para.is_empty() {
								if let Some(item) = list_item_content.as_mut() {
									item.push(serde_json::json!({
										"type": "paragraph",
										"content": para
									}));
								}
							}
						}
						if let Some(item_content) = list_item_content.take() {
							if let Some((_, items)) = list_stack.last_mut() {
								if item_content.is_empty() {
									items.push(serde_json::json!({
										"type": "listItem",
										"content": [{
											"type": "paragraph",
											"content": []
										}]
									}));
								} else {
									items.push(serde_json::json!({
										"type": "listItem",
										"content": item_content
									}));
								}
							}
						}
					}
					pulldown_cmark::TagEnd::BlockQuote => {
						in_blockquote = false;
						if !blockquote_content.is_empty() {
							content.push(serde_json::json!({
								"type": "blockquote",
								"content": blockquote_content.clone()
							}));
							blockquote_content.clear();
						}
					}
					_ => {}
				}
			}
			// Inline events
			Event::Text(text) => {
				let mut marks = Vec::new();
				if in_strong {
					marks.push(serde_json::json!({ "type": "bold" }));
				}
				if in_em {
					marks.push(serde_json::json!({ "type": "italic" }));
				}

				let text_node = if marks.is_empty() {
					serde_json::json!({
						"type": "text",
						"text": text.to_string()
					})
				} else {
					serde_json::json!({
						"type": "text",
						"text": text.to_string(),
						"marks": marks
					})
				};

				if in_code_block {
					code_block_content.push_str(&text);
				} else if heading_level > 0 {
					heading_content.push(text_node);
				} else if in_blockquote {
					if let Some(para) = current_paragraph.as_mut() {
						para.push(text_node);
					}
				} else if let Some(para) = current_paragraph.as_mut() {
					para.push(text_node);
				}
			}
			Event::Code(text) => {
				let text_node = serde_json::json!({
					"type": "text",
					"text": text.to_string(),
					"marks": [{ "type": "code" }]
				});

				if in_code_block {
					code_block_content.push_str(&text);
				} else if heading_level > 0 {
					heading_content.push(text_node);
				} else if in_blockquote {
					if let Some(para) = current_paragraph.as_mut() {
						para.push(text_node);
					}
				} else if let Some(para) = current_paragraph.as_mut() {
					para.push(text_node);
				}
			}
			Event::SoftBreak => {
				// Soft break becomes a space
				if let Some(para) = current_paragraph.as_mut() {
					if !para.is_empty() {
						if let Some(last) = para.last_mut() {
							if let Some(text_val) = last.get_mut("text") {
								if let serde_json::Value::String(s) = text_val {
									s.push(' ');
								}
							}
						}
					}
				}
			}
			Event::HardBreak => {
				// Hard break - could end paragraph or just add newline
				// For now treat as space like soft break
				if let Some(para) = current_paragraph.as_mut() {
					if !para.is_empty() {
						if let Some(last) = para.last_mut() {
							if let Some(text_val) = last.get_mut("text") {
								if let serde_json::Value::String(s) = text_val {
									s.push(' ');
								}
							}
						}
					}
				}
			}
			_ => {}
		}
	}

	if content.is_empty() {
		content.push(serde_json::json!({
			"type": "paragraph",
			"content": []
		}));
	}

	serde_json::json!({
		"type": "doc",
		"content": content
	})
}

// Split content by delimiter into sections with titles
fn split_by_delimiter(
	content: &str,
	delimiter: &str,
	extract_titles: bool,
) -> Vec<(String, String)> {
	let lines: Vec<&str> = content.lines().collect();
	let mut sections = Vec::new();
	let mut chapter_num = 1;
	let mut current_title: Option<String> = None;
	let mut current_content = Vec::new();

	for line in lines {
		if line.starts_with(delimiter) {
			// Found a new section - save the previous one if exists
			if let Some(title) = current_title {
				let content_str = current_content.join("\n").trim().to_string();
				if !content_str.is_empty() || !extract_titles {
					sections.push((title, content_str));
					chapter_num += 1;
				}
			}

			// Extract title from this delimiter line
			let title = if extract_titles {
				let after_delim = line.strip_prefix(delimiter).unwrap_or("").trim();
				if !after_delim.is_empty() {
					after_delim.to_string()
				} else {
					format!("Chapter {}", chapter_num)
				}
			} else {
				format!("Chapter {}", chapter_num)
			};

			current_title = Some(title);
			current_content = Vec::new();
		} else if current_title.is_some() {
			// Add line to current section content
			current_content.push(line);
		}
		// Skip lines before the first delimiter
	}

	// Save the last section
	if let Some(title) = current_title {
		let content_str = current_content.join("\n").trim().to_string();
		if !content_str.is_empty() || !extract_titles {
			sections.push((title, content_str));
		}
	}

	// If no sections were created, return original content as one chapter
	if sections.is_empty() {
		sections.push((format!("Chapter 1"), content.to_string()));
	}

	sections
}

// Return a title that isn't already in used_titles, appending (1), (2), … as needed.
// Comparison is case-insensitive; the set stores lowercased titles.
fn make_unique_title(title: &str, used_titles: &HashSet<String>) -> String {
	if !used_titles.contains(&title.to_lowercase()) {
		return title.to_string();
	}
	let mut n = 1;
	loop {
		let candidate = format!("{} ({})", title, n);
		if !used_titles.contains(&candidate.to_lowercase()) {
			return candidate;
		}
		n += 1;
	}
}

// Import chapters from files (text and markdown)
#[tauri::command]
fn import_chapters(
	project_path: String,
	file_paths: Vec<String>,
	use_filename_as_title: bool,
	chapter_delimiter: Option<String>,
	extract_title_from_delimiter: bool,
) -> Result<Vec<Chapter>, String> {
	let project_path_buf = PathBuf::from(&project_path);
	let chapters_dir = project_path_buf.join("chapters");
	let project_file = project_path_buf.join("project.json");

	// Ensure chapters directory exists
	fs::create_dir_all(&chapters_dir)
		.map_err(|e| format!("Failed to create chapters directory: {}", e))?;

	// Read current project to get max chapter ID
	let mut project_data: serde_json::Value = if project_file.exists() {
		let content = fs::read_to_string(&project_file)
			.map_err(|e| format!("Failed to read project.json: {}", e))?;
		serde_json::from_str(&content)
			.map_err(|e| format!("Failed to parse project.json: {}", e))?
	} else {
		serde_json::json!({
			"title": "Project",
			"author": "",
			"chapterOrder": []
		})
	};

	// Get current max ID
	let current_ids: Vec<u32> = if let Some(ids) = project_data.get("chapterOrder").and_then(|v| v.as_array()) {
		ids.iter().filter_map(|id| id.as_u64().map(|i| i as u32)).collect()
	} else {
		Vec::new()
	};
	let max_id = current_ids.iter().max().copied().unwrap_or(0);

	// Seed the deduplication set with all titles already in the project
	let mut used_titles: HashSet<String> = HashSet::new();
	if let Some(titles_obj) = project_data.get("chapterTitles").and_then(|v| v.as_object()) {
		for (_, v) in titles_obj {
			if let Some(t) = v.as_str() {
				used_titles.insert(t.to_lowercase());
			}
		}
	}

	let mut imported_chapters = Vec::new();
	let mut next_id = max_id + 1;

	// Process each file
	for file_path in file_paths {
		let file_path_buf = PathBuf::from(&file_path);

		// Check file extension
		let extension = file_path_buf
			.extension()
			.and_then(|s| s.to_str())
			.unwrap_or("")
			.to_lowercase();

		if extension != "txt" && extension != "md" {
			continue; // Skip unsupported file types
		}

		// Read file content
		let file_content = fs::read_to_string(&file_path)
			.map_err(|e| format!("Failed to read file {}: {}", file_path, e))?;

		// Extract filename for title
		let filename = file_path_buf
			.file_stem()
			.and_then(|s| s.to_str())
			.unwrap_or("Chapter")
			.to_string();

		// If delimiter is provided, try to split the content
		let sections = if let Some(delimiter) = chapter_delimiter.as_ref() {
			split_by_delimiter(&file_content, delimiter, extract_title_from_delimiter)
		} else {
			// No delimiter: treat entire file as one section
			let title = if use_filename_as_title {
				filename
			} else {
				format!("Chapter {}", next_id)
			};
			vec![(title, file_content.clone())]
		};

		// Create a chapter for each section
		for (raw_title, section_content) in sections {
			let section_title = make_unique_title(&raw_title, &used_titles);
			used_titles.insert(section_title.to_lowercase());
			let tiptap_json = if extension == "md" {
				markdown_to_tiptap_json(&section_content)
			} else {
				text_to_tiptap_json(&section_content)
			};

			// Save chapter file
			let chapter_file = chapters_dir.join(format!("{}.json", next_id));
			let json_str = serde_json::to_string_pretty(&tiptap_json)
				.map_err(|e| format!("Failed to serialize chapter: {}", e))?;

			fs::write(&chapter_file, json_str)
				.map_err(|e| format!("Failed to write chapter file: {}", e))?;

			// Add to imported chapters list
			imported_chapters.push(Chapter {
				id: next_id,
				title: section_title,
				content: Some(tiptap_json),
			});

			// Add to project chapter order
			if let Some(order) = project_data.get_mut("chapterOrder").and_then(|v| v.as_array_mut()) {
				order.push(serde_json::Value::Number(next_id.into()));
			}

			next_id += 1;
		}
	}

	// Add all imported chapter titles to chapterTitles for consistency
	if !imported_chapters.is_empty() {
		if !project_data.get("chapterTitles").is_some() {
			project_data["chapterTitles"] = serde_json::json!({});
		}

		if let Some(titles_obj) = project_data.get_mut("chapterTitles").and_then(|v| v.as_object_mut()) {
			for chapter in &imported_chapters {
				titles_obj.insert(chapter.id.to_string(), serde_json::Value::String(chapter.title.clone()));
			}
		}
	}

	// Save updated project.json
	let json = serde_json::to_string_pretty(&project_data)
		.map_err(|e| format!("Failed to serialize project: {}", e))?;

	fs::write(&project_file, json)
		.map_err(|e| format!("Failed to write project.json: {}", e))?;

	Ok(imported_chapters)
}

// Update a chapter's title
#[tauri::command]
fn rename_chapter(project_path: String, chapter_id: u32, new_title: String) -> Result<(), String> {
    let path = PathBuf::from(&project_path);
    let project_file = path.join("project.json");

    if !project_file.exists() {
        return Err("project.json not found".to_string());
    }

    let content = fs::read_to_string(&project_file)
        .map_err(|e| format!("Failed to read project.json: {}", e))?;

    let mut project: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse project.json: {}", e))?;

    // Initialize chapterTitles object if it doesn't exist
    if !project.get("chapterTitles").is_some() {
        project["chapterTitles"] = serde_json::json!({});
    }

    // Update the chapter title
    project["chapterTitles"][chapter_id.to_string()] = serde_json::json!(new_title);

    let json = serde_json::to_string_pretty(&project)
        .map_err(|e| format!("Failed to serialize project: {}", e))?;

    fs::write(&project_file, json)
        .map_err(|e| format!("Failed to write project.json: {}", e))?;

    Ok(())
}

// Update app-level font preference
#[tauri::command]
fn update_font(handle: AppHandle, font_family: String) -> Result<(), String> {
    let config_dir = get_config_dir(&handle)?;
    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let config_path = get_config_path(&handle)?;

    let content = fs::read_to_string(&config_path)
        .unwrap_or_else(|_| "{}".to_string());

    let mut config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    config["fontFamily"] = serde_json::json!(font_family);

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, json)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

// Update project-level font preference
#[tauri::command]
fn update_project_font(project_path: String, font_family: String) -> Result<(), String> {
    let path = PathBuf::from(&project_path);
    let project_file = path.join("project.json");

    if !project_file.exists() {
        return Err("project.json not found".to_string());
    }

    let content = fs::read_to_string(&project_file)
        .map_err(|e| format!("Failed to read project.json: {}", e))?;

    let mut project: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse project.json: {}", e))?;

    project["fontFamily"] = serde_json::json!(font_family);

    let json = serde_json::to_string_pretty(&project)
        .map_err(|e| format!("Failed to serialize project: {}", e))?;

    fs::write(&project_file, json)
        .map_err(|e| format!("Failed to write project.json: {}", e))?;

    Ok(())
}

// Get global dictionary path
fn get_global_dict_path(handle: &AppHandle) -> Result<PathBuf, String> {
	let config_dir = get_config_dir(handle)?;
	let mut path = config_dir;
	path.push("custom_dictionary.json");
	Ok(path)
}

// Get project dictionary path
fn get_project_dict_path(project_path: &str) -> PathBuf {
	PathBuf::from(project_path).join("custom_dictionary.json")
}

// Load dictionary from file
fn load_dictionary(dict_path: &PathBuf) -> Result<Vec<String>, String> {
	if !dict_path.exists() {
		return Ok(Vec::new());
	}

	let content = fs::read_to_string(&dict_path)
		.map_err(|e| format!("Failed to read dictionary: {}", e))?;

	let dict: serde_json::Value = serde_json::from_str(&content)
		.map_err(|e| format!("Failed to parse dictionary: {}", e))?;

	let words = dict.get("words")
		.and_then(|v| v.as_array())
		.map(|arr| {
			arr.iter()
				.filter_map(|v| v.as_str().map(|s| s.to_string()))
				.collect::<Vec<_>>()
		})
		.unwrap_or_default();

	Ok(words)
}

// Save dictionary to file
fn save_dictionary(dict_path: &PathBuf, words: Vec<String>) -> Result<(), String> {
	// Ensure parent directory exists
	if let Some(parent) = dict_path.parent() {
		fs::create_dir_all(parent)
			.map_err(|e| format!("Failed to create dictionary directory: {}", e))?;
	}

	let dict = serde_json::json!({
		"words": words
	});

	let json = serde_json::to_string_pretty(&dict)
		.map_err(|e| format!("Failed to serialize dictionary: {}", e))?;

	fs::write(&dict_path, json)
		.map_err(|e| format!("Failed to write dictionary: {}", e))?;

	Ok(())
}

// Add word to dictionary (global or project-specific)
#[tauri::command]
fn add_to_dictionary(
	handle: AppHandle,
	word: String,
	scope: String,
	project_path: Option<String>,
) -> Result<(), String> {
	let dict_path = if scope == "global" {
		get_global_dict_path(&handle)?
	} else if scope == "project" {
		if let Some(proj_path) = project_path {
			get_project_dict_path(&proj_path)
		} else {
			return Err("Project path required for project-scope dictionary".to_string());
		}
	} else {
		return Err("Invalid scope: use 'global' or 'project'".to_string());
	};

	// Load existing words
	let mut words = load_dictionary(&dict_path)?;

	// Add word if not already present (case-insensitive check)
	let word_lower = word.to_lowercase();
	if !words.iter().any(|w| w.to_lowercase() == word_lower) {
		words.push(word);
		words.sort(); // Keep sorted for readability
	}

	// Save updated dictionary
	save_dictionary(&dict_path, words)?;

	Ok(())
}

// Get all dictionary words (global + project)
#[tauri::command]
fn get_dictionary_words(
	handle: AppHandle,
	project_path: Option<String>,
) -> Result<Vec<String>, String> {
	let mut all_words = Vec::new();

	// Load global dictionary
	if let Ok(global_dict_path) = get_global_dict_path(&handle) {
		if let Ok(words) = load_dictionary(&global_dict_path) {
			all_words.extend(words);
		}
	}

	// Load project dictionary
	if let Some(proj_path) = project_path {
		let proj_dict_path = get_project_dict_path(&proj_path);
		if let Ok(words) = load_dictionary(&proj_dict_path) {
			all_words.extend(words);
		}
	}

	// Remove duplicates and sort
	all_words.sort();
	all_words.dedup();

	Ok(all_words)
}

// Delete a chapter: remove its file and all references in project.json
#[tauri::command]
fn delete_chapter(project_path: String, chapter_id: u32) -> Result<(), String> {
	let path = PathBuf::from(&project_path);
	let project_file = path.join("project.json");

	// Delete the chapter file
	let chapter_file = path.join("chapters").join(format!("{}.json", chapter_id));
	if chapter_file.exists() {
		fs::remove_file(&chapter_file)
			.map_err(|e| format!("Failed to delete chapter file: {}", e))?;
	}

	// Update project.json
	if !project_file.exists() {
		return Ok(());
	}

	let content = fs::read_to_string(&project_file)
		.map_err(|e| format!("Failed to read project.json: {}", e))?;
	let mut project: serde_json::Value = serde_json::from_str(&content)
		.map_err(|e| format!("Failed to parse project.json: {}", e))?;

	// Remove from chapterOrder
	if let Some(order) = project.get_mut("chapterOrder").and_then(|v| v.as_array_mut()) {
		order.retain(|v| v.as_u64().map(|id| id as u32) != Some(chapter_id));
	}

	// Remove from chapterTitles
	if let Some(titles) = project.get_mut("chapterTitles").and_then(|v| v.as_object_mut()) {
		titles.remove(&chapter_id.to_string());
	}

	let json = serde_json::to_string_pretty(&project)
		.map_err(|e| format!("Failed to serialize project: {}", e))?;

	fs::write(&project_file, json)
		.map_err(|e| format!("Failed to write project.json: {}", e))?;

	Ok(())
}

// Export project chapters to RTF file
#[tauri::command]
fn export_project(
    project_path: String,
    export_dir: String,
    chapter_ids: Vec<u32>,
) -> Result<String, String> {
    let project_path_buf = PathBuf::from(&project_path);
    let chapters_dir = project_path_buf.join("chapters");

    // Load project metadata for title
    let project_file = project_path_buf.join("project.json");
    let project_content = fs::read_to_string(&project_file)
        .map_err(|e| format!("Failed to read project.json: {}", e))?;

    let project: Project = serde_json::from_str(&project_content)
        .map_err(|e| format!("Failed to parse project.json: {}", e))?;

    // Determine which chapters to export
    let ids_to_export: Vec<u32> = if chapter_ids.is_empty() {
        project.chapter_order.clone()
    } else {
        chapter_ids
    };

    // Build a single RTF document with all chapters
    let mut rtf_content = String::from("{\\rtf1\\ansi\\ansicpg1252\\cocoartf2\n");
    rtf_content.push_str("{\\colortbl;\\red255\\green255\\blue255;}\n");
    rtf_content.push_str("{\\*\\expandedcolortbl;;}\n");
    rtf_content.push_str("\\margl1440\\margr1440\\margtsxn0\\margbsxn0\\vieww11900\\viewh8605\\viewkind0\n");
    rtf_content.push_str("\\pard\\tx720\\tx1440\\tx2160\\pardirnatural\\partightenfactor200\n\n");

    // Load and add chapter content
    for (i, chapter_id) in ids_to_export.iter().enumerate() {
        let chapter_file = chapters_dir.join(format!("{}.json", chapter_id));

        if chapter_file.exists() {
            // Add chapter title as a heading
            let chapter_title = format!("Chapter {}", chapter_id);
            rtf_content.push_str("{\\pard \\fs28 \\b ");
            rtf_content.push_str(&chapter_title);
            rtf_content.push_str("\\b0\\par}\n");

            // Add spacing (two blank lines)
            rtf_content.push_str("{\\pard \\par}\n");
            rtf_content.push_str("{\\pard \\par}\n");

            let chapter_json = fs::read_to_string(&chapter_file)
                .map_err(|e| format!("Failed to read chapter {}: {}", chapter_id, e))?;

            let chapter_content: Option<serde_json::Value> = serde_json::from_str(&chapter_json).ok();
            rtf_content.push_str(&json_to_rtf_content(&chapter_content));

            // Add page break between chapters (not after the last one)
            if i < ids_to_export.len() - 1 {
                rtf_content.push_str("\\page\n");
            }
        }
    }

    // Close the RTF document
    rtf_content.push_str("}");

    // Generate filename
    let date = Local::now().format("%Y-%m-%d").to_string();
    let filename = if ids_to_export.len() == project.chapter_order.len() {
        format!("{}_{}.rtf", project.title.replace(" ", "_"), date)
    } else {
        let id_range = ids_to_export.iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join("-");
        format!("{}_{}_Chapters_{}.rtf", project.title.replace(" ", "_"), date, id_range)
    };

    // Write RTF file
    let export_path = PathBuf::from(&export_dir).join(&filename);
    fs::write(&export_path, rtf_content)
        .map_err(|e| format!("Failed to write RTF file: {}", e))?;

    // Return the full path to the exported file
    export_path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Failed to convert path to string".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            read_config,
            write_config,
            create_project,
            load_project,
            save_chapter,
            save_project,
            export_project,
            get_default_export_dir,
            update_export_dir,
            import_chapters,
            update_font,
            update_project_font,
            rename_chapter,
            add_to_dictionary,
            get_dictionary_words,
            delete_chapter,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
