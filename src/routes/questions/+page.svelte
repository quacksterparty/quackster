<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';
	import { isDraftId } from '$lib/data/ids';
	import Button from '$lib/components/Button.svelte';
	import { resolve } from '$app/paths';
	import { goto } from '$app/navigation';
	import { formatRelative } from '$lib/data/seed';

	let query = $state('');
	let kindFilter = $state<'all' | 'text' | 'numeric' | 'order'>('all');
	let statusFilter = $state<'all' | 'named' | 'draft' | 'referenced'>('all');
	let langFilter = $state<'all' | 'de' | 'en'>('all');
	let selectedId = $state<string | null>(null);

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

	<div class="grid">
		<ul class="list">
			{#each filtered as q (q.id)}
				<li>
					<button
						class="item"
						class:active={selectedId === q.id}
						onclick={() => (selectedId = q.id)}
					>
						<div class="i-head">
							<code class="i-id" class:draft={isDraftId(q.id)}>{q.id}</code>
							<span class="i-kind">{q.kind}</span>
							<span class="i-lang">{q.defaultLang}</span>
							{#if q.status === 'referenced'}
								<span class="badge-ref">ref ×{pool.referenceLocations(q.id).length}</span>
							{/if}
						</div>
						<div class="i-prompt">{q.prompt || '(empty prompt)'}</div>
						<div class="i-meta">→ {q.answer || '(no answer)'}</div>
					</button>
				</li>
			{/each}
			{#if filtered.length === 0}
				<li class="empty">No questions match.</li>
			{/if}
		</ul>

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
					<dt>Prompt</dt><dd>{selected.prompt || '(empty)'}</dd>
					<dt>Answer</dt><dd>{selected.answer || '(none)'}</dd>
					<dt>Kind</dt><dd>{selected.kind}</dd>
					<dt>Lang</dt><dd>{selected.defaultLang}</dd>
					<dt>License</dt><dd>{selected.license}</dd>
					<dt>Tags</dt><dd>{selected.tags.length ? selected.tags.join(', ') : '(none)'}</dd>
					<dt>Variants</dt><dd>{selected.variants.join(', ') || '(none)'}</dd>
					<dt>Status</dt><dd>{selected.status}</dd>
					<dt>Created</dt><dd>{formatRelative(selected.createdAt)}</dd>
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
	.grid {
		flex: 1;
		min-height: 0;
		display: grid;
		grid-template-columns: 1.2fr 1fr;
		gap: var(--space-3);
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
		background: var(--bg-surface);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		padding: var(--space-2);
		overflow-y: auto;
	}
	.item {
		display: flex;
		flex-direction: column;
		gap: 2px;
		width: 100%;
		text-align: left;
		padding: var(--space-2) var(--space-3);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		cursor: pointer;
		font-family: inherit;
		color: var(--color-text);
	}
	.item:hover,
	.item.active {
		background: var(--bg-primary);
	}
	.item.active {
		outline: 2px solid var(--color-primary);
		outline-offset: -2px;
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
	.i-kind,
	.i-lang,
	.badge-ref {
		font-family: var(--font-mono);
		font-size: calc(0.65rem * var(--font-scale));
		color: var(--color-text-muted);
		background: var(--bg-primary);
		padding: 1px 6px;
		border-radius: var(--radius-full);
	}
	.badge-ref {
		background: color-mix(in srgb, var(--color-primary) 18%, transparent);
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
