<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';

	let {
		activeDraftId,
		activeCell,
		onSelectCell,
		onSelectDraft
	}: {
		activeDraftId: string;
		activeCell: { categoryIdx: number; point: number } | null;
		onSelectCell: (c: { categoryIdx: number; point: number } | null) => void;
		onSelectDraft: (id: string) => void;
	} = $props();

	const draft = $derived(pool.getDraft(activeDraftId));
	const points = $derived(
		draft
			? Object.keys(draft.board.categories[0]?.questions ?? {})
					.map(Number)
					.sort((a, b) => a - b)
			: []
	);
</script>

<aside class="rail" aria-label="Drafts and board">
	<section class="section">
		<header class="head">
			<h2>Drafts</h2>
			<button class="new" title="New draft">+</button>
		</header>
		<ul class="drafts">
			{#each pool.drafts as d (d.id)}
				<li>
					<button
						class="draft"
						class:active={d.id === activeDraftId}
						onclick={() => onSelectDraft(d.id)}
					>
						<div class="d-title">{d.title}</div>
						<div class="d-meta">
							<span class="bar" style:--p={d.progress}></span>
							<span class="d-pct">{Math.round(d.progress * 100)}%</span>
							<span class="badge badge-{d.status}">{d.status.replace('_', ' ')}</span>
							{#if d.do_not_delete}<span class="badge-protected" title="do_not_delete">🛡</span>{/if}
						</div>
					</button>
				</li>
			{/each}
		</ul>
	</section>

	{#if draft}
		<section class="section grow">
			<header class="head">
				<h2>Board</h2>
				<span class="ct">{Math.round(draft.progress * 100)}% filled</span>
			</header>
			<div class="board">
				<div class="brow head">
					<div></div>
					{#each points as p (p)}
						<div class="bcell head">{p}</div>
					{/each}
				</div>
				{#each draft.board.categories as cat, ci (cat.name)}
					<div class="brow">
						<div class="bcell head cat">{cat.name}</div>
						{#each points as p (p)}
							{@const cell = cat.questions[p]}
							{@const q = cell ? pool.getQuestion(cell.questionId) : null}
							<button
								class="bcell"
								class:filled={!!cell}
								class:complete={q?.answer}
								class:selected={activeCell?.categoryIdx === ci && activeCell?.point === p}
								onclick={() => onSelectCell({ categoryIdx: ci, point: p })}
								title={q ? `${q.id} · ${q.prompt}` : 'Empty — pick or create a question'}
							>
								{#if q}
									<span class="qid">{q.id}</span>
									<span class="qp">{q.prompt}</span>
								{:else}
									<span class="empty">+</span>
								{/if}
							</button>
						{/each}
					</div>
				{/each}
			</div>
		</section>
	{/if}
</aside>

<style>
	.rail {
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: var(--bg-surface);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		overflow: hidden;
	}
	.section {
		padding: var(--space-3);
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		border-bottom: var(--border-width) var(--border-style) var(--border-color);
	}
	.section.grow {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
	}
	.head {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
	}
	.head h2 {
		margin: 0;
		font-family: var(--font-heading);
		font-size: calc(0.95rem * var(--font-scale));
	}
	.new {
		background: var(--color-primary);
		color: var(--color-text-inverse);
		border: none;
		width: 1.5rem;
		height: 1.5rem;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 1rem;
	}
	.ct {
		font-size: calc(0.7rem * var(--font-scale));
		color: var(--color-text-muted);
		font-family: var(--font-mono);
	}
	.drafts {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.draft {
		text-align: left;
		width: 100%;
		padding: var(--space-2);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		background: var(--bg-primary);
		cursor: pointer;
		color: var(--color-text);
		font-family: inherit;
	}
	.draft.active {
		border-color: var(--color-primary);
	}
	.d-title {
		font-weight: 600;
		font-size: calc(0.85rem * var(--font-scale));
	}
	.d-meta {
		display: flex;
		gap: var(--space-2);
		align-items: center;
		font-size: calc(0.7rem * var(--font-scale));
		color: var(--color-text-muted);
		margin-top: 4px;
	}
	.bar {
		width: 3rem;
		height: 4px;
		background: var(--border-color);
		border-radius: var(--radius-full);
		overflow: hidden;
		position: relative;
	}
	.bar::before {
		content: '';
		position: absolute;
		inset: 0;
		width: calc(var(--p) * 100%);
		background: var(--color-primary);
	}
	.d-pct {
		font-family: var(--font-mono);
	}
	[class*='badge-'] {
		padding: 1px 6px;
		border-radius: var(--radius-full);
		font-size: calc(0.65rem * var(--font-scale));
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.badge-saved {
		background: color-mix(in srgb, var(--color-success) 20%, transparent);
		color: var(--color-success);
	}
	.badge-unsaved_changes {
		background: color-mix(in srgb, var(--color-accent) 25%, transparent);
		color: var(--color-accent);
	}
	.badge-incomplete {
		background: var(--bg-surface);
		color: var(--color-text-muted);
	}
	.badge-invalid {
		background: color-mix(in srgb, var(--color-danger) 20%, transparent);
		color: var(--color-danger);
	}
	.badge-protected {
		font-size: calc(0.7rem * var(--font-scale));
	}
	.board {
		display: flex;
		flex-direction: column;
		gap: 2px;
		background: var(--border-color);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		overflow: hidden;
	}
	.brow {
		display: grid;
		grid-template-columns: 5rem repeat(5, 1fr);
		gap: 2px;
	}
	.bcell {
		padding: var(--space-2);
		background: var(--bg-primary);
		text-align: left;
		font-size: calc(0.7rem * var(--font-scale));
		color: var(--color-text);
		border: none;
		cursor: pointer;
		font-family: inherit;
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-height: 3.5rem;
		overflow: hidden;
	}
	.bcell.head {
		background: var(--bg-surface);
		font-weight: 600;
		text-align: center;
		align-items: center;
		justify-content: center;
		cursor: default;
	}
	.bcell.head.cat {
		text-align: left;
		align-items: flex-start;
	}
	.bcell.filled {
		background: color-mix(in srgb, var(--color-primary) 5%, var(--bg-primary));
	}
	.bcell.complete {
		background: color-mix(in srgb, var(--color-success) 8%, var(--bg-primary));
	}
	.bcell.selected {
		outline: 2px solid var(--color-primary);
		outline-offset: -2px;
	}
	.qid {
		font-family: var(--font-mono);
		color: var(--color-text-muted);
		font-size: calc(0.65rem * var(--font-scale));
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.qp {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.empty {
		color: var(--color-text-muted);
		font-size: 1.2rem;
		text-align: center;
	}
</style>
