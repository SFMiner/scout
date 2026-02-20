import { writable } from 'svelte/store';
import type { Chapter, Project, ProjectStyles, PageSettings, StyleKey } from './types';

export const DEFAULT_STYLES: ProjectStyles = {
	paragraph:  { fontSize: 12, lineHeight: 1.7 },
	h2:         { fontSize: 22, bold: true,  lineHeight: 1.3 },
	h3:         { fontSize: 18, bold: true,  lineHeight: 1.3 },
	h4:         { fontSize: 15, bold: true,  lineHeight: 1.3 },
	h5:         { fontSize: 13, bold: true,  lineHeight: 1.3 },
	h6:         { fontSize: 12, bold: true,  lineHeight: 1.3 },
	blockquote: { fontSize: 12, italic: true, lineHeight: 1.8 },
};

/** Deep-merge project styles over defaults, per style key. */
export function mergeWithDefaults(overrides?: ProjectStyles): ProjectStyles {
	const result: ProjectStyles = {};
	for (const key of Object.keys(DEFAULT_STYLES) as StyleKey[]) {
		result[key] = { ...DEFAULT_STYLES[key], ...(overrides?.[key] ?? {}) };
	}
	return result;
}

export const projectStyles = writable<ProjectStyles>(mergeWithDefaults());

export const DEFAULT_PAGE_SETTINGS: PageSettings = {
	paperSize: 'letter',
	margins: { top: 1, bottom: 1, left: 1.25, right: 1.25 },
	pageNumbering: true,
	firstPageNumber: 1,
	pageNumberPosition: 'bottom-center',
	textIndent: 0,
	paragraphSpacing: 0,
	alignment: 'left',
};

export const pageSettings = writable<PageSettings>({ ...DEFAULT_PAGE_SETTINGS });

/**
 * The current project (null if no project is open)
 */
export const project = writable<Project & { path: string } | null>(null);

/**
 * All chapters in the current project
 */
export const chapters = writable<Chapter[]>([]);

/**
 * Whether the app is currently loading data
 */
export const loading = writable(false);

/**
 * Error message to display to user (null if no error)
 */
export const error = writable<string | null>(null);

/**
 * Whether the user has completed startup (selected/created a project)
 */
export const hasStarted = writable(false);

/**
 * Set of chapter IDs that have unsaved changes (empty set = all saved)
 */
export const unsavedChapters = writable<Set<number>>(new Set());

/**
 * Helper to add a chapter to the unsaved set
 */
let _unsavedSnapshot = new Set<number>();
unsavedChapters.subscribe((s) => { _unsavedSnapshot = s; });

export function markChapterUnsaved(chapterId: number) {
	if (_unsavedSnapshot.has(chapterId)) return; // already unsaved, skip re-render
	unsavedChapters.update((set) => {
		set.add(chapterId);
		return set;
	});
}

/**
 * Helper to remove a chapter from the unsaved set
 */
export function markChapterSaved(chapterId: number) {
	unsavedChapters.update((set) => {
		set.delete(chapterId);
		return set;
	});
}

/**
 * Clear all unsaved markers
 */
export function clearUnsaved() {
	unsavedChapters.set(new Set());
}

/**
 * Set the current project and chapters
 */
export function setProject(
	newProject: Project & { path: string },
	newChapters: Chapter[]
) {
	project.set(newProject);
	chapters.set(newChapters);
	hasStarted.set(true);
	clearUnsaved();
	projectStyles.set(mergeWithDefaults(newProject.styles));
	pageSettings.set({ ...DEFAULT_PAGE_SETTINGS, ...newProject.pageSettings });
}

/**
 * Clear the current project (on close)
 */
export function clearProject() {
	project.set(null);
	chapters.set([]);
	hasStarted.set(false);
	clearUnsaved();
	projectStyles.set(mergeWithDefaults());
	pageSettings.set({ ...DEFAULT_PAGE_SETTINGS });
}

/**
 * Add imported chapters to the project
 */
export function addChapters(newChapters: Chapter[]) {
	chapters.update((existing) => [...existing, ...newChapters]);
	project.update((proj) => {
		if (proj) {
			proj.chapterOrder = [...proj.chapterOrder, ...newChapters.map(ch => ch.id)];
		}
		return proj;
	});
}

/**
 * The current app font family (global setting)
 */
export const appFont = writable<string>('Inter, Avenir, Helvetica, Arial, sans-serif');

/**
 * Helper to set the app font
 */
export function setAppFont(fontFamily: string) {
	appFont.set(fontFamily);
}
