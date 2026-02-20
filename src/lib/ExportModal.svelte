<script lang="ts">
	import { loading, error } from './stores';
	import { open } from '@tauri-apps/plugin-dialog';
	import {
		getDefaultExportDir,
		saveExportDir,
		exportProjectToRTF,
		exportProjectToEPUB,
		saveChapter,
	} from './fileIO';
	import type { Chapter, Project } from './types';

	export let project: Project & { path: string };
	export let chapters: Chapter[];
	export let selectedChapters: Set<number>;
	export let onClose: () => void;
	export let onExportSuccess: () => void;

	let exportDir = '';
	let rememberLocation = false;
	let isLoadingDir = true;
	let exportFormat: 'rtf' | 'epub' = 'epub';

	// Load default export directory on mount
	async function loadDefaultDir() {
		try {
			exportDir = await getDefaultExportDir(project.path);
		} catch (err) {
			exportDir = '';
		}
		isLoadingDir = false;
	}

	loadDefaultDir();

	async function handleChooseExportDir() {
		const selected = await open({
			directory: true,
			title: 'Choose export location',
		});

		if (selected) {
			exportDir = selected as string;
		}
	}

	async function handleExport() {
		if (!exportDir.trim()) {
			error.set('Please select an export location');
			return;
		}

		loading.set(true);
		error.set(null);

		try {
			// Always save all chapters before exporting to ensure they're on disk
			for (const chapter of chapters) {
				if (chapter.content) {
					await saveChapter(project.path, chapter.id, chapter.content);
				}
			}

			const chapterIds = selectedChapters.size > 0
			? Array.from(selectedChapters).sort((a, b) => a - b)
			: [];

			const filePath = exportFormat === 'epub'
				? await exportProjectToEPUB(project.path, exportDir, chapterIds)
				: await exportProjectToRTF(project.path, exportDir, chapterIds);

			// Save export directory if remember is checked
			if (rememberLocation) {
				await saveExportDir(project.path, exportDir);
			}

			error.set(null);
			onExportSuccess();
			onClose();
		} catch (err) {
			error.set(`Export failed: ${err instanceof Error ? err.message : String(err)}`);
		} finally {
			loading.set(false);
		}
	}

	function getChaptersToExport() {
		if (selectedChapters.size === 0) {
			return chapters;
		}
		return chapters.filter((ch) => selectedChapters.has(ch.id));
	}
</script>

<div class="modal-overlay">
	<div class="modal">
		<div class="modal-header">
			<h2>Export Project</h2>
			<button class="close-btn" onclick={onClose}>×</button>
		</div>

		{#if $error}
			<div class="error-message">{$error}</div>
		{/if}

		<div class="modal-content">
			<div class="section">
				<label>Format:</label>
				<div class="format-group">
					<label class="radio-label">
						<input type="radio" bind:group={exportFormat} value="epub" />
						EPUB <span class="format-hint">(ebook, recommended)</span>
					</label>
					<label class="radio-label">
						<input type="radio" bind:group={exportFormat} value="rtf" />
						RTF <span class="format-hint">(Word-compatible)</span>
					</label>
				</div>
			</div>

			<div class="section">
				<label>Export Location:</label>
				{#if isLoadingDir}
					<p class="loading">Loading...</p>
				{:else}
					<div class="location-selector">
						<input type="text" value={exportDir} readonly class="location-input" />
						<button
							class="btn btn-secondary"
							onclick={handleChooseExportDir}
							disabled={$loading}
						>
							Change
						</button>
					</div>
				{/if}
			</div>

			<div class="section">
				<label>Chapters to Export ({getChaptersToExport().length} of {chapters.length}):</label>
				<div class="chapter-list">
					{#if selectedChapters.size > 0}
						<p class="chapter-hint">Selected chapters will be exported</p>
					{:else}
						<p class="chapter-hint">All chapters will be exported</p>
					{/if}
					{#each getChaptersToExport() as chapter (chapter.id)}
						<div class="chapter-item">
							<span class="chapter-checkbox">☑</span>
							<span class="chapter-title">{chapter.title}</span>
						</div>
					{/each}
				</div>
			</div>

			<div class="section">
				<label class="checkbox">
					<input type="checkbox" bind:checked={rememberLocation} disabled={$loading} />
					Remember this location
				</label>
			</div>
		</div>

		<div class="modal-footer">
			<button class="btn btn-secondary" onclick={onClose} disabled={$loading}>
				Cancel
			</button>
			<button
				class="btn btn-primary"
				onclick={handleExport}
				disabled={$loading || !exportDir || isLoadingDir}
			>
				{$loading ? 'Exporting...' : 'Export'}
			</button>
		</div>
	</div>
</div>

<style>
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background-color: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background: white;
		border-radius: 8px;
		box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
		display: flex;
		flex-direction: column;
		max-width: 500px;
		width: 90%;
		max-height: 80vh;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid #e0e0e0;
	}

	.modal-header h2 {
		margin: 0;
		font-size: 1.3rem;
		color: #0f0f0f;
	}

	.close-btn {
		background: none;
		border: none;
		font-size: 1.5rem;
		cursor: pointer;
		color: #666;
		padding: 0;
		width: 2rem;
		height: 2rem;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.close-btn:hover {
		color: #000;
	}

	.error-message {
		background-color: #fee;
		border-left: 4px solid #f44;
		color: #c33;
		padding: 0.75rem 1rem;
		margin: 1rem;
		border-radius: 4px;
		font-size: 0.9rem;
	}

	.modal-content {
		padding: 1.5rem;
		overflow-y: auto;
		flex: 1;
	}

	.section {
		margin-bottom: 1.5rem;
	}

	.section label {
		display: block;
		font-weight: 500;
		color: #0f0f0f;
		margin-bottom: 0.5rem;
	}

	.location-selector {
		display: flex;
		gap: 0.5rem;
	}

	.location-input {
		flex: 1;
		padding: 0.5rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-family: monospace;
		font-size: 0.85rem;
		background-color: #f9f9f9;
	}

	.chapter-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-height: 150px;
		overflow-y: auto;
	}

	.chapter-hint {
		margin: 0 0 0.5rem;
		font-size: 0.85rem;
		color: #999;
		font-style: italic;
	}

	.chapter-item {
		padding: 0.5rem;
		background-color: #f5f5f5;
		border-radius: 4px;
		font-size: 0.9rem;
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.chapter-checkbox {
		color: #cba6f7;
		font-size: 0.8rem;
	}

	.chapter-title {
		color: #666;
	}

	.format-group {
		display: flex;
		gap: 1.5rem;
	}

	.radio-label {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		cursor: pointer;
		font-size: 0.9rem;
		font-weight: normal;
	}

	.radio-label input {
		cursor: pointer;
		accent-color: #cba6f7;
	}

	.format-hint {
		color: #999;
		font-size: 0.8rem;
	}

	.checkbox {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		font-weight: normal;
	}

	.checkbox input {
		cursor: pointer;
	}

	.loading {
		color: #999;
		font-size: 0.9rem;
	}

	.modal-footer {
		display: flex;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid #e0e0e0;
	}

	.btn {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.95rem;
		font-weight: 500;
		transition: all 0.15s;
		flex: 1;
	}

	.btn-primary {
		background-color: #cba6f7;
		color: white;
	}

	.btn-primary:hover:not(:disabled) {
		background-color: #b896e7;
	}

	.btn-secondary {
		background-color: #f0f0f0;
		color: #0f0f0f;
		border: 1px solid #ddd;
	}

	.btn-secondary:hover:not(:disabled) {
		background-color: #e8e8e8;
	}

	.btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	/* Dark mode */
	@media (prefers-color-scheme: dark) {
		.modal {
			background: #1e1e2e;
			color: #cdd6f4;
		}

		.modal-header {
			border-bottom-color: #313244;
		}

		.modal-header h2 {
			color: #cdd6f4;
		}

		.close-btn {
			color: #a6adc8;
		}

		.close-btn:hover {
			color: #cdd6f4;
		}

		.error-message {
			background-color: #3a1f1f;
			border-left-color: #f0a0a0;
			color: #f0a0a0;
		}

		.section label {
			color: #cdd6f4;
		}

		.location-input {
			background-color: #313244;
			border-color: #45475a;
			color: #cdd6f4;
		}

		.chapter-item {
			background-color: #313244;
		}

		.chapter-title {
			color: #a6adc8;
		}

		.modal-footer {
			border-top-color: #313244;
		}

		.btn-secondary {
			background-color: #313244;
			color: #cdd6f4;
			border-color: #45475a;
		}

		.btn-secondary:hover:not(:disabled) {
			background-color: #45475a;
		}
	}
</style>
