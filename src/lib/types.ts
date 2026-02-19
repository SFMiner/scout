export interface TipTapJSON {
	type?: string;
	[key: string]: any;
}

export interface Chapter {
	id: number;
	title: string;
	content: TipTapJSON | null;
}

export interface Project {
	title: string;
	author: string;
	chapterOrder: number[];
	exportDir?: string;
	fontFamily?: string;
}

export interface Config {
	lastProjectPath: string | null;
	fontFamily?: string;
}

export interface ProjectData extends Project {
	path: string;
}

export interface LoadProjectResponse {
	project: Project;
	chapters: Chapter[];
	path: string;
}

export interface CreateProjectResponse {
	project: Project;
	path: string;
}
