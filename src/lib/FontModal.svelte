<script lang="ts">
	import { updateFont, updateProjectFont } from './fileIO';
	import type { Project } from './types';

	export let currentProject: (Project & { path: string }) | undefined;
	export let onClose: () => void;
	export let onFontChange: (font: string) => void;

	let selectedFont = 'Inter, Avenir, Helvetica, Arial, sans-serif';
	let isSaving = false;

	const fontCategories = {
		'Sans-serif': [
			{ label: 'Default', value: 'Inter, Avenir, Helvetica, Arial, sans-serif' },
			{ label: 'Segoe UI', value: '"Segoe UI", sans-serif' },
			{ label: 'Helvetica Neue', value: '"Helvetica Neue", Helvetica, sans-serif' },
			{ label: 'Arial', value: 'Arial, sans-serif' },
			{ label: 'Verdana', value: 'Verdana, sans-serif' },
			{ label: 'Trebuchet MS', value: '"Trebuchet MS", sans-serif' },
		],
		'Serif': [
			{ label: 'Georgia', value: 'Georgia, serif' },
			{ label: 'Times New Roman', value: '"Times New Roman", serif' },
			{ label: 'Garamond', value: 'Garamond, serif' },
			{ label: 'Palatino', value: '"Palatino Linotype", Palatino, serif' },
		],
		'Monospace': [
			{ label: 'Courier New', value: '"Courier New", monospace' },
			{ label: 'Consolas', value: 'Consolas, monospace' },
			{ label: 'Monaco', value: 'Monaco, monospace' },
			{ label: 'Menlo', value: 'Menlo, monospace' },
		],
	};

	async function handleSave() {
		isSaving = true;
		try {
			if (currentProject) {
				// Save to project-level
				await updateProjectFont(currentProject.path, selectedFont);
			} else {
				// Save to app-level
				await updateFont(selectedFont);
			}
			onFontChange(selectedFont);
		} catch (err) {
			console.error('Failed to save font:', err);
		} finally {
			isSaving = false;
		}
	}

	function getFontLabel(value: string): string {
		for (const category of Object.values(fontCategories)) {
			const font = category.find(f => f.value === value);
			if (font) return font.label;
		}
		return 'Custom';
	}
</script>

<div class="modal-overlay" role="presentation" onclick={onClose}>
	<div class="modal" role="dialog" onclick={(e) => e.stopPropagation()}>
		<div class="modal-header">
			<h2>Select Font</h2>
			<button class="close-btn" onclick={onClose}>×</button>
		</div>

		<div class="modal-content">
			<div class="font-section">
				{#each Object.entries(fontCategories) as [category, fonts]}
					<div class="category">
						<h3 class="category-title">{category}</h3>
						<div class="font-list">
							{#each fonts as font}
								<label
									class="font-item"
									style="font-family: {font.value}"
									class:selected={selectedFont === font.value}
								>
									<input
										type="radio"
										bind:group={selectedFont}
										value={font.value}
									/>
									<span class="font-label">{font.label}</span>
								</label>
							{/each}
						</div>
					</div>
				{/each}
			</div>

			<div class="preview-section">
				<label>Preview:</label>
				<div class="preview" style="font-family: {selectedFont}">
					The quick brown fox jumps over the lazy dog
				</div>
				<div class="preview-heading" style="font-family: {selectedFont}">
					Heading Example
				</div>
			</div>
		</div>

		<div class="modal-footer">
			<button class="btn btn-secondary" onclick={onClose} disabled={isSaving}>
				Cancel
			</button>
			<button
				class="btn btn-primary"
				onclick={handleSave}
				disabled={isSaving}
			>
				{isSaving ? 'Saving...' : 'Save'}
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

	.modal-content {
		padding: 1.5rem;
		overflow-y: auto;
		flex: 1;
	}

	.font-section {
		margin-bottom: 1.5rem;
	}

	.category {
		margin-bottom: 1.5rem;
	}

	.category-title {
		font-size: 0.9rem;
		font-weight: 600;
		color: #666;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin: 0 0 0.75rem;
	}

	.font-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.font-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 0.75rem;
		border-radius: 4px;
		cursor: pointer;
		border: 1px solid transparent;
		transition: all 0.15s;
		background: none;
	}

	.font-item:hover {
		background-color: #f5f5f5;
		border-color: #ddd;
	}

	.font-item:focus {
		outline: 2px solid #cba6f7;
		outline-offset: -2px;
	}

	.font-item.selected {
		background-color: #e8e0f5;
		border-color: #cba6f7;
	}

	.font-item input {
		cursor: pointer;
		margin: 0;
	}

	.font-label {
		font-size: 0.95rem;
		color: #0f0f0f;
		user-select: none;
	}

	.font-item.selected .font-label {
		color: #cba6f7;
		font-weight: 500;
	}

	.preview-section {
		border-top: 1px solid #e0e0e0;
		padding-top: 1.5rem;
		margin-top: 1.5rem;
	}

	.preview-section label {
		display: block;
		font-weight: 500;
		color: #0f0f0f;
		margin-bottom: 0.5rem;
		font-size: 0.9rem;
	}

	.preview {
		padding: 1rem;
		background-color: #f9f9f9;
		border-radius: 4px;
		border: 1px solid #ddd;
		color: #0f0f0f;
		font-size: 1rem;
		line-height: 1.5;
		margin-bottom: 0.75rem;
	}

	.preview-heading {
		padding: 1rem;
		background-color: #f9f9f9;
		border-radius: 4px;
		border: 1px solid #ddd;
		color: #0f0f0f;
		font-size: 1.3rem;
		font-weight: 600;
		line-height: 1.4;
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

		.category-title {
			color: #a6adc8;
		}

		.font-item:hover {
			background-color: #313244;
			border-color: #45475a;
		}

		.font-item.selected {
			background-color: #45475a;
			border-color: #cba6f7;
		}

		.font-label {
			color: #cdd6f4;
		}

		.font-item.selected .font-label {
			color: #cba6f7;
		}

		.preview-section {
			border-top-color: #313244;
		}

		.preview-section label {
			color: #cdd6f4;
		}

		.preview {
			background-color: #313244;
			border-color: #45475a;
			color: #cdd6f4;
		}

		.preview-heading {
			background-color: #313244;
			border-color: #45475a;
			color: #cdd6f4;
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
