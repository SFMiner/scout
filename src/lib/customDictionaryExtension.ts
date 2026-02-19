import { Extension } from '@tiptap/core'
import { Plugin, PluginKey } from 'prosemirror-state'
import { Decoration, DecorationSet } from 'prosemirror-view'
import type { Node } from 'prosemirror-model'

export const DictionaryPluginKey = new PluginKey<DecorationSet>('customDictionary')

// Module-level word set — updated externally via setDictionaryWords / addDictionaryWord
const customWords = new Set<string>()

export function setDictionaryWords(words: string[]): void {
	customWords.clear()
	for (const word of words) {
		customWords.add(word.toLowerCase())
	}
}

export function addDictionaryWord(word: string): void {
	customWords.add(word.toLowerCase())
}

function buildDecorations(doc: Node): DecorationSet {
	if (customWords.size === 0) return DecorationSet.empty

	const decorations: Decoration[] = []

	doc.descendants((node, pos) => {
		if (!node.isText || !node.text) return

		const text = node.text
		const wordRegex = /\b\w+\b/g
		let match: RegExpExecArray | null

		while ((match = wordRegex.exec(text)) !== null) {
			if (customWords.has(match[0].toLowerCase())) {
				decorations.push(
					Decoration.inline(
						pos + match.index,
						pos + match.index + match[0].length,
						{ spellcheck: 'false' }
					)
				)
			}
		}
	})

	return DecorationSet.create(doc, decorations)
}

export const CustomDictionaryExtension = Extension.create({
	name: 'customDictionary',

	addProseMirrorPlugins() {
		return [
			new Plugin({
				key: DictionaryPluginKey,

				state: {
					init(_, { doc }) {
						return buildDecorations(doc)
					},
					apply(tr, decorationSet, _, newState) {
						if (!tr.docChanged && !tr.getMeta(DictionaryPluginKey)) {
							return decorationSet
						}
						return buildDecorations(newState.doc)
					},
				},

				props: {
					decorations(state) {
						return DictionaryPluginKey.getState(state)
					},
				},
			}),
		]
	},
})
