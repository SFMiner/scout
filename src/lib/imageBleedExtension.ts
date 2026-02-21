import { Node, mergeAttributes } from '@tiptap/core';

declare module '@tiptap/core' {
	interface Commands<ReturnType> {
		imageBleed: {
			insertImageBleed: (attrs: { src: string; name: string; alt?: string }) => ReturnType;
			updateImageBleed: (attrs: { src: string; name: string }) => ReturnType;
		};
	}
}

export const ImageBleed = Node.create({
	name: 'imageBleed',
	group: 'block',
	atom: true,
	selectable: true,

	addAttributes() {
		return {
			src:  { default: '' },
			name: { default: '' },
			alt:  { default: '' },
		};
	},

	parseHTML() {
		return [{ tag: 'div[data-image-bleed]' }];
	},

	renderHTML({ node, HTMLAttributes }) {
		const { src, alt } = node.attrs;
		return [
			'div',
			mergeAttributes(HTMLAttributes, {
				'data-image-bleed': '',
				class: 'image-bleed',
			}),
			['img', { src: src || '', alt: alt || '' }],
		];
	},

	addCommands() {
		return {
			insertImageBleed:
				(attrs) =>
				({ commands }) => {
					return commands.insertContent({
						type: this.name,
						attrs,
					});
				},

			updateImageBleed:
				(attrs) =>
				({ editor, tr, dispatch }) => {
					// NodeSelection case (user clicked the image to select it)
					const sel = editor.state.selection as any;
					if (sel.node?.type?.name === 'imageBleed') {
						if (dispatch) {
							tr.setNodeMarkup(sel.from, undefined, { ...sel.node.attrs, ...attrs });
							dispatch(tr);
						}
						return true;
					}
					// Cursor adjacent to the node
					const { from } = editor.state.selection;
					let found = false;
					editor.state.doc.nodesBetween(from, from, (node, pos) => {
						if (node.type.name === 'imageBleed') {
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
