<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';
	import Drawer from '../Drawer.svelte';
	import Button from '../Button.svelte';
	import { isDraftId } from '$lib/data/ids';

	let {
		open = $bindable(false),
		activeDraftId,
		activeCell,
		onPicked
	}: {
		open: boolean;
		activeDraftId: string;
		activeCell: { categoryIdx: number; point: number };
		onPicked: (qid: string) => void;
	} = $props();

	let query = $state('');
	let kindFilter = $state<'all' | 'text' | 'numeric' | 'order'>('all');
	let statusFilter = $state<'all' | 'named' | 'draft' | 'referenced'>('all');
	let langFilter = $state<'all' | 'de' | 'en'>('all');

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

	function pick(qid: string) {
		pool.attachQuestion(activeDraftId, activeCell.categoryIdx, activeCell.point, qid);
		onPicked(qid);
		open = false;
	}

	function createAndAttach() {
		const q = pool.createDraftQuestion();
		pool.attachQuestion(activeDraftId, activeCell.categoryIdx, activeCell.point, q.id);
		onPicked(q.id);
		open = false;
	}
</script>

<Drawer bind:open title="Pick a question">
	<div class="head">
		<Button variant="primary" onclick={createAndAttach}>+ Create new question</Button>
		<span class="ct">{filtered.length} of {pool.questions.length} questions</span>
	</div>
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
	</div>
	<ul class="list">
		{#each filtered.slice(0, 80) as q (q.id)}
			<li>
				<button class="item" onclick={() => pick(q.id)}>
					<div class="i-head">
						<code class="i-id" class:draft={isDraftId(q.id)}>{q.id}</code>
						<span class="i-kind">{q.kind}</span>
						<span class="i-lang">{q.defaultLang}</span>
					</div>
					<div class="i-prompt">{q.prompt || '(empty prompt)'}</div>
					<div class="i-meta">→ {q.answer || '(no answer)'}</div>
				</button>
			</li>
		{/each}
		{#if filtered.length > 80}
			<li class="more">…and {filtered.length - 80} more. Refine the search.</li>
		{/if}
		{#if filtered.length === 0}
			<li class="more">No questions match. Create a new one above.</li>
		{/if}
	</ul>
</Drawer>

<style>
	.head {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-2);
	}
	.ct {
		font-size: calc(0.75rem * var(--font-scale));
		color: var(--color-text-muted);
		font-family: var(--font-mono);
	}
	.filters {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.search {
		padding: var(--space-2) var(--space-3);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		background: var(--bg-primary);
		color: var(--color-text);
		font-family: var(--font-body);
	}
	.row {
		display: grid;
		grid-template-columns: 1fr 1fr 1fr;
		gap: var(--space-2);
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
		background: var(--bg-primary);
		color: var(--color-text);
		font-size: calc(0.85rem * var(--font-scale));
		text-transform: none;
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		flex: 1;
		overflow-y: auto;
		min-height: 0;
	}
	.item {
		display: flex;
		flex-direction: column;
		gap: 2px;
		width: 100%;
		text-align: left;
		padding: var(--space-2) var(--space-3);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		background: var(--bg-primary);
		cursor: pointer;
		font-family: inherit;
		color: var(--color-text);
	}
	.item:hover {
		border-color: var(--color-primary);
	}
	.i-head {
		display: flex;
		gap: var(--space-2);
		align-items: center;
	}
	.i-id {
		font-family: var(--font-mono);
		font-size: calc(0.75rem * var(--font-scale));
		color: var(--color-text);
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.i-id.draft {
		color: var(--color-accent);
	}
	.i-kind,
	.i-lang {
		font-family: var(--font-mono);
		font-size: calc(0.65rem * var(--font-scale));
		color: var(--color-text-muted);
		background: var(--bg-surface);
		padding: 1px 6px;
		border-radius: var(--radius-full);
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
