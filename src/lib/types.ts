export interface TipTapJSON {
	type?: string;
	[key: string]: any;
}

export interface Chapter {
	id: number;
	title: string;
	content: TipTapJSON | null;
}

export interface StyleDefinition {
	fontSize?: number;       // pt
	fontFamily?: string;
	lineHeight?: number;
	bold?: boolean;
	italic?: boolean;
}

export type StyleKey = 'paragraph' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6' | 'blockquote';

export interface ProjectStyles {
	paragraph?: StyleDefinition;
	h2?: StyleDefinition;
	h3?: StyleDefinition;
	h4?: StyleDefinition;
	h5?: StyleDefinition;
	h6?: StyleDefinition;
	blockquote?: StyleDefinition;
}

export interface PageSettings {
	paperSize: 'letter' | 'a4' | 'trade' | 'digest' | 'pocket';
	margins: { top: number; bottom: number; left: number; right: number }; // inches
	pageNumbering: boolean;
	firstPageNumber: number;
	pageNumberPosition: 'bottom-outside' | 'top-outside' | 'bottom-center';
	textIndent: number;       // inches — first-line indent for paragraphs
	paragraphSpacing: number; // pt — space after each paragraph
	alignment: 'left' | 'justify';
}

export interface Project {
	title: string;
	author: string;
	chapterOrder: number[];
	exportDir?: string;
	fontFamily?: string;
	styles?: ProjectStyles;
	pageSettings?: PageSettings;
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
