<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';
	import { isDraftId } from '$lib/data/ids';
	import { formatRelative, type PoolQuestion } from '$lib/data/seed';
	import QuestionList from '$lib/components/QuestionList.svelte';
	import Button from '$lib/components/Button.svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';

	let query = $state('');
	let kindFilter = $state<'all' | 'text' | 'numeric' | 'order'>('all');
	let statusFilter = $state<'all' | 'named' | 'draft' | 'referenced'>('all');
	let langFilter = $state<'all' | 'de' | 'en'>('all');
	let selectedId = $state<string | null>(null);

	const selected = $derived(selectedId ? pool.getQuestion(selectedId) : null);
	const selectedRefs = $derived(selected ? pool.referenceLocations(selected.id) : []);

	function editInCurate() {
		void goto(resolve('/curate', {}));
	}
</script>

<svelte:head>
	<title>Questions — Quackster</title>
</svelte:head>

<div class="page">
	<header class="head">
		<div>
			<h1>Question library</h1>
			<p class="ct">Read-only browse. Edit in <a href={resolve('/curate', {})}>curate</a>.</p>
		</div>
	</header>

	<div class="grid">
		<div class="list-wrap">
			<QuestionList
				bind:query
				bind:kindFilter
				bind:statusFilter
				bind:langFilter
				showRefs
				actions={selectedBadge}
			/>
		</div>

		<aside class="detail">
			{#if !selected}
				<p class="empty">Select a question to see details.</p>
			{:else}
				<header class="d-head">
					<div>
						<div class="kicker">Question</div>
						<code class="d-id" class:draft={isDraftId(selected.id)}>{selected.id}</code>
					</div>
					<Button size="sm" onclick={editInCurate}>Edit in curate →</Button>
				</header>
				<dl>
					<dt>Prompt</dt>
					<dd>{selected.prompt || '(empty)'}</dd>
					<dt>Answer</dt>
					<dd>{selected.answer || '(none)'}</dd>
					<dt>Kind</dt>
					<dd>{selected.kind}</dd>
					<dt>Lang</dt>
					<dd>{selected.defaultLang}</dd>
					<dt>License</dt>
					<dd>{selected.license}</dd>
					<dt>Tags</dt>
					<dd>{selected.tags.length ? selected.tags.join(', ') : '(none)'}</dd>
					<dt>Variants</dt>
					<dd>{selected.variants.join(', ') || '(none)'}</dd>
					<dt>Status</dt>
					<dd>{selected.status}</dd>
					<dt>Created</dt>
					<dd>{formatRelative(selected.createdAt)}</dd>
					{#if selected.previousIds?.length}
						<dt>Renamed from</dt>
						<dd>
							{#each selected.previousIds as p (p)}
								<code class="prev">{p}</code>
							{/each}
						</dd>
					{/if}
				</dl>
				{#if selected.choices}
					<section>
						<h3>Choices</h3>
						<ul class="choices">
							{#each selected.choices as c (c.id)}
								<li class:correct={c.correct}>{c.text || '—'}</li>
							{/each}
						</ul>
					</section>
				{/if}
				{#if selected.explanation}
					<section>
						<h3>Explanation</h3>
						<p>{selected.explanation}</p>
					</section>
				{/if}
				<section>
					<h3>Referenced by</h3>
					{#if selectedRefs.length === 0}
						<p class="muted">Not yet referenced.</p>
					{:else}
						<ul class="refs">
							{#each selectedRefs as r (r.draft.id + r.category + r.point)}
								<li><strong>{r.draft.title}</strong> · {r.category} · {r.point} pts</li>
							{/each}
						</ul>
					{/if}
				</section>
			{/if}
		</aside>
	</div>
</div>

{#snippet selectedBadge(q: PoolQuestion)}
	<button
		class="select-btn"
		class:active={selectedId === q.id}
		onclick={() => (selectedId = q.id)}
		aria-pressed={selectedId === q.id}>View →</button
	>
{/snippet}

<style>
	.page {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		height: 100%;
		min-height: 0;
		padding: var(--space-4);
		box-sizing: border-box;
	}
	.head h1 {
		font-family: var(--font-heading);
		margin: 0;
	}
	.head .ct {
		color: var(--color-text-muted);
		margin: 0;
	}
	.head a {
		color: var(--color-primary);
	}
	.grid {
		flex: 1;
		min-height: 0;
		display: grid;
		grid-template-columns: 1.2fr 1fr;
		gap: var(--space-3);
	}
	.list-wrap {
		background: var(--bg-surface);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		padding: var(--space-2);
		overflow-y: auto;
	}
	.select-btn {
		align-self: flex-start;
		background: transparent;
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		padding: 2px var(--space-2);
		font-family: var(--font-body);
		font-size: calc(0.75rem * var(--font-scale));
		color: var(--color-text-muted);
		cursor: pointer;
	}
	.select-btn.active {
		background: var(--color-primary);
		border-color: var(--color-primary);
		color: var(--color-text-inverse);
	}
	.detail {
		background: var(--bg-surface);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		padding: var(--space-4);
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.d-head {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
	}
	.kicker {
		font-size: calc(0.7rem * var(--font-scale));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
	}
	.d-id {
		font-family: var(--font-mono);
		font-size: calc(0.9rem * var(--font-scale));
		display: inline-block;
		margin-top: 2px;
	}
	.d-id.draft {
		color: var(--color-accent);
	}
	dl {
		display: grid;
		grid-template-columns: 6rem 1fr;
		gap: var(--space-1) var(--space-3);
		margin: 0;
		font-size: calc(0.85rem * var(--font-scale));
	}
	dt {
		color: var(--color-text-muted);
	}
	dd {
		margin: 0;
	}
	.prev {
		font-family: var(--font-mono);
		font-size: calc(0.75rem * var(--font-scale));
		background: var(--bg-primary);
		padding: 1px var(--space-2);
		border-radius: var(--radius-sm);
		margin-right: var(--space-1);
	}
	.choices,
	.refs {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.choices li {
		padding: var(--space-1) var(--space-2);
		background: var(--bg-primary);
		border-radius: var(--radius-sm);
	}
	.choices li.correct {
		border-left: 3px solid var(--color-success);
	}
	section h3 {
		margin: 0 0 var(--space-1) 0;
		font-family: var(--font-body);
		font-size: calc(0.7rem * var(--font-scale));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
	}
	.muted {
		color: var(--color-text-muted);
		font-size: calc(0.85rem * var(--font-scale));
		margin: 0;
	}
	.empty {
		text-align: center;
		color: var(--color-text-muted);
		padding: var(--space-6);
	}
</style>
