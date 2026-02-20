import { Node, mergeAttributes } from '@tiptap/core';

declare module '@tiptap/core' {
	interface Commands<ReturnType> {
		colorBleed: {
			insertColorBleed: (attrs: { backgroundColor: string; textColor: string }) => ReturnType;
			updateBleedColor: (attrs: { backgroundColor: string; textColor: string }) => ReturnType;
		};
	}
}

export const ColorBleed = Node.create({
	name: 'colorBleed',
	group: 'block',
	content: 'block+',
	defining: true,
	isolating: true,

	addAttributes() {
		return {
			backgroundColor: { default: '#000000' },
			textColor: { default: '#ffffff' },
		};
	},

	parseHTML() {
		return [{ tag: 'div[data-color-bleed]' }];
	},

	renderHTML({ node, HTMLAttributes }) {
		const { backgroundColor, textColor } = node.attrs;
		return [
			'div',
			mergeAttributes(HTMLAttributes, {
				'data-color-bleed': '',
				class: 'color-bleed',
				style: `background-color:${backgroundColor};color:${textColor};`,
			}),
			0,
		];
	},

	addCommands() {
		return {
			insertColorBleed:
				(attrs: { backgroundColor: string; textColor: string }) =>
				({ commands }) => {
					return commands.insertContent({
						type: this.name,
						attrs,
						content: [{ type: 'paragraph' }],
					});
				},

			updateBleedColor:
				(attrs: { backgroundColor: string; textColor: string }) =>
				({ editor, tr, dispatch }) => {
					const { from } = editor.state.selection;
					let found = false;
					editor.state.doc.nodesBetween(from, from, (node, pos) => {
						if (node.type.name === 'colorBleed') {
							if (dispatch) {
								tr.setNodeMarkup(pos, undefined, { ...node.attrs, ...attrs });
								dispatch(tr);
							}
							found = true;
							return false;
						}
					});
					return found;
				},
		};
	},
});

/** Returns white (#ffffff) for dark backgrounds, black (#000000) for light. */
export function contrastColor(hex: string): string {
	const r = parseInt(hex.slice(1, 3), 16);
	const g = parseInt(hex.slice(3, 5), 16);
	const b = parseInt(hex.slice(5, 7), 16);
	// Perceived luminance (ITU-R BT.601)
	const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
	return luminance > 0.5 ? '#000000' : '#ffffff';
}
