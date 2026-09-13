<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';
	import type { CurateDraft, GridQuizBoard } from '$lib/wire';

	let {
		draft,
		activeCell,
		onSelectCell
	}: {
		draft: CurateDraft;
		activeCell: { categoryIdx: number; point: number } | null;
		onSelectCell: (c: { categoryIdx: number; point: number } | null) => void;
	} = $props();

	const board = $derived(
		draft && draft.board.mode === 'grid_quiz' ? (draft.board as GridQuizBoard) : null
	);

	const points = $derived(
		board && board.categories[0]
			? Object.keys(board.categories[0].questions)
					.map(Number)
					.sort((a, b) => a - b)
			: []
	);
	// +1 trailing column holds the "+ Category" header cell — kept narrow.
	const cols = $derived(
		board ? `6rem repeat(${board.categories.length}, minmax(8rem, 1fr)) 5rem` : ''
	);

	function addCategory() {
		pool.addGridCategory(draft.id);
	}
	function removeCategory(name: string) {
		if (!board) return;
		const idx = board.categories.findIndex((c) => c.name === name);
		if (idx < 0) return;
		if (
			!confirm(
				`Remove category "${name}"? Any questions attached to its cells will become unreferenced.`
			)
		)
			return;
		pool.removeGridCategory(draft.id, idx);
		if (activeCell && activeCell.categoryIdx === idx) onSelectCell(null);
	}
	function addPoint() {
		pool.addGridPoint(draft.id);
	}
	function removePoint(p: number) {
		if (!confirm(`Remove the ${p} pts row? Any questions attached will become unreferenced.`))
			return;
		pool.removeGridPoint(draft.id, p);
		if (activeCell && activeCell.point === p) onSelectCell(null);
	}
</script>

<section class="canvas" aria-label="Board">
	{#if board && board.categories.length > 0}
		<div class="board" style:grid-template-columns={cols}>
			<!-- Header row: corner + categories + add-category -->
			<div class="brow head">
				<div></div>
				{#each board.categories as cat (cat.name)}
					<div class="bcell head cat">
						<span class="cat-name">{cat.name}</span>
						<button
							class="rm"
							title="Remove category"
							aria-label="Remove category {cat.name}"
							onclick={() => {
								removeCategory(cat.name);
							}}>✕</button
						>
					</div>
				{/each}
				<button class="bcell head add" onclick={addCategory}>+ Category</button>
			</div>

			<!-- Data rows: row-head + N cells + 1 trailing spacer so each row fills all 8 columns -->
			{#each points as p (p)}
				<div class="brow">
					<div class="bcell head">
						<span>{p} pts</span>
						<button
							class="rm"
							title="Remove row"
							aria-label="Remove {p} pts row"
							onclick={() => {
								removePoint(p);
							}}>✕</button
						>
					</div>
					{#each board.categories as cat, ci (cat.name)}
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
					<!-- ponytail: spacer keeps the next row's row-head from wrapping into this row's +Cat slot -->
					<div class="bcell spacer"></div>
				</div>
			{/each}

			<!-- Add-point row: add button in the row-head slot + N spacers + 1 trailing spacer -->
			<div class="brow">
				<button class="bcell head add" onclick={addPoint}>+ Point row</button>
				{#each board.categories as _cat (_cat.name)}
					<div class="bcell spacer"></div>
				{/each}
				<div class="bcell spacer"></div>
			</div>
		</div>
	{:else if board}
		<div class="placeholder">
			<p>Empty grid. Add a category to get started.</p>
			<button class="add-btn" onclick={addCategory}>+ Add first category</button>
		</div>
	{/if}
</section>

<style>
	.canvas {
		display: flex;
		flex-direction: column;
		flex: 0 0 auto;
		min-height: 0;
		background: var(--bg-surface);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		overflow: hidden;
	}
	.board {
		display: grid;
		gap: 2px;
		background: var(--border-color);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		overflow-x: auto;
		overflow-y: hidden;
		min-width: 0;
	}
	.brow {
		display: contents;
	}
	.bcell {
		position: relative;
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
		padding-right: 1.5rem;
	}
	.bcell.add {
		color: var(--color-text-muted);
		font-style: italic;
		background: transparent;
		border: 1px dashed var(--border-color);
		cursor: pointer;
	}
	.bcell.add:hover {
		background: var(--bg-primary);
		color: var(--color-primary);
		border-color: var(--color-primary);
	}
	.bcell.spacer {
		background: transparent;
		min-height: 2.5rem;
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
	.cat-name {
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 100%;
	}
	.rm {
		position: absolute;
		top: 2px;
		right: 2px;
		width: 1.25rem;
		height: 1.25rem;
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--color-text-muted);
		font-size: calc(0.65rem * var(--font-scale));
		line-height: 1;
		cursor: pointer;
		opacity: 0;
		transition: opacity 100ms;
	}
	.bcell:hover .rm {
		opacity: 1;
	}
	.rm:hover {
		background: var(--color-danger);
		color: var(--color-text-inverse);
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
	.placeholder {
		padding: var(--space-6);
		color: var(--color-text-muted);
		text-align: center;
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		align-items: center;
	}
	.placeholder p {
		margin: 0;
	}
	.add-btn {
		padding: var(--space-2) var(--space-4);
		border: 1px dashed var(--border-color);
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--color-primary);
		font-family: var(--font-body);
		font-size: calc(0.85rem * var(--font-scale));
		cursor: pointer;
	}
	.add-btn:hover {
		background: var(--bg-primary);
		border-style: solid;
	}
</style>
