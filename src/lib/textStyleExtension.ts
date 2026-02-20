import { TextStyle } from '@tiptap/extension-text-style'

/**
 * Extends TipTap's TextStyle mark to support inline fontSize (pt) and fontFamily.
 * These inline marks are what "Update Style" captures to build style definitions.
 */
export const CustomTextStyle = TextStyle.extend({
	addAttributes() {
		return {
			...this.parent?.(),
			fontSize: {
				default: null,
				parseHTML: (element: HTMLElement) => {
					const size = element.style.fontSize
					if (!size) return null
					return parseFloat(size) || null
				},
				renderHTML: (attributes: Record<string, any>) => {
					if (!attributes.fontSize) return {}
					return { style: `font-size: ${attributes.fontSize}pt` }
				},
			},
			fontFamily: {
				default: null,
				parseHTML: (element: HTMLElement) => element.style.fontFamily || null,
				renderHTML: (attributes: Record<string, any>) => {
					if (!attributes.fontFamily) return {}
					return { style: `font-family: ${attributes.fontFamily}` }
				},
			},
		}
	},

	addCommands() {
		return {
			...this.parent?.(),
			setFontSize:
				(size: number | null) =>
				({ chain }: any) => {
					if (size === null) {
						return chain()
							.setMark('textStyle', { fontSize: null })
							.removeEmptyTextStyle()
							.run()
					}
					return chain().setMark('textStyle', { fontSize: size }).run()
				},
			setFontFamily:
				(family: string | null) =>
				({ chain }: any) => {
					if (family === null) {
						return chain()
							.setMark('textStyle', { fontFamily: null })
							.removeEmptyTextStyle()
							.run()
					}
					return chain().setMark('textStyle', { fontFamily: family }).run()
				},
		} as any
	},
})
