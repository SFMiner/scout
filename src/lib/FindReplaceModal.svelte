<script lang="ts">
	import { onMount } from 'svelte';
	import type { Editor } from '@tiptap/core';
	import type { Chapter } from './types';
	import { TextSelection } from '@tiptap/pm/state';

	export let editor: Editor;
	export let chapters: Chapter[] = [];
	export let selectedChapters: Set<number> = new Set();
	export let searchAllChapters: boolean = true;
	export let onClose: () => void;
	export let onNavigateChapter: (chapterId: number) => void = () => {};

	let findInput = '';
	let replaceInput = '';
	let matchCount = 0;
	let currentMatchIndex = 0;
	let caseSensitive = false;
	let wholeWord = false;

	// Reactive declarations to ensure UI updates
	$: console.log('matchCount changed to:', matchCount);
	$: console.log('replaceInput changed to:', replaceInput);
	$: isReplaceDisabled = matchCount === 0 || !replaceInput;
	$: console.log('isReplaceDisabled:', isReplaceDisabled);
	$: console.log('Props - chapters:', chapters?.length, 'editor:', !!editor);

	interface Match {
		chapterId: number;
		chapterTitle: string;
		from: number;
		to: number;
		text: string;
	}

	let matches: Match[] = [];

	onMount(() => {
		const findField = document.querySelector('input[data-find-input]') as HTMLInputElement;
		if (findField) findField.focus();
	});

	function isWordBoundary(text: string, index: number, length: number): boolean {
		const charBefore = index > 0 ? text[index - 1] : ' ';
		const charAfter = index + length < text.length ? text[index + length] : ' ';
		const isWordChar = /\w/;

		return !isWordChar.test(charBefore) && !isWordChar.test(charAfter);
	}

	function performFind() {
		console.log('=== performFind called ===');
		console.log('findInput:', findInput);

		if (!findInput) {
			console.log('Early return: no input');
			matches = [];
			matchCount = 0;
			currentMatchIndex = 0;
			return;
		}

		matches = [];
		const searchText = caseSensitive ? findInput : findInput.toLowerCase();
		const fullText = editor.getText();

		console.log('Searching for:', findInput);
		console.log('Editor text length:', fullText.length);
		console.log('Editor text:', fullText.substring(0, 100));

		// Search in current editor (always search the active chapter)
		let foundAny = false;
		let textNodesChecked = 0;
		editor.state.doc.descendants((node, pos) => {
			if (node.isText) {
				textNodesChecked++;
				const text = node.text || '';
				const compareText = caseSensitive ? text : text.toLowerCase();
				let startIndex = 0;
				let index: number;

				if (textNodesChecked <= 5) {
					console.log(`Text node ${textNodesChecked} at pos ${pos}: "${text.substring(0, 50)}..."`);
					console.log(`  compareText: "${compareText.substring(0, 50)}..."`);
					console.log(`  searching for "${searchText}" in compareText`);
					const testIndex = compareText.indexOf(searchText);
					console.log(`  indexOf result: ${testIndex}`);
				}

				while ((index = compareText.indexOf(searchText, startIndex)) !== -1) {
					foundAny = true;
					console.log(`FOUND at index ${index}`);
					if (wholeWord && !isWordBoundary(compareText, index, searchText.length)) {
						startIndex = index + 1;
						continue;
					}

					const matchFrom = pos + index;
					const matchTo = pos + index + findInput.length;

					// Get current chapter info - use first chapter with content as fallback
					const currentChapter = (chapters && chapters.length > 0) ? (chapters.find(ch => ch?.content) || chapters[0]) : null;
					if (currentChapter) {
						matches.push({
							chapterId: currentChapter.id,
							chapterTitle: currentChapter.title,
							from: matchFrom,
							to: matchTo,
							text: text.substring(index, index + findInput.length)
						});
						console.log(`Found match at pos ${matchFrom}-${matchTo}: "${text.substring(index, index + findInput.length)}"`);
					}

					startIndex = index + findInput.length;
				}
			}
		});

		console.log('Total text nodes checked:', textNodesChecked);
		console.log('Total matches found:', matches.length);
		matchCount = matches.length;
		currentMatchIndex = 0;

		// Force reactivity
		matches = matches;

		if (matches.length > 0) {
			scrollToMatch();
		}
	}

	function scrollToMatch() {
		if (!matches[currentMatchIndex]) return;

		const match = matches[currentMatchIndex];

		// Select the match in the editor
		const selection = TextSelection.create(editor.state.doc, match.from, match.to);
		editor.view.dispatch(editor.state.tr.setSelection(selection));
	}

	function nextMatch() {
		if (matches.length > 0) {
			currentMatchIndex = (currentMatchIndex + 1) % matches.length;
			scrollToMatch();
		}
	}

	function prevMatch() {
		if (matches.length > 0) {
			currentMatchIndex = (currentMatchIndex - 1 + matches.length) % matches.length;
			scrollToMatch();
		}
	}

	function replaceOne() {
		if (matches.length === 0 || !replaceInput) return;

		const match = matches[currentMatchIndex];
		const { state } = editor.view;

		const tr = state.tr.replaceWith(match.from, match.to, state.schema.text(replaceInput));
		editor.view.dispatch(tr);

		performFind();
	}

	function replaceAll() {
		if (matches.length === 0 || !replaceInput) return;

		const { state } = editor.view;
		let tr = state.tr;

		// Process in reverse order to maintain positions
		const sortedMatches = [...matches].reverse();
		for (const match of sortedMatches) {
			tr = tr.replaceWith(match.from, match.to, state.schema.text(replaceInput));
		}

		editor.view.dispatch(tr);
		performFind();
	}

	function handleFindKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.shiftKey ? prevMatch() : nextMatch();
		} else if (e.key === 'Escape') {
			onClose();
		}
	}

	function handleReplaceKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			replaceOne();
		} else if (e.key === 'Escape') {
			onClose();
		}
	}
</script>

<div class="find-replace-panel">
	<div class="find-replace-inputs">
		<div class="input-group">
			<input
				type="text"
				placeholder="Find..."
				bind:value={findInput}
				onkeydown={handleFindKeydown}
				oninput={performFind}
				data-find-input
				class="find-input"
			/>
			<span class="match-count">{matchCount > 0 ? `${currentMatchIndex + 1}/${matchCount}` : ''}</span>
		</div>

		<div class="input-group">
			<input
				type="text"
				placeholder="Replace..."
				bind:value={replaceInput}
				onkeydown={handleReplaceKeydown}
				class="replace-input"
			/>
		</div>
	</div>

	<div class="find-replace-controls">
		<div class="button-group">
			<button
				class="icon-btn"
				title="Previous match (Shift+Enter)"
				onclick={prevMatch}
				disabled={matchCount === 0}
			>
				↑
			</button>
			<button
				class="icon-btn"
				title="Next match (Enter)"
				onclick={nextMatch}
				disabled={matchCount === 0}
			>
				↓
			</button>
		</div>

		<div class="button-group">
			<button
				class="replace-btn"
				onclick={replaceOne}
				disabled={isReplaceDisabled}
			>
				Replace
			</button>
			<button
				class="replace-all-btn"
				onclick={replaceAll}
				disabled={isReplaceDisabled}
			>
				Replace All
			</button>
		</div>

		<div class="options">
			<label class="checkbox-small" title="Case sensitive">
				<input type="checkbox" bind:checked={caseSensitive} onchange={performFind} />
				<span>Aa</span>
			</label>
			<label class="checkbox-small" title="Whole word">
				<input type="checkbox" bind:checked={wholeWord} onchange={performFind} />
				<span>ww</span>
			</label>
		</div>

		<button class="close-btn" onclick={onClose}>×</button>
	</div>

	{#if matches.length > 0 && currentMatchIndex < matches.length}
		<div class="match-info">
			<span class="chapter-name">{matches[currentMatchIndex].chapterTitle}</span>
		</div>
	{/if}
</div>

<style>
	.find-replace-panel {
		position: fixed;
		top: 3rem;
		right: 1rem;
		background: white;
		border: 1px solid #ddd;
		border-radius: 6px;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
		z-index: 1001;
		padding: 0.75rem;
		max-width: 400px;
		min-width: 350px;
	}

	.find-replace-inputs {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}

	.input-group {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}

	.find-input,
	.replace-input {
		flex: 1;
		padding: 0.4rem 0.6rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 0.85rem;
		outline: none;
	}

	.find-input:focus,
	.replace-input:focus {
		border-color: #cba6f7;
		box-shadow: 0 0 0 2px rgba(203, 166, 247, 0.1);
	}

	.match-count {
		font-size: 0.75rem;
		color: #999;
		min-width: 2rem;
		text-align: right;
	}

	.find-replace-controls {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		flex-wrap: wrap;
		margin-bottom: 0.5rem;
	}

	.button-group {
		display: flex;
		gap: 0.25rem;
	}

	.icon-btn {
		padding: 0.3rem 0.5rem;
		background: #f0f0f0;
		border: 1px solid #ddd;
		border-radius: 3px;
		cursor: pointer;
		font-size: 0.85rem;
		color: #0f0f0f;
		transition: all 0.15s;
		min-width: 2rem;
	}

	.icon-btn:hover:not(:disabled) {
		background: #e8e8e8;
	}

	.icon-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.replace-btn,
	.replace-all-btn {
		padding: 0.3rem 0.6rem;
		background: #cba6f7;
		color: white;
		border: none;
		border-radius: 3px;
		cursor: pointer;
		font-size: 0.8rem;
		font-weight: 500;
		transition: all 0.15s;
	}

	.replace-btn:hover:not(:disabled),
	.replace-all-btn:hover:not(:disabled) {
		background: #b896e7;
	}

	.replace-btn:disabled,
	.replace-all-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.options {
		display: flex;
		gap: 0.5rem;
	}

	.checkbox-small {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		cursor: pointer;
		font-size: 0.8rem;
		color: #0f0f0f;
	}

	.checkbox-small input {
		cursor: pointer;
		accent-color: #cba6f7;
	}

	.checkbox-small span {
		font-weight: 600;
	}

	.close-btn {
		background: none;
		border: none;
		font-size: 1.2rem;
		cursor: pointer;
		color: #999;
		padding: 0;
		margin-left: auto;
		width: 1.5rem;
		height: 1.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.close-btn:hover {
		color: #0f0f0f;
	}

	.match-info {
		border-top: 1px solid #e0e0e0;
		padding-top: 0.5rem;
		font-size: 0.8rem;
		color: #666;
	}

	.chapter-name {
		font-weight: 500;
		color: #333;
	}

	/* Dark mode */
	@media (prefers-color-scheme: dark) {
		.find-replace-panel {
			background: #1e1e2e;
			border-color: #313244;
			color: #cdd6f4;
		}

		.find-input,
		.replace-input {
			background-color: #313244;
			border-color: #45475a;
			color: #cdd6f4;
		}

		.find-input:focus,
		.replace-input:focus {
			border-color: #cba6f7;
			box-shadow: 0 0 0 2px rgba(203, 166, 247, 0.2);
		}

		.match-count {
			color: #a6adc8;
		}

		.icon-btn {
			background: #313244;
			border-color: #45475a;
			color: #cdd6f4;
		}

		.icon-btn:hover:not(:disabled) {
			background: #45475a;
		}

		.checkbox-small {
			color: #cdd6f4;
		}

		.close-btn {
			color: #a6adc8;
		}

		.close-btn:hover {
			color: #cdd6f4;
		}

		.match-info {
			border-top-color: #313244;
			color: #a6adc8;
		}

		.chapter-name {
			color: #cdd6f4;
		}
	}
</style>
