use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
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
    #[serde(rename = "fontFamily", skip_serializing_if = "Option::is_none")]
    font_family: Option<String>,
    #[serde(rename = "exportDir", skip_serializing_if = "Option::is_none")]
    export_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    styles: Option<serde_json::Value>,
    #[serde(rename = "pageSettings", skip_serializing_if = "Option::is_none")]
    page_settings: Option<serde_json::Value>,
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
        font_family: None,
        export_dir: None,
        styles: None,
        page_settings: None,
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

// ============================================================
// Asset handling
// ============================================================

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let n = chunk.len();
        let b = [
            chunk[0],
            if n > 1 { chunk[1] } else { 0 },
            if n > 2 { chunk[2] } else { 0 },
        ];
        out.push(CHARS[((b[0] >> 2) & 0x3f) as usize] as char);
        out.push(CHARS[(((b[0] & 0x03) << 4) | ((b[1] >> 4) & 0x0f)) as usize] as char);
        out.push(if n >= 2 { CHARS[(((b[1] & 0x0f) << 2) | ((b[2] >> 6) & 0x03)) as usize] as char } else { '=' });
        out.push(if n >= 3 { CHARS[(b[2] & 0x3f) as usize] as char } else { '=' });
    }
    out
}

fn image_mime_for_ext(ext: &str) -> &'static str {
    let lower = ext.to_lowercase();
    match lower.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png"  => "image/png",
        "gif"  => "image/gif",
        "webp" => "image/webp",
        "svg"  => "image/svg+xml",
        _      => "image/jpeg",
    }
}

/// Copy an image file into the project's assets/ dir and return a data URL.
#[tauri::command]
fn copy_asset_and_encode(
    project_path: String,
    src_path: String,
) -> Result<serde_json::Value, String> {
    let project_path_buf = PathBuf::from(&project_path);
    let assets_dir = project_path_buf.join("assets");
    fs::create_dir_all(&assets_dir)
        .map_err(|e| format!("Failed to create assets directory: {}", e))?;

    let src = PathBuf::from(&src_path);
    let raw_name = src.file_name()
        .ok_or_else(|| "Invalid source path".to_string())?
        .to_string_lossy()
        .to_string();

    // Sanitize filename
    let safe_name: String = raw_name.chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect();

    // Find a non-conflicting destination path
    let dest_path = {
        let candidate = assets_dir.join(&safe_name);
        if !candidate.exists() {
            candidate
        } else {
            let ext = PathBuf::from(&safe_name)
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default();
            let stem_len = safe_name.len().saturating_sub(ext.len());
            let stem = &safe_name[..stem_len];
            let mut n = 1u32;
            loop {
                let c = assets_dir.join(format!("{}_{}{}", stem, n, ext));
                if !c.exists() { break c; }
                n += 1;
            }
        }
    };

    let bytes = fs::read(&src)
        .map_err(|e| format!("Failed to read image: {}", e))?;

    fs::write(&dest_path, &bytes)
        .map_err(|e| format!("Failed to copy image: {}", e))?;

    let final_name = dest_path.file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let ext = dest_path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let mime = image_mime_for_ext(ext);
    let data_url = format!("data:{};base64,{}", mime, base64_encode(&bytes));

    Ok(serde_json::json!({
        "name": final_name,
        "dataUrl": data_url,
    }))
}

// ============================================================
// EPUB export
// ============================================================

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}

/// Pseudo-UUID v4 from FNV hash of seed + current timestamp.
fn generate_epub_uuid(seed: &str) -> String {
    let ts = Local::now().timestamp_millis() as u64;
    let mut h: u64 = 0xcbf29ce484222325;
    for b in seed.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h ^= ts;
    h = h.wrapping_mul(0x100000001b3);
    let a = (h >> 32) as u32;
    let b = ((h >> 16) & 0xffff) as u16;
    let c = 0x4000u16 | ((h >> 4) & 0x0fff) as u16;
    let d = 0x8000u16 | ((h >> 2) & 0x3fff) as u16;
    let e = h.wrapping_mul(0x9e3779b97f4a7c15) & 0xffffffffffff;
    format!("{:08x}-{:04x}-{:04x}-{:04x}-{:012x}", a, b, c, d, e)
}

/// Render TipTap inline content (text nodes + hardBreak) to XHTML.
fn render_inline(items: &[serde_json::Value]) -> String {
    let mut out = String::new();
    for item in items {
        match item.get("type").and_then(|v| v.as_str()).unwrap_or("") {
            "hardBreak" => out.push_str("<br/>"),
            "text" => {
                let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("");
                let empty = vec![];
                let marks = item.get("marks").and_then(|m| m.as_array()).unwrap_or(&empty);
                // Open marks
                for mark in marks.iter() {
                    match mark.get("type").and_then(|v| v.as_str()).unwrap_or("") {
                        "bold"   => out.push_str("<strong>"),
                        "italic" => out.push_str("<em>"),
                        "strike" => out.push_str("<s>"),
                        "code"   => out.push_str("<code>"),
                        "textStyle" => {
                            let a = mark.get("attrs");
                            let fs = a.and_then(|x| x.get("fontSize")).and_then(|v| v.as_f64());
                            let ff = a.and_then(|x| x.get("fontFamily")).and_then(|v| v.as_str()).filter(|s| !s.is_empty());
                            if fs.is_some() || ff.is_some() {
                                let mut style = String::new();
                                if let Some(sz) = fs  { style.push_str(&format!("font-size:{}pt;", sz)); }
                                if let Some(fm) = ff  { style.push_str(&format!("font-family:{};", escape_xml(fm))); }
                                out.push_str(&format!("<span style=\"{}\">", style));
                            }
                        }
                        _ => {}
                    }
                }
                out.push_str(&escape_xml(text));
                // Close marks in reverse
                for mark in marks.iter().rev() {
                    match mark.get("type").and_then(|v| v.as_str()).unwrap_or("") {
                        "bold"   => out.push_str("</strong>"),
                        "italic" => out.push_str("</em>"),
                        "strike" => out.push_str("</s>"),
                        "code"   => out.push_str("</code>"),
                        "textStyle" => {
                            let a = mark.get("attrs");
                            let fs = a.and_then(|x| x.get("fontSize")).and_then(|v| v.as_f64());
                            let ff = a.and_then(|x| x.get("fontFamily")).and_then(|v| v.as_str()).filter(|s| !s.is_empty());
                            if fs.is_some() || ff.is_some() { out.push_str("</span>"); }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Render TipTap block nodes to XHTML.
fn render_blocks(nodes: &[serde_json::Value]) -> String {
    let mut out = String::new();
    for node in nodes {
        let t = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let align = node.get("attrs")
            .and_then(|a| a.get("textAlign"))
            .and_then(|v| v.as_str())
            .filter(|&a| a != "left");
        let style = align.map(|a| format!(" style=\"text-align:{}\"", a)).unwrap_or_default();

        match t {
            "paragraph" => {
                let inner = node.get("content").and_then(|c| c.as_array())
                    .map(|items| render_inline(items)).unwrap_or_default();
                if inner.is_empty() {
                    out.push_str(&format!("<p{}>&#160;</p>\n", style));
                } else {
                    out.push_str(&format!("<p{}>{}</p>\n", style, inner));
                }
            }
            "heading" => {
                let level = node.get("attrs").and_then(|a| a.get("level"))
                    .and_then(|v| v.as_u64()).unwrap_or(2).clamp(2, 6);
                let inner = node.get("content").and_then(|c| c.as_array())
                    .map(|items| render_inline(items)).unwrap_or_default();
                out.push_str(&format!("<h{}{}>{}</h{}>\n", level, style, inner, level));
            }
            "blockquote" => {
                out.push_str("<blockquote>\n");
                if let Some(inner) = node.get("content").and_then(|c| c.as_array()) {
                    out.push_str(&render_blocks(inner));
                }
                out.push_str("</blockquote>\n");
            }
            "bulletList" | "orderedList" => {
                let tag = if t == "bulletList" { "ul" } else { "ol" };
                out.push_str(&format!("<{}>\n", tag));
                if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
                    for item in items {
                        out.push_str("<li>");
                        if let Some(item_content) = item.get("content").and_then(|c| c.as_array()) {
                            for para in item_content {
                                if let Some(inline) = para.get("content").and_then(|c| c.as_array()) {
                                    out.push_str(&render_inline(inline));
                                }
                            }
                        }
                        out.push_str("</li>\n");
                    }
                }
                out.push_str(&format!("</{}>\n", tag));
            }
            "horizontalRule" => out.push_str("<hr/>\n"),
            "colorBleed" => {
                let bg = node.get("attrs").and_then(|a| a.get("backgroundColor"))
                    .and_then(|v| v.as_str()).unwrap_or("#000000");
                let text = node.get("attrs").and_then(|a| a.get("textColor"))
                    .and_then(|v| v.as_str()).unwrap_or("#ffffff");
                out.push_str(&format!(
                    "<div style=\"background-color:{};color:{};margin:0 -2em;padding:2em;\">\n",
                    escape_xml(bg), escape_xml(text)
                ));
                if let Some(inner) = node.get("content").and_then(|c| c.as_array()) {
                    out.push_str(&render_blocks(inner));
                }
                out.push_str("</div>\n");
            }
            "imageBleed" => {
                let name = node.get("attrs").and_then(|a| a.get("name"))
                    .and_then(|v| v.as_str()).unwrap_or("");
                let alt = node.get("attrs").and_then(|a| a.get("alt"))
                    .and_then(|v| v.as_str()).unwrap_or("");
                if !name.is_empty() {
                    out.push_str(&format!(
                        "<div class=\"image-bleed\"><img src=\"../images/{}\" alt=\"{}\"/></div>\n",
                        escape_xml(name), escape_xml(alt)
                    ));
                }
            }
            _ => {}
        }
    }
    out
}

/// Collect all imageBleed asset names from a chapter's TipTap JSON.
fn collect_image_names(content: &Option<serde_json::Value>) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(doc) = content {
        if let Some(nodes) = doc.get("content").and_then(|c| c.as_array()) {
            for node in nodes {
                collect_image_names_from_node(node, &mut names);
            }
        }
    }
    names
}

fn collect_image_names_from_node(node: &serde_json::Value, names: &mut Vec<String>) {
    if node.get("type").and_then(|v| v.as_str()) == Some("imageBleed") {
        if let Some(name) = node.get("attrs")
            .and_then(|a| a.get("name"))
            .and_then(|v| v.as_str())
        {
            let s = name.to_string();
            if !s.is_empty() && !names.contains(&s) {
                names.push(s);
            }
        }
    }
    if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
        for child in children {
            collect_image_names_from_node(child, names);
        }
    }
}

fn chapter_to_xhtml(title: &str, content: &Option<serde_json::Value>) -> String {
    let body = content.as_ref()
        .and_then(|doc| doc.get("content").and_then(|c| c.as_array()))
        .map(|nodes| render_blocks(nodes))
        .unwrap_or_default();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE html>\n\
         <html xmlns=\"http://www.w3.org/1999/xhtml\">\n\
         <head>\n<title>{title}</title>\n\
         <link rel=\"stylesheet\" type=\"text/css\" href=\"../style.css\"/>\n\
         </head>\n<body>\n{body}</body>\n</html>\n",
        title = escape_xml(title), body = body
    )
}

fn build_opf(title: &str, author: &str, uuid: &str, modified: &str, n: usize, images: &[String]) -> String {
    let author_el = if !author.is_empty() {
        format!("    <dc:creator>{}</dc:creator>\n", escape_xml(author))
    } else { String::new() };
    let manifest: String = (0..n).map(|i| format!(
        "    <item id=\"ch{i:03}\" href=\"chapters/ch{i:03}.xhtml\" media-type=\"application/xhtml+xml\"/>\n",
        i = i + 1
    )).collect();
    let image_manifest: String = images.iter().map(|img| {
        let ext = std::path::Path::new(img.as_str())
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let mime = image_mime_for_ext(ext);
        // Use a safe ID (replace non-alphanumeric with _)
        let id: String = img.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
            .collect();
        format!("    <item id=\"img-{id}\" href=\"images/{img}\" media-type=\"{mime}\"/>\n",
            id = id, img = escape_xml(img), mime = mime)
    }).collect();
    let spine: String = (0..n).map(|i| format!(
        "    <itemref idref=\"ch{:03}\"/>\n", i + 1
    )).collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"book-id\">\n\
           <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n\
             <dc:identifier id=\"book-id\">urn:uuid:{uuid}</dc:identifier>\n\
             <dc:title>{title}</dc:title>\n\
         {author_el}    <dc:language>en</dc:language>\n\
             <meta property=\"dcterms:modified\">{modified}</meta>\n\
           </metadata>\n\
           <manifest>\n\
             <item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n\
             <item id=\"ncx\" href=\"toc.ncx\" media-type=\"application/x-dtbncx+xml\"/>\n\
             <item id=\"css\" href=\"style.css\" media-type=\"text/css\"/>\n\
         {manifest}{image_manifest}  </manifest>\n\
           <spine toc=\"ncx\">\n\
         {spine}  </spine>\n\
         </package>",
        uuid = uuid, title = escape_xml(title),
        author_el = author_el, modified = modified,
        manifest = manifest, image_manifest = image_manifest, spine = spine
    )
}

fn build_nav(title: &str, chapter_titles: &[String]) -> String {
    let items: String = chapter_titles.iter().enumerate().map(|(i, t)| format!(
        "      <li><a href=\"chapters/ch{:03}.xhtml\">{}</a></li>\n", i + 1, escape_xml(t)
    )).collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE html>\n\
         <html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\n\
         <head><title>{title}</title></head>\n\
         <body>\n  <nav epub:type=\"toc\">\n    <h1>{title}</h1>\n    <ol>\n\
         {items}    </ol>\n  </nav>\n</body>\n</html>",
        title = escape_xml(title), items = items
    )
}

fn build_ncx(title: &str, uuid: &str, chapter_titles: &[String]) -> String {
    let nav_points: String = chapter_titles.iter().enumerate().map(|(i, t)| format!(
        "    <navPoint id=\"ch{i:03}\" playOrder=\"{ord}\">\n\
           <navLabel><text>{title}</text></navLabel>\n\
           <content src=\"chapters/ch{i:03}.xhtml\"/>\n\
         </navPoint>\n",
        i = i + 1, ord = i + 1, title = escape_xml(t)
    )).collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <ncx xmlns=\"http://www.daisy.org/z3986/2005/ncx/\" version=\"2005-1\">\n\
           <head>\n\
             <meta name=\"dtb:uid\" content=\"urn:uuid:{uuid}\"/>\n\
             <meta name=\"dtb:depth\" content=\"1\"/>\n\
             <meta name=\"dtb:totalPageCount\" content=\"0\"/>\n\
             <meta name=\"dtb:maxPageNumber\" content=\"0\"/>\n\
           </head>\n\
           <docTitle><text>{title}</text></docTitle>\n\
           <navMap>\n{nav_points}  </navMap>\n\
         </ncx>",
        uuid = uuid, title = escape_xml(title), nav_points = nav_points
    )
}

const EPUB_CSS: &str = "\
body { font-family: serif; font-size: 1em; line-height: 1.6; margin: 0; padding: 0; }\n\
p { margin: 0 0 1em; orphans: 2; widows: 2; }\n\
h2 { font-size: 1.5em; font-weight: bold; margin: 1.5em 0 0.5em; page-break-after: avoid; }\n\
h3 { font-size: 1.3em; font-weight: bold; margin: 1.5em 0 0.5em; page-break-after: avoid; }\n\
h4 { font-size: 1.1em; font-weight: bold; margin: 1.5em 0 0.5em; page-break-after: avoid; }\n\
h5 { font-size: 1em;   font-weight: bold; margin: 1.5em 0 0.5em; page-break-after: avoid; }\n\
h6 { font-size: 0.9em; font-weight: bold; margin: 1.5em 0 0.5em; page-break-after: avoid; }\n\
blockquote { margin: 1em 2em; font-style: italic; }\n\
ul, ol { margin: 0 0 1em; padding-left: 2em; }\n\
li { margin: 0.25em 0; }\n\
hr { border: none; border-top: 1px solid #ccc; margin: 2em 0; }\n\
strong { font-weight: bold; }\n\
em { font-style: italic; }\n\
s { text-decoration: line-through; }\n\
code { font-family: monospace; font-size: 0.9em; }";

#[tauri::command]
fn export_epub(
    project_path: String,
    export_dir: String,
    chapter_ids: Vec<u32>,
) -> Result<String, String> {
    use zip::write::SimpleFileOptions;
    use zip::CompressionMethod;

    let project_path_buf = PathBuf::from(&project_path);
    let chapters_dir = project_path_buf.join("chapters");

    // Load project metadata
    let project_file = project_path_buf.join("project.json");
    let project_content = fs::read_to_string(&project_file)
        .map_err(|e| format!("Failed to read project.json: {}", e))?;
    let project_value: serde_json::Value = serde_json::from_str(&project_content)
        .map_err(|e| format!("Failed to parse project.json: {}", e))?;
    let project: Project = serde_json::from_value(project_value.clone())
        .map_err(|e| format!("Failed to parse project: {}", e))?;

    // Load chapter titles map (stored separately from Project struct)
    let chapter_titles_map = project_value
        .get("chapterTitles")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    // Determine chapters to export, maintaining project order
    let ids_to_export: Vec<u32> = if chapter_ids.is_empty() {
        project.chapter_order.clone()
    } else {
        project.chapter_order.iter()
            .filter(|id| chapter_ids.contains(id))
            .copied()
            .collect()
    };

    // Load chapter content and titles
    let mut chapters: Vec<(String, Option<serde_json::Value>)> = Vec::new();
    for &id in &ids_to_export {
        let chapter_file = chapters_dir.join(format!("{}.json", id));
        let content = if chapter_file.exists() {
            let s = fs::read_to_string(&chapter_file)
                .map_err(|e| format!("Failed to read chapter {}: {}", id, e))?;
            serde_json::from_str(&s).ok()
        } else {
            None
        };
        let title = chapter_titles_map
            .get(&id.to_string())
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("Chapter {}", id))
            .to_string();
        chapters.push((title, content));
    }

    let uuid = generate_epub_uuid(&project.title);
    let modified = Local::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let date = Local::now().format("%Y-%m-%d").to_string();

    let safe_title: String = project.title.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let filename = format!("{}_{}.epub", safe_title, date);
    let export_path = PathBuf::from(&export_dir).join(&filename);

    let file = fs::File::create(&export_path)
        .map_err(|e| format!("Failed to create EPUB file: {}", e))?;
    let mut zip = zip::ZipWriter::new(file);

    let stored   = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    // mimetype — must be first entry, uncompressed
    zip.start_file("mimetype", stored).map_err(|e| e.to_string())?;
    zip.write_all(b"application/epub+zip").map_err(|e| e.to_string())?;

    // META-INF/container.xml
    zip.start_file("META-INF/container.xml", deflated).map_err(|e| e.to_string())?;
    zip.write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
        <container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\n\
          <rootfiles>\n\
            <rootfile full-path=\"OEBPS/content.opf\" media-type=\"application/oebps-package+xml\"/>\n\
          </rootfiles>\n\
        </container>").map_err(|e| e.to_string())?;

    // Collect all image filenames referenced by imageBleed nodes
    let mut all_image_names: Vec<String> = Vec::new();
    for (_, content) in &chapters {
        for name in collect_image_names(content) {
            if !all_image_names.contains(&name) {
                all_image_names.push(name);
            }
        }
    }

    // OEBPS/style.css
    zip.start_file("OEBPS/style.css", deflated).map_err(|e| e.to_string())?;
    zip.write_all(EPUB_CSS.as_bytes()).map_err(|e| e.to_string())?;

    // OEBPS/images/* — embed any referenced images
    for img_name in &all_image_names {
        let img_path = project_path_buf.join("assets").join(img_name);
        if img_path.exists() {
            let img_bytes = fs::read(&img_path)
                .map_err(|e| format!("Failed to read image {}: {}", img_name, e))?;
            zip.start_file(&format!("OEBPS/images/{}", img_name), deflated)
                .map_err(|e| e.to_string())?;
            zip.write_all(&img_bytes).map_err(|e| e.to_string())?;
        }
    }

    // OEBPS/chapters/chNNN.xhtml — one file per chapter
    let chapter_titles: Vec<String> = chapters.iter().map(|(t, _)| t.clone()).collect();
    for (i, (title, content)) in chapters.iter().enumerate() {
        let fname = format!("OEBPS/chapters/ch{:03}.xhtml", i + 1);
        zip.start_file(&fname, deflated).map_err(|e| e.to_string())?;
        zip.write_all(chapter_to_xhtml(title, content).as_bytes()).map_err(|e| e.to_string())?;
    }

    // OEBPS/nav.xhtml (EPUB 3 navigation document)
    zip.start_file("OEBPS/nav.xhtml", deflated).map_err(|e| e.to_string())?;
    zip.write_all(build_nav(&project.title, &chapter_titles).as_bytes()).map_err(|e| e.to_string())?;

    // OEBPS/toc.ncx (EPUB 2 compatibility)
    zip.start_file("OEBPS/toc.ncx", deflated).map_err(|e| e.to_string())?;
    zip.write_all(build_ncx(&project.title, &uuid, &chapter_titles).as_bytes()).map_err(|e| e.to_string())?;

    // OEBPS/content.opf (package document)
    zip.start_file("OEBPS/content.opf", deflated).map_err(|e| e.to_string())?;
    zip.write_all(
        build_opf(&project.title, &project.author, &uuid, &modified, chapters.len(), &all_image_names).as_bytes()
    ).map_err(|e| e.to_string())?;

    zip.finish().map_err(|e| format!("Failed to finalize EPUB: {}", e))?;

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
            export_epub,
            copy_asset_and_encode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
