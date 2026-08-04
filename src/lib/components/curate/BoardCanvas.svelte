<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';

	let {
		activeDraftId,
		activeCell,
		onSelectCell
	}: {
		activeDraftId: string;
		activeCell: { categoryIdx: number; point: number } | null;
		onSelectCell: (c: { categoryIdx: number; point: number } | null) => void;
	} = $props();

	const draft = $derived(pool.getDraft(activeDraftId));
	// Point values come from the data — fix for the hardcoded-5-col bug.
	// Length adapts to board.points.length; minmax keeps cells readable.
	const points = $derived(
		draft
			? Object.keys(draft.board.categories[0]?.questions ?? {})
					.map(Number)
					.sort((a, b) => a - b)
			: []
	);
	const cols = $derived(`6rem repeat(${points.length}, minmax(8rem, 1fr))`);
</script>

<section class="canvas" aria-label="Board">
	{#if draft}
		<div class="board" style:grid-template-columns={cols}>
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
							class:complete={q ? q.answer : false}
							class:selected={activeCell !== null &&
								activeCell.categoryIdx === ci &&
								activeCell.point === p}
							onclick={() => {
								onSelectCell({ categoryIdx: ci, point: p });
							}}
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
	{/if}
</section>

<style>
	.canvas {
		display: flex;
		flex-direction: column;
		/* keep our content size in the page flex column — don't shrink, don't grow */
		flex: 0 0 auto;
		min-height: 0;
		background: var(--bg-surface);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		overflow: hidden;
	}
	.board {
		display: grid;
		/* column count injected from data via the `grid-template-columns` style */
		gap: 2px;
		background: var(--border-color);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		overflow-x: auto;
		overflow-y: hidden;
		min-width: 0;
	}
	.brow {
		/* virtual row — children flow into the parent grid via auto-flow */
		display: contents;
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
