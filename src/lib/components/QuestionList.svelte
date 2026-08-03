<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';
	import { isDraftId } from '$lib/data/ids';
	import type { PoolQuestion, QuestionKind, QuestionStatus } from '$lib/data/seed';
	import type { Snippet } from 'svelte';

	type KindFilter = 'all' | QuestionKind;
	type StatusFilter = 'all' | QuestionStatus;
	type LangFilter = 'all' | 'de' | 'en';

	let {
		query = $bindable(''),
		kindFilter = $bindable<KindFilter>('all'),
		statusFilter = $bindable<StatusFilter>('all'),
		langFilter = $bindable<LangFilter>('all'),
		max = Infinity,
		showRefs = false,
		actions
	}: {
		query?: string;
		kindFilter?: KindFilter;
		statusFilter?: StatusFilter;
		langFilter?: LangFilter;
		max?: number;
		showRefs?: boolean;
		/** Trailing slot rendered inside each row (e.g. "→" indicator, "Edit" link). */
		actions?: Snippet<[PoolQuestion]>;
	} = $props();

	const filtered = $derived.by(() => {
		const q = query.trim().toLowerCase();
		return pool.questions.filter((qq) => {
			if (q && !`${qq.id} ${qq.prompt} ${qq.answer} ${qq.tags.join(' ')}`.toLowerCase().includes(q))
				return false;
			if (kindFilter !== 'all' && qq.kind !== kindFilter) return false;
			if (statusFilter !== 'all' && qq.status !== statusFilter) return false;
			if (langFilter !== 'all' && qq.defaultLang !== langFilter) return false;
			return true;
		});
	});

	const shown = $derived(filtered.slice(0, max));
	const overflow = $derived(filtered.length - shown.length);
</script>

<div class="filters">
	<input
		class="search"
		bind:value={query}
		placeholder="Search id, prompt, answer, tag…"
		aria-label="Search questions"
	/>
	<div class="row">
		<label>
			Kind
			<select bind:value={kindFilter}>
				<option value="all">all</option>
				<option value="text">text</option>
				<option value="numeric">numeric</option>
				<option value="order">order</option>
			</select>
		</label>
		<label>
			Status
			<select bind:value={statusFilter}>
				<option value="all">all</option>
				<option value="named">named</option>
				<option value="draft">draft</option>
				<option value="referenced">referenced</option>
			</select>
		</label>
		<label>
			Lang
			<select bind:value={langFilter}>
				<option value="all">all</option>
				<option value="de">de</option>
				<option value="en">en</option>
			</select>
		</label>
	</div>
	<p class="ct">{filtered.length} of {pool.questions.length} questions</p>
</div>

<ul class="list">
	{#each shown as q (q.id)}
		<li>
			<div class="item">
				<div class="i-head">
					<code class="i-id" class:draft={isDraftId(q.id)}>{q.id}</code>
					<span class="badge">{q.kind}</span>
					<span class="badge">{q.defaultLang}</span>
					{#if showRefs && q.status === 'referenced'}
						<span class="badge badge-ref">ref ×{pool.referenceLocations(q.id).length}</span>
					{/if}
				</div>
				<div class="i-prompt">{q.prompt || '(empty prompt)'}</div>
				<div class="i-meta">→ {q.answer || '(no answer)'}</div>
				{#if actions}{@render actions(q)}{/if}
			</div>
		</li>
	{/each}
	{#if overflow > 0}
		<li class="more">…and {overflow} more. Refine the search.</li>
	{/if}
	{#if filtered.length === 0}
		<li class="more">No questions match.</li>
	{/if}
</ul>

<style>
	.filters {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.search {
		padding: var(--space-2) var(--space-3);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		background: var(--bg-surface);
		color: var(--color-text);
		font-family: var(--font-body);
	}
	.row {
		display: grid;
		grid-template-columns: 1fr 1fr 1fr;
		gap: var(--space-2);
		max-width: 32rem;
	}
	.row label {
		display: flex;
		flex-direction: column;
		gap: 2px;
		font-size: calc(0.7rem * var(--font-scale));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
	}
	.row select {
		padding: var(--space-1) var(--space-2);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		background: var(--bg-surface);
		color: var(--color-text);
		font-size: calc(0.85rem * var(--font-scale));
		text-transform: none;
	}
	.ct {
		margin: 0;
		font-family: var(--font-mono);
		font-size: calc(0.75rem * var(--font-scale));
		color: var(--color-text-muted);
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.item {
		display: flex;
		flex-direction: column;
		gap: 2px;
		width: 100%;
		padding: var(--space-2) var(--space-3);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		background: var(--bg-surface);
		color: var(--color-text);
	}
	.i-head {
		display: flex;
		gap: var(--space-2);
		align-items: center;
	}
	.i-id {
		font-family: var(--font-mono);
		font-size: calc(0.75rem * var(--font-scale));
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.i-id.draft {
		color: var(--color-accent);
	}
	.badge {
		font-family: var(--font-mono);
		font-size: calc(0.65rem * var(--font-scale));
		color: var(--color-text-muted);
		background: var(--bg-primary);
		padding: 1px 6px;
		border-radius: var(--radius-full);
	}
	.badge-ref {
		background: var(--bg-primary-soft);
		color: var(--color-primary);
	}
	.i-prompt {
		font-size: calc(0.85rem * var(--font-scale));
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.i-meta {
		font-size: calc(0.75rem * var(--font-scale));
		color: var(--color-text-muted);
	}
	.more {
		text-align: center;
		color: var(--color-text-muted);
		font-size: calc(0.8rem * var(--font-scale));
		padding: var(--space-2);
	}
</style>