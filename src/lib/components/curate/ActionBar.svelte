<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';
	import { formatRelative } from '$lib/wire';
	import { listGamemodes, type ModeId } from '$lib/gamemodes';
	import Button from '../Button.svelte';

	let {
		activeDraftId,
		loading = false,
		loadError = null,
		dirtyQuestions,
		dirtyDrafts,
		saving = false,
		onSelectDraft,
		onNewQuestion,
		onValidate,
		onSaveAll,
		onModeChange
	}: {
		activeDraftId: string;
		loading?: boolean;
		loadError?: string | null;
		dirtyQuestions?: Set<string>;
		dirtyDrafts?: Set<string>;
		saving?: boolean;
		onSelectDraft: (id: string) => void;
		onNewQuestion: () => void;
		onValidate: () => void;
		onSaveAll: () => void;
		onModeChange: (mode: ModeId) => void;
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
	const dirtyCount = $derived((dirtyQuestions?.size ?? 0) + (dirtyDrafts?.size ?? 0));
</script>

<header class="bar">
	<div class="left">
		<select
			class="select"
			value={activeDraftId}
			onchange={(e) => {
				onSelectDraft((e.target as HTMLSelectElement).value);
			}}
		>
			{#each pool.drafts as d (d.id)}
				<option value={d.id}>{d.title}</option>
			{/each}
		</select>
		<select
			class="select"
			aria-label="Gamemode"
			title="Gamemode"
			value={draft?.board.mode ?? 'grid_quiz'}
			onchange={(e) => {
				onModeChange((e.target as HTMLSelectElement).value as ModeId);
			}}
		>
			{#each listGamemodes() as g (g.id)}
				<option value={g.id}>{g.title}</option>
			{/each}
		</select>
		<span class="meta">
			<span class="status status-{draft?.status ?? 'incomplete'}">●</span>
			{statusLabel}
			{#if draft}· updated {formatRelative(draft.updated)}{/if}
		</span>
	</div>
	<div class="right">
		{#if loading}
			<span class="meta">Loading backend…</span>
		{:else if loadError}
			<span class="meta err" title={loadError}>backend offline</span>
		{:else if dirtyCount > 0}
			<span class="meta">{dirtyCount} dirty</span>
		{:else}
			<span class="meta ok">in sync</span>
		{/if}
		<Button variant="ghost" size="sm" onclick={onNewQuestion} disabled={loading}>+ New question</Button>
		<Button variant="ghost" size="sm" onclick={onValidate} disabled={loading}>Validate</Button>
		<Button variant="primary" size="sm" onclick={onSaveAll} disabled={loading || saving}>
			{saving ? 'Saving…' : `Save all${dirtyCount > 0 ? ` (${dirtyCount})` : ''}`}
		</Button>
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
	.select {
		padding: var(--space-1) var(--space-2);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		background: var(--bg-surface);
		color: var(--color-text);
		font-family: var(--font-body);
		font-size: calc(0.8rem * var(--font-scale));
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
		align-items: center;
	}
</style>
