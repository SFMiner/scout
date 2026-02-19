<script lang="ts">
	import { loading, error } from './stores';
	import { open } from '@tauri-apps/plugin-dialog';
	import { importChaptersFromFiles } from './fileIO';
	import type { Chapter, Project } from './types';

	export let project: Project & { path: string };
	export let onClose: () => void;
	export let onImportSuccess: (chapters: Chapter[]) => void;

	let selectedFiles: string[] = [];
	let useFilenameAsTitle = true;
	let showPreview = false;
	let chapterDelimiter = '';
	let extractTitleFromDelimiter = true;

	async function handleSelectFiles() {
		const selected = await open({
			multiple: true,
			title: 'Select files to import',
			filters: [
				{
					name: 'Text/Markdown',
					extensions: ['txt', 'md'],
				},
			],
		});

		if (selected) {
			selectedFiles = Array.isArray(selected) ? selected : [selected];
		}
	}

	async function handleImport() {
		if (selectedFiles.length === 0) {
			error.set('Please select at least one file');
			return;
		}

		loading.set(true);
		error.set(null);

		try {
			const newChapters = await importChaptersFromFiles(
				project.path,
				selectedFiles,
				useFilenameAsTitle,
				chapterDelimiter || undefined,
				extractTitleFromDelimiter
			);

			if (newChapters.length === 0) {
				error.set('No chapters were imported');
				loading.set(false);
				return;
			}

			error.set(null);
			onImportSuccess(newChapters);
			onClose();
		} catch (err) {
			error.set(`Import failed: ${err instanceof Error ? err.message : String(err)}`);
		} finally {
			loading.set(false);
		}
	}

	function getFileName(path: string): string {
		const parts = path.split('\\');
		return parts[parts.length - 1];
	}
</script>

<div class="modal-overlay">
	<div class="modal">
		<div class="modal-header">
			<h2>Import Chapters</h2>
			<button class="close-btn" onclick={onClose}>×</button>
		</div>

		{#if $error}
			<div class="error-message">{$error}</div>
		{/if}

		<div class="modal-content">
			<div class="section">
				<label>Select Files:</label>
				<div class="file-selector">
					<button
						class="btn btn-secondary"
						onclick={handleSelectFiles}
						disabled={$loading}
					>
						Choose Files
					</button>
				</div>
			</div>

			{#if selectedFiles.length > 0}
				<div class="section">
					<label>Files to Import ({selectedFiles.length}):</label>
					<div class="file-list">
						{#each selectedFiles as filePath (filePath)}
							<div class="file-item">
								<span class="file-icon">📄</span>
								<span class="file-name">{getFileName(filePath)}</span>
							</div>
						{/each}
					</div>
				</div>

				<div class="section">
					<label class="checkbox">
						<input
							type="checkbox"
							bind:checked={useFilenameAsTitle}
							disabled={$loading}
						/>
						Use filename as chapter title
					</label>
				</div>

				<div class="section">
					<label>Chapter Delimiter (optional):</label>
					<input
						type="text"
						placeholder="e.g., ## or CHAPTER"
						bind:value={chapterDelimiter}
						class="delimiter-input"
						disabled={$loading}
					/>
					{#if chapterDelimiter}
						<div class="delimiter-options">
							<label class="radio-label">
								<input
									type="radio"
									bind:group={extractTitleFromDelimiter}
									value={true}
									disabled={$loading}
								/>
								Extract title from delimiter line
							</label>
							<label class="radio-label">
								<input
									type="radio"
									bind:group={extractTitleFromDelimiter}
									value={false}
									disabled={$loading}
								/>
								Use sequential titles (Chapter 1, Chapter 2, ...)
							</label>
						</div>
					{/if}
				</div>
			{/if}
		</div>

		<div class="modal-footer">
			<button class="btn btn-secondary" onclick={onClose} disabled={$loading}>
				Cancel
			</button>
			<button
				class="btn btn-primary"
				onclick={handleImport}
				disabled={$loading || selectedFiles.length === 0}
			>
				{$loading ? 'Importing...' : 'Import'}
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

	.file-selector {
		display: flex;
		gap: 0.5rem;
	}

	.file-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-height: 200px;
		overflow-y: auto;
	}

	.file-item {
		padding: 0.75rem;
		background-color: #f5f5f5;
		border-radius: 4px;
		font-size: 0.9rem;
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.file-icon {
		color: #999;
		font-size: 1rem;
	}

	.file-name {
		color: #333;
		word-break: break-all;
		font-family: monospace;
		font-size: 0.85rem;
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

	.delimiter-input {
		width: 100%;
		padding: 0.5rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 0.95rem;
		box-sizing: border-box;
	}

	.delimiter-options {
		margin-top: 0.75rem;
		padding: 0.75rem;
		background-color: #f5f5f5;
		border-radius: 4px;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.radio-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		font-size: 0.9rem;
		font-weight: normal;
	}

	.radio-label input[type="radio"] {
		cursor: pointer;
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

		.file-item {
			background-color: #313244;
		}

		.file-name {
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

		.delimiter-input {
			background-color: #313244;
			border-color: #45475a;
			color: #cdd6f4;
		}

		.delimiter-options {
			background-color: #313244;
		}

		.radio-label {
			color: #cdd6f4;
		}
	}
</style>
