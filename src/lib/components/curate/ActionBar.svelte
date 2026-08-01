<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';
	import { formatRelative } from '$lib/data/seed';
	import Button from '../Button.svelte';

	let {
		activeDraftId,
		onNewQuestion,
		onValidate,
		onPreview,
		onSaveAll
	}: {
		activeDraftId: string;
		onNewQuestion: () => void;
		onValidate: () => void;
		onPreview: () => void;
		onSaveAll: () => void;
	} = $props();

	const draft = $derived(pool.getDraft(activeDraftId));
	const statusLabel = $derived(
		draft
			? draft.status === 'saved'
				? 'Saved'
				: draft.status === 'unsaved_changes'
					? 'Unsaved changes'
					: draft.status === 'invalid'
						? 'Validation errors'
						: 'Incomplete'
			: '—'
	);
</script>

<header class="bar">
	<div class="left">
		<select class="draft-select" value={activeDraftId} onchange={() => undefined /* re-derive */}>
			{#each pool.drafts as d (d.id)}
				<option value={d.id}>{d.title}</option>
			{/each}
		</select>
		<span class="meta">
			<span class="status status-{draft?.status ?? 'incomplete'}">●</span>
			{statusLabel}
			{#if draft}· updated {formatRelative(draft.updated)}{/if}
		</span>
	</div>
	<div class="right">
		<Button variant="ghost" size="sm" onclick={onNewQuestion}>+ New question</Button>
		<Button variant="ghost" size="sm" onclick={onPreview}>Preview</Button>
		<Button variant="ghost" size="sm" onclick={onValidate}>Validate</Button>
		<Button variant="primary" size="sm" onclick={onSaveAll}>Save all</Button>
	</div>
</header>

<style>
	.bar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-3) var(--space-4);
		background: var(--bg-surface);
		border-bottom: var(--border-width) var(--border-style) var(--border-color);
		gap: var(--space-3);
	}
	.left {
		display: flex;
		gap: var(--space-3);
		align-items: center;
		min-width: 0;
	}
	.draft-select {
		padding: var(--space-1) var(--space-2);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		background: var(--bg-primary);
		color: var(--color-text);
		font-family: var(--font-body);
		font-weight: 600;
		font-size: calc(0.95rem * var(--font-scale));
	}
	.meta {
		color: var(--color-text-muted);
		font-size: calc(0.8rem * var(--font-scale));
	}
	.status {
		font-size: 0.6em;
		margin-right: 2px;
	}
	.status-saved {
		color: var(--color-success);
	}
	.status-unsaved_changes {
		color: var(--color-accent);
	}
	.status-invalid {
		color: var(--color-danger);
	}
	.status-incomplete {
		color: var(--color-text-muted);
	}
	.right {
		display: flex;
		gap: var(--space-2);
	}
</style>
