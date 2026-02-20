<script lang="ts">
	import type { PageSettings } from './types';

	export let settings: PageSettings;
	export let onClose: () => void;
	export let onSave: (s: PageSettings) => void;

	let local: PageSettings = {
		...settings,
		margins: { ...settings.margins },
	};

	function handleApply() {
		onSave(local);
	}

	function handleOverlayClick(e: MouseEvent) {
		if (e.target === e.currentTarget) onClose();
	}
</script>

<div class="modal-overlay" role="dialog" aria-modal="true" onclick={handleOverlayClick}>
	<div class="modal">
		<h2>Page Settings</h2>

		<section>
			<h3>Page</h3>
			<div class="field-row">
				<label for="paper-size">Paper size</label>
				<select id="paper-size" bind:value={local.paperSize}>
					<optgroup label="Standard">
						<option value="letter">Letter (8.5 × 11 in)</option>
						<option value="a4">A4 (210 × 297 mm)</option>
					</optgroup>
					<optgroup label="Paperback / Ebook">
						<option value="trade">Trade Paperback (6 × 9 in)</option>
						<option value="digest">Digest Paperback (5.5 × 8.5 in)</option>
						<option value="pocket">Pocket / Ebook (5 × 8 in)</option>
					</optgroup>
				</select>
			</div>

			<fieldset>
				<legend>Margins (inches)</legend>
				<div class="margins-grid">
					<label>
						Top
						<input type="number" bind:value={local.margins.top} min="0" max="4" step="0.25" />
					</label>
					<label>
						Bottom
						<input type="number" bind:value={local.margins.bottom} min="0" max="4" step="0.25" />
					</label>
					<label>
						Left
						<input type="number" bind:value={local.margins.left} min="0" max="4" step="0.25" />
					</label>
					<label>
						Right
						<input type="number" bind:value={local.margins.right} min="0" max="4" step="0.25" />
					</label>
				</div>
			</fieldset>
		</section>

		<section>
			<h3>Paragraph</h3>
			<div class="field-row">
				<label for="text-indent">First-line indent (in)</label>
				<input
					id="text-indent"
					type="number"
					bind:value={local.textIndent}
					min="0"
					max="2"
					step="0.25"
				/>
			</div>
			<div class="field-row">
				<label for="para-spacing">Space after paragraph (pt)</label>
				<input
					id="para-spacing"
					type="number"
					bind:value={local.paragraphSpacing}
					min="0"
					max="72"
					step="1"
				/>
			</div>
			<div class="field-row">
				<span class="field-label">Alignment</span>
				<div class="radio-group">
					<label class="radio-label">
						<input type="radio" bind:group={local.alignment} value="left" />
						Left
					</label>
					<label class="radio-label">
						<input type="radio" bind:group={local.alignment} value="justify" />
						Justify
					</label>
				</div>
			</div>
		</section>

		<section>
			<h3>Page Numbers</h3>
			<div class="field-row">
				<label class="checkbox-row">
					<input type="checkbox" bind:checked={local.pageNumbering} />
					Show page count in status bar
				</label>
			</div>
			{#if local.pageNumbering}
				<div class="field-row">
					<label for="first-page-num">First page number</label>
					<input
						id="first-page-num"
						type="number"
						bind:value={local.firstPageNumber}
						min="0"
						max="999"
						step="1"
					/>
				</div>
				<div class="field-row">
					<span class="field-label">Position</span>
					<div class="radio-group">
						<label class="radio-label">
							<input type="radio" bind:group={local.pageNumberPosition} value="bottom-center" />
							Bottom Center
						</label>
						<label class="radio-label">
							<input type="radio" bind:group={local.pageNumberPosition} value="bottom-outside" />
							Bottom Outside Corner
						</label>
						<label class="radio-label">
							<input type="radio" bind:group={local.pageNumberPosition} value="top-outside" />
							Top Outside Corner
						</label>
					</div>
				</div>
			{/if}
		</section>

		<div class="modal-buttons">
			<button class="modal-btn primary" onclick={handleApply}>Apply</button>
			<button class="modal-btn secondary" onclick={onClose}>Cancel</button>
		</div>
	</div>
</div>

<style>
	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background: white;
		border-radius: 8px;
		padding: 1.5rem 2rem 2rem;
		width: 420px;
		max-height: 90vh;
		overflow-y: auto;
		box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
	}

	h2 {
		margin: 0 0 1.25rem;
		font-size: 1.4rem;
		color: #0f0f0f;
	}

	h3 {
		margin: 0 0 0.75rem;
		font-size: 0.95rem;
		font-weight: 600;
		color: #0f0f0f;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	section {
		margin-bottom: 1.5rem;
		padding-bottom: 1.5rem;
		border-bottom: 1px solid #e8e8e8;
	}

	section:last-of-type {
		border-bottom: none;
		margin-bottom: 1rem;
	}

	fieldset {
		border: 1px solid #ddd;
		border-radius: 4px;
		padding: 0.75rem;
		margin: 0;
	}

	legend {
		font-size: 0.85rem;
		color: #555;
		padding: 0 0.25rem;
	}

	.margins-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.5rem;
	}

	.margins-grid label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.85rem;
		color: #333;
	}

	.field-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		margin-bottom: 0.6rem;
	}

	.field-row label,
	.field-label {
		font-size: 0.9rem;
		color: #333;
		flex-shrink: 0;
	}

	.field-row select,
	.field-row input[type="number"] {
		padding: 0.3rem 0.4rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 0.9rem;
		font-family: inherit;
		background: white;
		color: #0f0f0f;
		width: 100px;
	}

	.field-row select {
		width: auto;
		flex: 1;
	}

	.margins-grid input[type="number"] {
		padding: 0.3rem 0.4rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 0.9rem;
		font-family: inherit;
		background: white;
		color: #0f0f0f;
		width: 100%;
	}

	.radio-group {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		align-items: flex-end;
	}

	.radio-label {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.9rem;
		color: #333;
		cursor: pointer;
	}

	.checkbox-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.9rem;
		color: #333;
		cursor: pointer;
	}

	.modal-buttons {
		display: flex;
		gap: 0.75rem;
		justify-content: flex-end;
	}

	.modal-btn {
		padding: 0.5rem 1.25rem;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.95rem;
		font-weight: 500;
		transition: background 0.15s;
	}

	.modal-btn.primary {
		background-color: #cba6f7;
		color: white;
	}

	.modal-btn.primary:hover {
		background-color: #b896e7;
	}

	.modal-btn.secondary {
		background-color: #f0f0f0;
		color: #0f0f0f;
		border: 1px solid #ddd;
	}

	.modal-btn.secondary:hover {
		background-color: #e8e8e8;
	}

	/* Dark mode */
	@media (prefers-color-scheme: dark) {
		.modal {
			background: #1e1e2e;
			color: #cdd6f4;
		}

		h2, h3 {
			color: #cdd6f4;
		}

		section {
			border-bottom-color: #313244;
		}

		fieldset {
			border-color: #45475a;
		}

		legend {
			color: #a6adc8;
		}

		.margins-grid label,
		.field-row label,
		.field-label,
		.radio-label,
		.checkbox-row {
			color: #cdd6f4;
		}

		.field-row select,
		.field-row input[type="number"],
		.margins-grid input[type="number"] {
			background: #313244;
			border-color: #45475a;
			color: #cdd6f4;
		}

		.modal-btn.secondary {
			background-color: #313244;
			color: #cdd6f4;
			border-color: #45475a;
		}

		.modal-btn.secondary:hover {
			background-color: #45475a;
		}
	}
</style>
