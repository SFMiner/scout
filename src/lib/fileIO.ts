import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import {
	type Chapter,
	type Config,
	type Project,
	type ProjectStyles,
	type PageSettings,
	type LoadProjectResponse,
	type CreateProjectResponse,
	type TipTapJSON,
} from './types';

/**
 * Read the config file from the app config directory
 */
export async function readConfig(): Promise<Config> {
	try {
		return await invoke<Config>('read_config');
	} catch (error) {
		console.error('Failed to read config:', error);
		return { lastProjectPath: null };
	}
}

/**
 * Write the lastProjectPath to the config file
 */
export async function writeConfig(lastProjectPath: string): Promise<void> {
	try {
		await invoke('write_config', { lastProjectPath });
	} catch (error) {
		console.error('Failed to write config:', error);
		throw error;
	}
}

/**
 * Create a new project with the given title in the selected directory
 */
export async function createProject(title: string): Promise<CreateProjectResponse> {
	try {
		// Open folder picker
		const selectedPath = await open({
			directory: true,
			title: 'Choose location for new project',
		});

		if (!selectedPath) {
			throw new Error('No directory selected');
		}

		// Create project folder
		const projectPath = `${selectedPath}/${title.replace(/\s+/g, '_')}`;

		const response = await invoke<CreateProjectResponse>('create_project', {
			path: projectPath,
			title,
		});

		// Save the path as the last opened project
		await writeConfig(response.path);

		return response;
	} catch (error) {
		console.error('Failed to create project:', error);
		throw error;
	}
}

/**
 * Open an existing project from a user-selected directory
 */
export async function openProject(): Promise<LoadProjectResponse> {
	try {
		// Open folder picker
		const selectedPath = await open({
			directory: true,
			title: 'Select project directory',
		});

		if (!selectedPath) {
			throw new Error('No directory selected');
		}

		// Load project
		const response = await invoke<LoadProjectResponse>('load_project', {
			path: selectedPath,
		});

		// Save the path as the last opened project
		await writeConfig(response.path);

		return response;
	} catch (error) {
		console.error('Failed to open project:', error);
		throw error;
	}
}

/**
 * Open the last project without showing the folder picker
 */
export async function openRecentProject(projectPath: string): Promise<LoadProjectResponse> {
	try {
		const response = await invoke<LoadProjectResponse>('load_project', {
			path: projectPath,
		});

		return response;
	} catch (error) {
		console.error('Failed to open recent project:', error);
		throw error;
	}
}

/**
 * Save a single chapter's content
 */
export async function saveChapter(
	projectPath: string,
	chapterId: number,
	content: TipTapJSON | null
): Promise<void> {
	try {
		const jsonContent = JSON.stringify(content || { type: 'doc', content: [] });
		await invoke('save_chapter', {
			projectPath,
			chapterId,
			jsonContent,
		});
	} catch (error) {
		console.error('Failed to save chapter:', error);
		throw error;
	}
}

/**
 * Save project metadata (title, author, chapter order)
 */
export async function saveProjectMetadata(
	projectPath: string,
	project: Project
): Promise<void> {
	try {
		const projectData = {
			title: project.title,
			author: project.author,
			chapterOrder: project.chapterOrder,
			...(project.exportDir && { exportDir: project.exportDir }),
			...(project.fontFamily && { fontFamily: project.fontFamily }),
		};

		await invoke('save_project', {
			projectPath,
			projectData,
		});
	} catch (error) {
		console.error('Failed to save project:', error);
		throw error;
	}
}

/**
 * Get the default export directory for a project
 */
export async function getDefaultExportDir(projectPath: string): Promise<string> {
	try {
		return await invoke<string>('get_default_export_dir', {
			projectPath,
		});
	} catch (error) {
		console.error('Failed to get default export dir:', error);
		throw error;
	}
}

/**
 * Update the saved export directory for a project
 */
export async function saveExportDir(
	projectPath: string,
	exportDir: string
): Promise<void> {
	try {
		await invoke('update_export_dir', {
			projectPath,
			newExportDir: exportDir,
		});
	} catch (error) {
		console.error('Failed to save export dir:', error);
		throw error;
	}
}

/**
 * Export selected chapters to RTF file
 */
export async function exportProjectToRTF(
	projectPath: string,
	exportDir: string,
	chapterIds: number[]
): Promise<string> {
	try {
		return await invoke<string>('export_project', {
			projectPath,
			exportDir,
			chapterIds,
		});
	} catch (error) {
		console.error('Failed to export project:', error);
		throw error;
	}
}

/**
 * Import chapters from text or markdown files
 */
export async function importChaptersFromFiles(
	projectPath: string,
	filePaths: string[],
	useFilenameAsTitle: boolean = true,
	chapterDelimiter?: string,
	extractTitleFromDelimiter: boolean = true
): Promise<Chapter[]> {
	try {
		return await invoke<Chapter[]>('import_chapters', {
			projectPath,
			filePaths,
			useFilenameAsTitle,
			chapterDelimiter: chapterDelimiter || null,
			extractTitleFromDelimiter,
		});
	} catch (error) {
		console.error('Failed to import chapters:', error);
		throw error;
	}
}

/**
 * Update the app's global font preference
 */
export async function updateFont(fontFamily: string): Promise<void> {
	try {
		await invoke('update_font', { fontFamily });
	} catch (error) {
		console.error('Failed to update font:', error);
		throw error;
	}
}

/**
 * Update a project's font preference
 */
export async function updateProjectFont(projectPath: string, fontFamily: string): Promise<void> {
	try {
		await invoke('update_project_font', {
			projectPath,
			fontFamily,
		});
	} catch (error) {
		console.error('Failed to update project font:', error);
		throw error;
	}
}

/**
 * Rename a chapter
 */
export async function renameChapter(projectPath: string, chapterId: number, newTitle: string): Promise<void> {
	try {
		await invoke('rename_chapter', {
			projectPath,
			chapterId,
			newTitle,
		});
	} catch (error) {
		console.error('Failed to rename chapter:', error);
		throw error;
	}
}

/**
 * Add a word to a custom dictionary
 */
export async function addToDictionary(
	word: string,
	scope: 'global' | 'project',
	projectPath: string | null
): Promise<void> {
	try {
		await invoke('add_to_dictionary', {
			word,
			scope,
			projectPath,
		});
	} catch (error) {
		console.error('Failed to add word to dictionary:', error);
		throw error;
	}
}

/**
 * Save project styles to project.json (merged in, other fields preserved)
 */
export async function saveStyles(projectPath: string, styles: ProjectStyles): Promise<void> {
	await invoke('save_project', { projectPath, projectData: { styles } });
}

/**
 * Save page settings to project.json (merged in, other fields preserved)
 */
export async function savePageSettings(projectPath: string, settings: PageSettings): Promise<void> {
	await invoke('save_project', { projectPath, projectData: { pageSettings: settings } });
}

/**
 * Export selected chapters to EPUB file
 */
export async function exportProjectToEPUB(
	projectPath: string,
	exportDir: string,
	chapterIds: number[]
): Promise<string> {
	try {
		return await invoke<string>('export_epub', {
			projectPath,
			exportDir,
			chapterIds,
		});
	} catch (error) {
		console.error('Failed to export EPUB:', error);
		throw error;
	}
}

/**
 * Delete a chapter: removes the chapter file and its entries in project.json
 */
export async function deleteChapter(projectPath: string, chapterId: number): Promise<void> {
	await invoke('delete_chapter', { projectPath, chapterId });
}

/**
 * Get all custom dictionary words (global + project)
 */
export async function getDictionaryWords(projectPath: string | null): Promise<string[]> {
	try {
		return await invoke<string[]>('get_dictionary_words', {
			projectPath,
		});
	} catch (error) {
		console.error('Failed to get dictionary words:', error);
		return [];
	}
}
