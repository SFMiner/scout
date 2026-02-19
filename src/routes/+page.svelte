<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Editor } from '@tiptap/core';
	import StarterKit from '@tiptap/starter-kit';
	import Heading from '@tiptap/extension-heading';
	import Image from '@tiptap/extension-image';
	import Blockquote from '@tiptap/extension-blockquote';
	import Placeholder from '@tiptap/extension-placeholder';
	import StartupModal from '$lib/StartupModal.svelte';
	import ExportModal from '$lib/ExportModal.svelte';
	import ImportModal from '$lib/ImportModal.svelte';
	import FontModal from '$lib/FontModal.svelte';
	import FindReplaceModal from '$lib/FindReplaceModal.svelte';
	import {
		project,
		chapters,
		loading,
		hasStarted,
		unsavedChapters,
		markChapterUnsaved,
		markChapterSaved,
		clearProject,
		addChapters,
		appFont,
	} from '$lib/stores';
	import { readConfig, saveChapter, saveProjectMetadata, renameChapter, addToDictionary, getDictionaryWords, deleteChapter } from '$lib/fileIO';
	import { CustomDictionaryExtension, DictionaryPluginKey, setDictionaryWords, addDictionaryWord } from '$lib/customDictionaryExtension';
	import type { Chapter } from '$lib/types';

	let editorElement: HTMLElement;
	let editor: Editor;
	let activeChapterId = 1;
	let recentProjectPath: string | null = null;
	let selectedChapters = new Set<number>();
	let showExportModal = false;
	let showImportModal = false;
	let showFontModal = false;
	let showFindReplaceModal = false;
	let editingChapterId: number | null = null;
	let editingTitle = '';
	let selectMode = false;

	// Dictionary context menu state
	let showDictContextMenu = false;
	let contextMenuX = 0;
	let contextMenuY = 0;
	let selectedWord = '';

	// Drag-to-reorder state
	let draggedChapterId: number | null = null;
	let dragOverChapterId: number | null = null;
	let dragOverPosition: 'before' | 'after' | null = null;

	// Chapter context menu / delete state
	let showChapterContextMenu = false;
	let chapterContextMenuX = 0;
	let chapterContextMenuY = 0;
	let contextMenuChapterId: number | null = null;
	let showDeleteConfirm = false;
	let chapterToDelete: number | null = null;

	// Auto-save debouncing
	let autoSaveTimeout: number | null = null;
	const AUTO_SAVE_DELAY = 1000; // 1 second

	function handleKeyDown(e: KeyboardEvent) {
		// Ctrl+H for find/replace
		if ((e.ctrlKey || e.metaKey) && e.key === 'h') {
			e.preventDefault();
			showFindReplaceModal = true;
		}
	}

	function getWordAtCursor(event: MouseEvent): string {
		// Get the position of the click
		const target = event.target as HTMLElement;

		// Get the text content and position
		const selection = window.getSelection();
		if (!selection) return '';

		// If text is already selected, use it
		if (selection.toString()) {
			return selection.toString().trim();
		}

		// Try to get the word at cursor by finding word boundaries
		const range = document.caretRangeFromPoint(event.clientX, event.clientY);
		if (!range || !range.startContainer.textContent) return '';

		const text = range.startContainer.textContent;
		const offset = range.startOffset;

		// Find word boundaries (simple approach: split on spaces/punctuation)
		const before = text.substring(0, offset).match(/\w*$/)?.[0] || '';
		const after = text.substring(offset).match(/^\w*/)?.[0] || '';
		const word = before + after;

		return word.trim();
	}

	onMount(async () => {
		window.addEventListener('keydown', handleKeyDown);
		// Check for recent project on app start
		try {
			const config = await readConfig();
			if (config.lastProjectPath) {
				recentProjectPath = config.lastProjectPath;
			}
			// Load app-level font preference
			if (config.fontFamily) {
				appFont.set(config.fontFamily);
			}
		} catch (err) {
			console.error('Failed to read config:', err);
		}

		// Initialize editor (will load content after project is selected)
		editor = new Editor({
			element: editorElement,
			extensions: [
				StarterKit.configure({
					heading: false,
					blockquote: false,
				}),
				Heading.configure({
					levels: [2, 3, 4, 5, 6],
				}),
				Blockquote,
				Image,
				Placeholder.configure({
					placeholder: 'Start writing...',
				}),
				CustomDictionaryExtension,
			],
			content: { type: 'doc', content: [] },
			onTransaction: () => {
				editor = editor;
				// Mark current chapter as unsaved
				if ($hasStarted) {
					markChapterUnsaved(activeChapterId);
					scheduleAutoSave();
				}
			},
		});

		// Right-click handler for dictionary context menu
		editor.view.dom.addEventListener('contextmenu', (e: MouseEvent) => {
			if (!$hasStarted) return;

			e.preventDefault();

			selectedWord = getWordAtCursor(e);
			if (!selectedWord || selectedWord.length < 2) {
				return; // Ignore clicks on punctuation or very short words
			}

			contextMenuX = e.clientX;
			contextMenuY = e.clientY;
			showDictContextMenu = true;

			// Close menu after 3 seconds if user doesn't click
			setTimeout(() => {
				if (showDictContextMenu) {
					showDictContextMenu = false;
				}
			}, 3000);
		});

		// Close context menus on click elsewhere
		document.addEventListener('click', () => {
			showDictContextMenu = false;
			showChapterContextMenu = false;
		});
	});

	onDestroy(() => {
		// Flush any pending saves before closing
		if (autoSaveTimeout) {
			clearTimeout(autoSaveTimeout);
			saveCurrentChapter();
		}
		if (editor) editor.destroy();
		window.removeEventListener('keydown', handleKeyDown);
	});

	// Subscribe to chapters store to ensure first chapter loads on project open
	chapters.subscribe((chaps: Chapter[]) => {
		if (chaps.length > 0 && !chaps.find((ch: Chapter) => ch.id === activeChapterId)) {
			activeChapterId = chaps[0].id;
		}
		// Load first chapter content when chapters first load
		if (editor && chaps.length > 0 && activeChapterId === chaps[0].id) {
			const chapter = chaps.find((ch: Chapter) => ch.id === activeChapterId);
			if (chapter) {
				const content = chapter.content || { type: 'doc', content: [] };
				editor.commands.setContent(content);
			}
		}
	});

	// Subscribe to project store to load project-level font and dictionary words
	project.subscribe(async (proj) => {
		if (proj?.fontFamily) {
			appFont.set(proj.fontFamily);
		}
		if (proj) {
			try {
				const words = await getDictionaryWords(proj.path);
				setDictionaryWords(words);
				// Trigger re-decoration if editor is already initialized
				if (editor) {
					editor.view.dispatch(editor.state.tr.setMeta(DictionaryPluginKey, true));
				}
			} catch (err) {
				console.error('Failed to load dictionary words:', err);
			}
		}
	});

	function scheduleAutoSave() {
		if (autoSaveTimeout) clearTimeout(autoSaveTimeout);

		autoSaveTimeout = window.setTimeout(() => {
			saveCurrentChapter();
			autoSaveTimeout = null;
		}, AUTO_SAVE_DELAY);
	}

	async function saveCurrentChapter() {
		if (!editor || !$hasStarted || !$project) return;

		const chapter = $chapters.find((ch: Chapter) => ch.id === activeChapterId);
		if (!chapter) return;

		try {
			const content = editor.getJSON();
			chapter.content = content;

			// Update store
			$chapters = $chapters;

			// Save to file
			await saveChapter($project.path, activeChapterId, content);
			markChapterSaved(activeChapterId);
		} catch (err) {
			console.error('Failed to auto-save chapter:', err);
		}
	}

	async function addChapter() {
		if (!$hasStarted || !$project) return;

		// Save current chapter first
		await saveCurrentChapter();

		const id = $chapters.length > 0 ? Math.max(...$chapters.map((ch: Chapter) => ch.id)) + 1 : 1;
		const newChapter: Chapter = { id, title: `Chapter ${id}`, content: null };

		$chapters = [...$chapters, newChapter];
		$project.chapterOrder = [...$project.chapterOrder, id];

		// Save updated project metadata
		saveProjectMetadata($project.path, {
			title: $project.title,
			author: $project.author,
			chapterOrder: $project.chapterOrder,
		}).catch((err) => console.error('Failed to save project metadata:', err));

		// Switch to new chapter (editor will load via store subscription)
		activeChapterId = id;
	}

	async function selectChapter(id: number) {
		if (!$hasStarted || !$project) return;

		// Don't switch if already on this chapter
		if (activeChapterId === id) return;

		// Save current chapter first
		await saveCurrentChapter();

		// Switch to new chapter
		activeChapterId = id;
		const chapter = $chapters.find((ch: Chapter) => ch.id === id);
		if (editor && chapter) {
			const content = chapter.content || { type: 'doc', content: [] };
			editor.commands.setContent(content);
		}
	}

	async function closeProject() {
		// Flush any pending saves
		if (autoSaveTimeout) {
			clearTimeout(autoSaveTimeout);
			await saveCurrentChapter();
		}
		// Clear project state and show startup modal
		clearProject();
	}

	function toggleSelectMode() {
		selectMode = !selectMode;
		if (!selectMode) {
			selectedChapters.clear();
			selectedChapters = selectedChapters;
		}
	}

	function handleChapterClick(chapterId: number) {
		if (selectMode) {
			// In select mode, clicking toggles selection
			if (selectedChapters.has(chapterId)) {
				selectedChapters.delete(chapterId);
			} else {
				selectedChapters.add(chapterId);
			}
			selectedChapters = selectedChapters;
		} else {
			// Normal mode: navigate to chapter
			selectChapter(chapterId);
		}
	}

	function handleDragStart(e: DragEvent, chapterId: number) {
		draggedChapterId = chapterId;
		if (e.dataTransfer) {
			e.dataTransfer.effectAllowed = 'move';
			e.dataTransfer.setData('text/plain', String(chapterId));
		}
	}

	function handleDragOver(e: DragEvent, chapterId: number) {
		e.preventDefault(); // must always be called — enables drop
		if (draggedChapterId === null || draggedChapterId === chapterId) return;
		if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';

		const target = e.currentTarget as HTMLElement;
		const rect = target.getBoundingClientRect();
		dragOverChapterId = chapterId;
		dragOverPosition = e.clientY < rect.top + rect.height / 2 ? 'before' : 'after';
	}

	function handleDragLeave() {
		dragOverChapterId = null;
		dragOverPosition = null;
	}

	async function handleDrop(e: DragEvent, targetChapterId: number) {
		e.preventDefault();
		if (draggedChapterId === null || draggedChapterId === targetChapterId) {
			handleDragEnd();
			return;
		}

		const fromIndex = $chapters.findIndex((ch: Chapter) => ch.id === draggedChapterId);
		const toIndex = $chapters.findIndex((ch: Chapter) => ch.id === targetChapterId);
		if (fromIndex === -1 || toIndex === -1) {
			handleDragEnd();
			return;
		}

		const reordered = [...$chapters];
		const [moved] = reordered.splice(fromIndex, 1);
		let insertAt = dragOverPosition === 'before' ? toIndex : toIndex + 1;
		if (fromIndex < toIndex) insertAt--;
		reordered.splice(insertAt, 0, moved);

		$chapters = reordered;
		$project!.chapterOrder = reordered.map((ch: Chapter) => ch.id);

		saveProjectMetadata($project!.path, {
			title: $project!.title,
			author: $project!.author,
			chapterOrder: $project!.chapterOrder,
		}).catch((err) => console.error('Failed to save chapter order:', err));

		handleDragEnd();
	}

	function handleDragEnd() {
		draggedChapterId = null;
		dragOverChapterId = null;
		dragOverPosition = null;
	}

	function openExportModal() {
		showExportModal = true;
	}

	function closeExportModal() {
		showExportModal = false;
	}

	function handleExportSuccess() {
		selectedChapters.clear();
		selectedChapters = selectedChapters;
		selectMode = false;
	}

	function openImportModal() {
		showImportModal = true;
	}

	function closeImportModal() {
		showImportModal = false;
	}

	function handleImportSuccess(newChapters: Chapter[]) {
		addChapters(newChapters);
		showImportModal = false;
	}

	function startEditingChapter(chapterId: number, currentTitle: string) {
		editingChapterId = chapterId;
		editingTitle = currentTitle;
		// Focus the input on next tick
		setTimeout(() => {
			const input = document.querySelector(`input[data-chapter="${chapterId}"]`) as HTMLInputElement;
			if (input) input.focus();
		}, 0);
	}

	function makeUniqueTitle(title: string, excludeId: number): string {
		const existing = new Set(
			$chapters
				.filter((ch: Chapter) => ch.id !== excludeId)
				.map((ch: Chapter) => ch.title.toLowerCase())
		);
		if (!existing.has(title.toLowerCase())) return title;
		let n = 1;
		while (true) {
			const candidate = `${title} (${n})`;
			if (!existing.has(candidate.toLowerCase())) return candidate;
			n++;
		}
	}

	async function saveChapterTitle() {
		if (!$hasStarted || !$project || editingChapterId === null) return;

		const trimmedTitle = editingTitle.trim();
		if (!trimmedTitle) {
			editingChapterId = null;
			return;
		}

		const uniqueTitle = makeUniqueTitle(trimmedTitle, editingChapterId);

		try {
			await renameChapter($project.path, editingChapterId, uniqueTitle);
			$chapters = $chapters.map((ch) =>
				ch.id === editingChapterId ? { ...ch, title: uniqueTitle } : ch
			);
			editingChapterId = null;
		} catch (err) {
			console.error('Failed to rename chapter:', err);
			editingChapterId = null;
		}
	}

	function handleChapterTitleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			saveChapterTitle();
		} else if (e.key === 'Escape') {
			editingChapterId = null;
		}
	}

	async function handleAddGlobalDict() {
		if (!selectedWord || !$project) return;
		try {
			await addToDictionary(selectedWord, 'global', null);
			addDictionaryWord(selectedWord);
			editor.view.dispatch(editor.state.tr.setMeta(DictionaryPluginKey, true));
			showDictContextMenu = false;
		} catch (err) {
			console.error('Failed to add word to dictionary:', err);
		}
	}

	function getCurrentStyle(): string {
		if (!editor) return 'paragraph';
		for (let level = 2; level <= 6; level++) {
			if (editor.isActive('heading', { level })) return `h${level}`;
		}
		if (editor.isActive('blockquote')) return 'blockquote';
		return 'paragraph';
	}

	function applyStyle(value: string) {
		if (!editor) return;
		const inBlockquote = editor.isActive('blockquote');
		if (value === 'paragraph') {
			if (inBlockquote) {
				editor.chain().focus().toggleBlockquote().run();
			} else {
				editor.chain().focus().setParagraph().run();
			}
		} else if (value === 'blockquote') {
			if (!inBlockquote) {
				editor.chain().focus().toggleBlockquote().run();
			}
		} else {
			const level = parseInt(value[1]) as 2 | 3 | 4 | 5 | 6;
			if (inBlockquote) {
				editor.chain().focus().toggleBlockquote().setHeading({ level }).run();
			} else {
				editor.chain().focus().setHeading({ level }).run();
			}
		}
	}

	function handleChapterContextMenu(e: MouseEvent, chapterId: number) {
		e.preventDefault();
		contextMenuChapterId = chapterId;
		chapterContextMenuX = e.clientX;
		chapterContextMenuY = e.clientY;
		showChapterContextMenu = true;
	}

	async function confirmDeleteChapter() {
		if (chapterToDelete === null || !$project) return;

		// If deleting the active chapter, switch to an adjacent one first
		if (chapterToDelete === activeChapterId) {
			const idx = $chapters.findIndex((ch: Chapter) => ch.id === chapterToDelete);
			const remaining = $chapters.filter((ch: Chapter) => ch.id !== chapterToDelete);
			if (remaining.length > 0) {
				const newIdx = Math.min(idx, remaining.length - 1);
				activeChapterId = remaining[newIdx].id;
				const chapter = remaining[newIdx];
				if (editor && chapter) {
					editor.commands.setContent(chapter.content || { type: 'doc', content: [] });
				}
			}
		}

		try {
			await deleteChapter($project.path, chapterToDelete);
			const idToRemove = chapterToDelete;
			$chapters = $chapters.filter((ch: Chapter) => ch.id !== idToRemove);
			$project.chapterOrder = $project.chapterOrder.filter((id: number) => id !== idToRemove);
		} catch (err) {
			console.error('Failed to delete chapter:', err);
		} finally {
			chapterToDelete = null;
			showDeleteConfirm = false;
		}
	}

	async function handleAddProjectDict() {
		if (!selectedWord || !$project) return;
		try {
			await addToDictionary(selectedWord, 'project', $project.path);
			addDictionaryWord(selectedWord);
			editor.view.dispatch(editor.state.tr.setMeta(DictionaryPluginKey, true));
			showDictContextMenu = false;
		} catch (err) {
			console.error('Failed to add word to dictionary:', err);
		}
	}
</script>

{#if !$hasStarted}
	<StartupModal {recentProjectPath} />
{/if}

{#if showExportModal && $hasStarted && $project}
	<ExportModal
		project={$project}
		chapters={$chapters}
		{selectedChapters}
		onClose={closeExportModal}
		onExportSuccess={handleExportSuccess}
	/>
{/if}

{#if showFindReplaceModal && $hasStarted && editor}
	<FindReplaceModal
		{editor}
		chapters={$chapters}
		{selectedChapters}
		searchAllChapters={!selectMode}
		onClose={() => { showFindReplaceModal = false; }}
		onNavigateChapter={(chapterId) => { selectChapter(chapterId); }}
	/>
{/if}

{#if showImportModal && $hasStarted && $project}
	<ImportModal
		project={$project}
		onClose={closeImportModal}
		onImportSuccess={handleImportSuccess}
	/>
{/if}

{#if showFontModal && $hasStarted}
	<FontModal
		currentProject={$project}
		onClose={() => { showFontModal = false; }}
		onFontChange={(font) => { appFont.set(font); showFontModal = false; }}
	/>
{/if}

{#if showDictContextMenu}
	<div
		class="context-menu"
		style="position: fixed; left: {contextMenuX}px; top: {contextMenuY}px; z-index: 2000;"
		role="menu"
	>
		<button class="context-menu-item" onclick={handleAddGlobalDict} role="menuitem">
			Add to Dictionary (Global)
		</button>
		<button class="context-menu-item" onclick={handleAddProjectDict} role="menuitem">
			Add to Dictionary (Project)
		</button>
	</div>
{/if}

{#if showChapterContextMenu && contextMenuChapterId !== null}
	<div
		class="context-menu"
		style="position: fixed; left: {chapterContextMenuX}px; top: {chapterContextMenuY}px; z-index: 2000;"
		role="menu"
	>
		<button
			class="context-menu-item context-menu-item--danger"
			onclick={() => {
				chapterToDelete = contextMenuChapterId;
				showChapterContextMenu = false;
				showDeleteConfirm = true;
			}}
			role="menuitem"
		>
			Delete Chapter
		</button>
	</div>
{/if}

{#if showDeleteConfirm && chapterToDelete !== null}
	<div class="confirm-overlay" role="dialog" aria-modal="true">
		<div class="confirm-dialog">
			<p>Delete <strong>{$chapters.find((ch: Chapter) => ch.id === chapterToDelete)?.title ?? 'this chapter'}</strong>?</p>
			<p class="confirm-subtext">This cannot be undone.</p>
			<div class="confirm-buttons">
				<button class="confirm-btn confirm-btn--danger" onclick={confirmDeleteChapter}>Delete</button>
				<button class="confirm-btn" onclick={() => { showDeleteConfirm = false; chapterToDelete = null; }}>Cancel</button>
			</div>
		</div>
	</div>
{/if}

<div class="app" class:hidden={!$hasStarted} style="--app-font: {$appFont};">
	<aside class="sidebar">
		<div class="sidebar-header">
			<div class="header-top">
				<h1 class="app-title">Scout</h1>
				<div class="header-buttons">
					<button class="header-btn" title="Import" onclick={openImportModal}>↑</button>
					<button class="header-btn" title="Export" onclick={openExportModal}>↓</button>
					<button class="close-project-btn" title="Close project" onclick={closeProject}>×</button>
				</div>
			</div>
			<p class="project-title">{$project?.title || 'Untitled Project'}</p>
		</div>

		<div class="select-chapters-toggle">
			<label class="checkbox-label">
				<input
					type="checkbox"
					checked={selectMode}
					onchange={toggleSelectMode}
				/>
				Select Chapters
			</label>
		</div>

		<nav class="chapter-list" ondragover={(e) => e.preventDefault()}>
			{#each $chapters as chapter (chapter.id)}
				{#if editingChapterId === chapter.id}
					<input
						type="text"
						class="chapter-edit-input"
						data-chapter={chapter.id}
						bind:value={editingTitle}
						onblur={saveChapterTitle}
						onkeydown={handleChapterTitleKeydown}
					/>
				{:else}
					<button
						class="chapter-item"
						class:active={chapter.id === activeChapterId && !selectMode}
						class:selected={selectMode && selectedChapters.has(chapter.id)}
						class:dragging={draggedChapterId === chapter.id}
						class:drag-over-before={dragOverChapterId === chapter.id && dragOverPosition === 'before'}
						class:drag-over-after={dragOverChapterId === chapter.id && dragOverPosition === 'after'}
						draggable={!selectMode}
						onclick={() => handleChapterClick(chapter.id)}
						ondblclick={() => startEditingChapter(chapter.id, chapter.title)}
						ondragstart={(e) => handleDragStart(e, chapter.id)}
						ondragover={(e) => handleDragOver(e, chapter.id)}
						ondragleave={handleDragLeave}
						ondrop={(e) => handleDrop(e, chapter.id)}
						ondragend={handleDragEnd}
						oncontextmenu={(e) => handleChapterContextMenu(e, chapter.id)}
					>
						{#if !selectMode}
							<span class="drag-handle" aria-hidden="true">⠿</span>
						{/if}
						{chapter.title}
						{#if $unsavedChapters.has(chapter.id)}
							<span class="unsaved-indicator">•</span>
						{/if}
					</button>
				{/if}
			{/each}
		</nav>

		<div class="sidebar-footer">
			<button class="new-chapter-btn" onclick={addChapter}>
				+ New Chapter
			</button>
		</div>
	</aside>

	<main class="editor-panel">
		<div class="editor-toolbar">
			{#if editor}
				<select
					class="toolbar-select"
					value={getCurrentStyle()}
					onchange={(e) => applyStyle((e.target as HTMLSelectElement).value)}
					title="Paragraph style"
				>
					<option value="paragraph">Normal</option>
					<option value="h2">Heading 2</option>
					<option value="h3">Heading 3</option>
					<option value="h4">Heading 4</option>
					<option value="h5">Heading 5</option>
					<option value="h6">Heading 6</option>
					<option value="blockquote">Blockquote</option>
				</select>

				<div class="toolbar-separator"></div>

				<button
					class="toolbar-btn"
					class:active={editor.isActive('bold')}
					onclick={() => editor.chain().focus().toggleBold().run()}
					title="Bold"
				>B</button>
				<button
					class="toolbar-btn italic"
					class:active={editor.isActive('italic')}
					onclick={() => editor.chain().focus().toggleItalic().run()}
					title="Italic"
				>I</button>
				<button
					class="toolbar-btn strikethrough"
					class:active={editor.isActive('strike')}
					onclick={() => editor.chain().focus().toggleStrike().run()}
					title="Strikethrough"
				>S</button>

				<div class="toolbar-separator"></div>

				<button
					class="toolbar-btn"
					class:active={editor.isActive('bulletList')}
					onclick={() => editor.chain().focus().toggleBulletList().run()}
					title="Bullet list"
				>•≡</button>
				<button
					class="toolbar-btn"
					class:active={editor.isActive('orderedList')}
					onclick={() => editor.chain().focus().toggleOrderedList().run()}
					title="Ordered list"
				>1≡</button>

				<div class="toolbar-separator"></div>

				<button
					class="toolbar-btn"
					title="Font"
					onclick={() => { showFontModal = true; }}
				>A▼</button>
			{/if}
		</div>

		<div class="editor-scroll">
			<div class="editor-content" bind:this={editorElement}></div>
		</div>
	</main>
</div>

<style>
	.app.hidden {
		display: none;
	}

	.unsaved-indicator {
		color: #cba6f7;
		margin-left: 0.5rem;
		font-weight: bold;
	}

	.header-top {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.close-project-btn {
		background: none;
		border: none;
		color: #cdd6f4;
		font-size: 1.5rem;
		cursor: pointer;
		padding: 0;
		width: 2rem;
		height: 2rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 4px;
		transition: background-color 0.15s;
	}

	.close-project-btn:hover {
		background-color: #45475a;
		color: #f38ba8;
	}

	.header-buttons {
		display: flex;
		gap: 0.25rem;
	}

	.header-btn {
		background: none;
		border: none;
		color: #cdd6f4;
		font-size: 1.2rem;
		cursor: pointer;
		padding: 0.25rem 0.5rem;
		width: 2rem;
		height: 2rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 4px;
		transition: background-color 0.15s;
	}

	.header-btn:hover {
		background-color: #45475a;
	}

	.select-chapters-toggle {
		padding: 0.75rem 1rem;
		border-top: 1px solid #313244;
		border-bottom: 1px solid #313244;
	}

	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		color: #cdd6f4;
		font-size: 0.9rem;
		user-select: none;
	}

	.checkbox-label input {
		cursor: pointer;
		accent-color: #cba6f7;
	}

	.chapter-item.selected {
		background-color: #45475a;
		color: #cba6f7;
	}

	.chapter-edit-input {
		display: block;
		width: 100%;
		padding: 0.5rem 1rem;
		margin: 0.5rem 0;
		background-color: #313244;
		border: 2px solid #cba6f7;
		border-radius: 4px;
		color: #cdd6f4;
		font-size: 0.9rem;
		font-family: inherit;
		outline: none;
	}

	.chapter-edit-input:focus {
		border-color: #cba6f7;
		box-shadow: 0 0 0 2px rgba(203, 166, 247, 0.1);
	}
	:global(body) {
		margin: 0;
		padding: 0;
		font-family: var(--app-font, Inter, Avenir, Helvetica, Arial, sans-serif);
		background-color: #f6f6f6;
		color: #0f0f0f;
	}

	.app {
		display: flex;
		height: 100vh;
		overflow: hidden;
	}

	/* Sidebar */
	.sidebar {
		width: 220px;
		min-width: 220px;
		background-color: #1e1e2e;
		color: #cdd6f4;
		display: flex;
		flex-direction: column;
		padding: 0;
	}

	.sidebar-header {
		padding: 1.5rem 1rem 1rem;
		border-bottom: 1px solid #313244;
	}

	.app-title {
		font-size: 1.4rem;
		font-weight: 700;
		margin: 0 0 0.25rem;
		color: #cba6f7;
	}

	.project-title {
		font-size: 0.8rem;
		color: #6c7086;
		margin: 0;
	}

	.chapter-list {
		flex: 1;
		overflow-y: auto;
		padding: 0.75rem 0;
	}

	.chapter-item {
		position: relative;
		display: flex;
		align-items: center;
		gap: 0.4rem;
		width: 100%;
		padding: 0.5rem 0.75rem 0.5rem 0.5rem;
		background: none;
		border: none;
		color: #cdd6f4;
		text-align: left;
		cursor: pointer;
		font-size: 0.9rem;
		border-left: 3px solid transparent;
		transition: background 0.15s, opacity 0.15s;
	}

	.chapter-item:hover {
		background-color: #313244;
	}

	.chapter-item.active {
		background-color: #313244;
		border-left-color: #cba6f7;
		color: #fff;
	}

	.chapter-item.dragging {
		opacity: 0.4;
	}

	.chapter-item.drag-over-before::before,
	.chapter-item.drag-over-after::after {
		content: '';
		position: absolute;
		left: 0;
		right: 0;
		height: 2px;
		background-color: #cba6f7;
		pointer-events: none;
	}

	.chapter-item.drag-over-before::before {
		top: 0;
	}

	.chapter-item.drag-over-after::after {
		bottom: 0;
	}

	.drag-handle {
		flex-shrink: 0;
		color: #6c7086;
		cursor: grab;
		font-size: 1rem;
		line-height: 1;
		user-select: none;
	}

	.chapter-item:hover .drag-handle {
		color: #a6adc8;
	}

	.sidebar-footer {
		padding: 1rem;
		border-top: 1px solid #313244;
	}

	.new-chapter-btn {
		width: 100%;
		padding: 0.5rem;
		background-color: #313244;
		border: none;
		border-radius: 6px;
		color: #cdd6f4;
		cursor: pointer;
		font-size: 0.85rem;
		transition: background 0.15s;
	}

	.new-chapter-btn:hover {
		background-color: #45475a;
	}

	/* Editor panel */
	.editor-panel {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background-color: #f6f6f6;
	}

	.editor-toolbar {
		display: flex;
		gap: 0.25rem;
		padding: 0.5rem 1rem;
		border-bottom: 1px solid #ddd;
		background-color: #fff;
	}

	.toolbar-btn {
		padding: 0.3rem 0.6rem;
		background: none;
		border: 1px solid #ddd;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.85rem;
		color: #0f0f0f;
		transition: background 0.15s;
	}

	.toolbar-btn.italic {
		font-style: italic;
	}

	.toolbar-btn.strikethrough {
		text-decoration: line-through;
	}

	.toolbar-btn:hover,
	.toolbar-btn.active {
		background-color: #e8e8f0;
		border-color: #aaa;
	}

	.toolbar-select {
		padding: 0.2rem 0.4rem;
		background: none;
		border: 1px solid #ddd;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.85rem;
		color: #0f0f0f;
		height: 28px;
	}

	.toolbar-select:hover {
		background-color: #e8e8f0;
		border-color: #aaa;
	}

	.toolbar-separator {
		width: 1px;
		height: 18px;
		background-color: #ddd;
		margin: 0 0.1rem;
		align-self: center;
		flex-shrink: 0;
	}

	.editor-scroll {
		flex: 1;
		overflow-y: auto;
		padding: 3rem;
		display: flex;
		justify-content: center;
	}

	.editor-content {
		width: 100%;
		max-width: 680px;
		min-height: 100%;
	}

	/* TipTap editor styles */
	:global(.tiptap) {
		outline: none;
		font-size: 1rem;
		line-height: 1.7;
		color: #0f0f0f;
		font-family: var(--app-font, Inter, Avenir, Helvetica, Arial, sans-serif);
	}

	:global(.tiptap p) {
		margin: 0 0 1em;
	}

	:global(.tiptap p.is-editor-empty:first-child::before) {
		content: attr(data-placeholder);
		float: left;
		color: #999;
		pointer-events: none;
		height: 0;
	}

	:global(.tiptap h2) { font-size: 1.6rem; margin: 1.5em 0 0.5em; }
	:global(.tiptap h3) { font-size: 1.3rem; margin: 1.5em 0 0.5em; }
	:global(.tiptap h4) { font-size: 1.1rem; margin: 1.5em 0 0.5em; }
	:global(.tiptap h5) { font-size: 1rem;   margin: 1.5em 0 0.5em; }
	:global(.tiptap h6) { font-size: 0.9rem; margin: 1.5em 0 0.5em; }

	:global(.tiptap blockquote) {
		border-left: 4px solid #cba6f7;
		margin: 1em 0;
		padding: 0.5em 1em;
		color: #555;
	}

	:global(.tiptap img) {
		max-width: 100%;
		display: block;
	}

	/* Dark mode */
	@media (prefers-color-scheme: dark) {
		:global(body) {
			background-color: #1e1e2e;
			color: #cdd6f4;
		}

		.editor-panel {
			background-color: #1e1e2e;
		}

		.editor-toolbar {
			background-color: #181825;
			border-bottom-color: #313244;
		}

		.toolbar-btn {
			color: #cdd6f4;
			border-color: #45475a;
		}

		.toolbar-btn:hover,
		.toolbar-btn.active {
			background-color: #313244;
			border-color: #6c7086;
		}

		.toolbar-select {
			color: #cdd6f4;
			border-color: #45475a;
			background-color: transparent;
		}

		.toolbar-select:hover {
			background-color: #313244;
			border-color: #6c7086;
		}

		.toolbar-separator {
			background-color: #45475a;
		}

		:global(.tiptap) {
			color: #cdd6f4;
		}

		:global(.tiptap blockquote) {
			color: #a6adc8;
		}
	}

	/* Dictionary context menu */
	.context-menu {
		background: white;
		border: 1px solid #ccc;
		border-radius: 4px;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
		min-width: 200px;
		overflow: hidden;
	}

	.context-menu-item {
		display: block;
		width: 100%;
		padding: 0.5rem 1rem;
		background: none;
		border: none;
		cursor: pointer;
		user-select: none;
		transition: background-color 0.15s;
		font-size: 0.9rem;
		color: #0f0f0f;
		text-align: left;
	}

	.context-menu-item:hover {
		background-color: #f0f0f0;
	}

	.context-menu-item--danger {
		color: #f38ba8;
	}

	.context-menu-item--danger:hover {
		background-color: rgba(243, 139, 168, 0.1);
	}

	/* Dark mode for context menu */
	@media (prefers-color-scheme: dark) {
		.context-menu {
			background: #1e1e2e;
			border-color: #313244;
			box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
		}

		.context-menu-item {
			color: #cdd6f4;
		}

		.context-menu-item:hover {
			background-color: #313244;
		}

		.context-menu-item:focus {
			outline: 1px solid #89b4fa;
		}
	}

	/* Delete confirmation dialog */
	.confirm-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.55);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 3000;
	}

	.confirm-dialog {
		background: #1e1e2e;
		border: 1px solid #313244;
		border-radius: 8px;
		padding: 1.5rem 2rem;
		min-width: 280px;
		color: #cdd6f4;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
	}

	.confirm-dialog p {
		margin: 0 0 0.4rem;
	}

	.confirm-subtext {
		font-size: 0.85rem;
		color: #6c7086;
	}

	.confirm-buttons {
		display: flex;
		gap: 0.75rem;
		margin-top: 1.25rem;
		justify-content: flex-end;
	}

	.confirm-btn {
		padding: 0.4rem 1.2rem;
		border-radius: 6px;
		border: 1px solid #45475a;
		background: #313244;
		color: #cdd6f4;
		cursor: pointer;
		font-size: 0.9rem;
		transition: background 0.15s;
	}

	.confirm-btn:hover {
		background: #45475a;
	}

	.confirm-btn--danger {
		background: #f38ba8;
		border-color: #f38ba8;
		color: #1e1e2e;
	}

	.confirm-btn--danger:hover {
		background: #eb6f92;
		border-color: #eb6f92;
	}
</style>
