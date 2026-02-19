import { writable } from 'svelte/store';
import type { Chapter, Project } from './types';

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
export function markChapterUnsaved(chapterId: number) {
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
}

/**
 * Clear the current project (on close)
 */
export function clearProject() {
	project.set(null);
	chapters.set([]);
	hasStarted.set(false);
	clearUnsaved();
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
