<script lang="ts">
	import { loading, error, setProject } from './stores';
	import {
		createProject,
		openProject,
		openRecentProject,
		writeConfig,
	} from './fileIO';

	export let recentProjectPath: string | null = null;

	let projectTitle = '';
	let isCreating = false;

	async function handleNewProject() {
		if (!projectTitle.trim()) {
			error.set('Please enter a project title');
			return;
		}

		isCreating = true;
		loading.set(true);
		error.set(null);

		try {
			const response = await createProject(projectTitle);
			setProject(
				{
					title: response.project.title,
					author: response.project.author,
					chapterOrder: response.project.chapterOrder || [],
					path: response.path,
				},
				[]
			);
		} catch (err) {
			error.set(`Failed to create project: ${err instanceof Error ? err.message : String(err)}`);
		} finally {
			isCreating = false;
			loading.set(false);
		}
	}

	async function handleOpenProject() {
		loading.set(true);
		error.set(null);

		try {
			const response = await openProject();
			setProject(
				{
					title: response.project.title,
					author: response.project.author,
					chapterOrder: response.project.chapterOrder || [],
					path: response.path,
				},
				response.chapters
			);
		} catch (err) {
			error.set(`Failed to open project: ${err instanceof Error ? err.message : String(err)}`);
		} finally {
			loading.set(false);
		}
	}

	async function handleRecentProject() {
		if (!recentProjectPath) return;

		loading.set(true);
		error.set(null);

		try {
			const response = await openRecentProject(recentProjectPath);
			setProject(
				{
					title: response.project.title,
					author: response.project.author,
					chapterOrder: response.project.chapterOrder || [],
					path: response.path,
				},
				response.chapters
			);
		} catch (err) {
			error.set(
				`Failed to open recent project: ${err instanceof Error ? err.message : String(err)}`
			);
		} finally {
			loading.set(false);
		}
	}
</script>

<div class="modal-overlay">
	<div class="modal">
		<h2>Scout</h2>
		<p class="subtitle">Create or open a project to get started</p>

		{#if $error}
			<div class="error-message">{$error}</div>
		{/if}

		{#if !isCreating}
			<div class="button-group">
				<button class="modal-btn primary" onclick={handleOpenProject} disabled={$loading}>
					{$loading ? 'Loading...' : 'Open Project'}
				</button>

				{#if recentProjectPath}
					<button class="modal-btn secondary" onclick={handleRecentProject} disabled={$loading}>
						Open Recent
					</button>
				{/if}

				<div class="divider">or</div>

				<button
					class="modal-btn secondary"
					onclick={() => (isCreating = true)}
					disabled={$loading}
				>
					New Project
				</button>
			</div>
		{:else}
			<div class="create-form">
				<label>
					Project Title
					<input
						type="text"
						bind:value={projectTitle}
						placeholder="Enter project title"
						disabled={$loading}
					/>
				</label>

				<div class="form-buttons">
					<button
						class="modal-btn primary"
						onclick={handleNewProject}
						disabled={$loading || !projectTitle.trim()}
					>
						{$loading ? 'Creating...' : 'Create Project'}
					</button>
					<button
						class="modal-btn secondary"
						onclick={() => (isCreating = false)}
						disabled={$loading}
					>
						Back
					</button>
				</div>
			</div>
		{/if}
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
		padding: 2rem;
		max-width: 400px;
		width: 100%;
		box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
	}

	h2 {
		margin: 0 0 0.5rem;
		font-size: 1.8rem;
		color: #0f0f0f;
	}

	.subtitle {
		margin: 0 0 1.5rem;
		color: #666;
		font-size: 0.95rem;
	}

	.error-message {
		background-color: #fee;
		border: 1px solid #fcc;
		border-radius: 4px;
		padding: 0.75rem;
		margin-bottom: 1rem;
		color: #c33;
		font-size: 0.9rem;
	}

	.button-group {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.modal-btn {
		padding: 0.75rem 1rem;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.95rem;
		font-weight: 500;
		transition: all 0.15s;
	}

	.modal-btn.primary {
		background-color: #cba6f7;
		color: white;
	}

	.modal-btn.primary:hover:not(:disabled) {
		background-color: #b896e7;
	}

	.modal-btn.secondary {
		background-color: #f0f0f0;
		color: #0f0f0f;
		border: 1px solid #ddd;
	}

	.modal-btn.secondary:hover:not(:disabled) {
		background-color: #e8e8e8;
	}

	.modal-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.divider {
		text-align: center;
		color: #999;
		font-size: 0.9rem;
		margin: 0.5rem 0;
	}

	.create-form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		font-weight: 500;
		color: #0f0f0f;
	}

	input {
		padding: 0.5rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 0.95rem;
		font-family: inherit;
	}

	input:focus {
		outline: none;
		border-color: #cba6f7;
		box-shadow: 0 0 0 2px rgba(203, 166, 247, 0.1);
	}

	.form-buttons {
		display: flex;
		gap: 0.75rem;
	}

	.form-buttons .modal-btn {
		flex: 1;
	}

	/* Dark mode */
	@media (prefers-color-scheme: dark) {
		.modal {
			background: #1e1e2e;
			color: #cdd6f4;
		}

		h2 {
			color: #cba6f7;
		}

		.subtitle {
			color: #a6adc8;
		}

		.error-message {
			background-color: #3a1f1f;
			border-color: #6b3a3a;
			color: #f0a0a0;
		}

		.modal-btn.secondary {
			background-color: #313244;
			color: #cdd6f4;
			border-color: #45475a;
		}

		.modal-btn.secondary:hover:not(:disabled) {
			background-color: #45475a;
		}

		.divider {
			color: #6c7086;
		}

		label {
			color: #cdd6f4;
		}

		input {
			background-color: #313244;
			border-color: #45475a;
			color: #cdd6f4;
		}

		input:focus {
			border-color: #cba6f7;
			box-shadow: 0 0 0 2px rgba(203, 166, 247, 0.2);
		}
	}
</style>
