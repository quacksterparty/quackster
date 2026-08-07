<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';
	import type { CurateDraft, LinearBoard as LinearBoardShape } from '$lib/data/seed';
	import Button from '../Button.svelte';

	let {
		draft,
		activeCell,
		onSelectCell
	}: {
		draft: CurateDraft;
		activeCell: number | null;
		onSelectCell: (cell: number | null) => void;
	} = $props();

	// Narrow once so the template can use `board.items` without repeated checks.
	const board = $derived(
		draft.board.mode === 'linear' ? (draft.board as LinearBoardShape) : null
	);

	let listEl: HTMLOListElement | null = $state(null);

	// Keep the newly selected slot in view — "+ Add question" appends, so without
	// this the user lands at the top with the new slot offscreen. `nearest` is a
	// no-op when the cell is already visible, so it's safe to fire on any select.
	$effect(() => {
		if (activeCell === null || !listEl) return;
		listEl.querySelector(`[data-idx="${activeCell}"]`)?.scrollIntoView({ block: 'nearest' });
	});

	function addSlot() {
		const idx = pool.addLinearSlot(draft.id);
		if (idx !== null) onSelectCell(idx);
	}

	function move(from: number, dir: -1 | 1) {
		const to = from + dir;
		pool.moveLinearSlot(draft.id, from, to);
		// Keep selection on the moved item so it stays visible to the user.
		if (activeCell === from) onSelectCell(to);
	}

	function remove(idx: number) {
		pool.removeLinearSlot(draft.id, idx);
		if (activeCell === idx) onSelectCell(null);
	}
</script>

<section class="board" aria-label="Linear board">
	{#if !board || board.items.length === 0}
		<div class="empty">
			<p>No questions yet. Add one to start the sequence.</p>
			<Button onclick={addSlot}>+ Add question</Button>
		</div>
	{:else}
		<ol class="slots" bind:this={listEl}>
			{#each board.items as slot, i (i)}
				{@const q = slot ? pool.getQuestion(slot.questionId) : null}
				<li
					class="slot"
					class:selected={activeCell === i}
					class:filled={!!slot}
					data-idx={i}
				>
					<button
						class="slot-main"
						onclick={() => {
							onSelectCell(i);
						}}
						title={q ? `${q.id} · ${q.prompt}` : 'Empty — pick or create a question'}
					>
						<span class="num">{i + 1}</span>
						<span class="body">
							{#if q}
								<code class="qid">{q.id}</code>
								<span class="prompt">{q.prompt}</span>
							{:else}
								<span class="placeholder">+ empty slot</span>
							{/if}
						</span>
					</button>
					<div class="slot-actions">
						<button
							class="icon-btn"
							disabled={i === 0}
							title="Move up"
							aria-label="Move up"
							onclick={() => {
								move(i, -1);
							}}>↑</button
						>
						<button
							class="icon-btn"
							disabled={i === board.items.length - 1}
							title="Move down"
							aria-label="Move down"
							onclick={() => {
								move(i, 1);
							}}>↓</button
						>
						<button
							class="icon-btn danger"
							title="Remove slot"
							aria-label="Remove slot"
							onclick={() => {
								remove(i);
							}}>✕</button
						>
					</div>
				</li>
			{/each}
		</ol>
		<div class="foot">
			<Button onclick={addSlot}>+ Add question</Button>
		</div>
	{/if}
</section>

<style>
	.board {
		display: flex;
		flex-direction: column;
		flex: 0 0 auto;
		min-height: 0;
		/* ponytail: cap at half the viewport; inner list scrolls when slots overflow */
		max-height: 40vh;
		background: var(--bg-surface);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		overflow: hidden;
	}
	.empty {
		padding: var(--space-6);
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		align-items: center;
		color: var(--color-text-muted);
		text-align: center;
	}
	.empty p {
		margin: 0;
	}
	.slots {
		list-style: none;
		margin: 0;
		padding: var(--space-2);
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
	}
	.slot {
		display: flex;
		align-items: stretch;
		gap: var(--space-2);
		padding: var(--space-1) var(--space-2);
		border: var(--border-width) var(--border-style) transparent;
		border-radius: var(--radius-sm);
		background: var(--bg-primary);
	}
	.slot.selected {
		border-color: var(--color-primary);
	}
	.slot-main {
		flex: 1;
		display: flex;
		gap: var(--space-3);
		align-items: center;
		text-align: left;
		background: transparent;
		border: none;
		color: var(--color-text);
		font-family: inherit;
		cursor: pointer;
		padding: var(--space-1) var(--space-2);
		min-width: 0;
	}
	.num {
		flex: 0 0 2rem;
		font-family: var(--font-mono);
		font-weight: 700;
		color: var(--color-text-muted);
		text-align: center;
	}
	.body {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
		flex: 1;
	}
	.qid {
		font-family: var(--font-mono);
		font-size: calc(0.65rem * var(--font-scale));
		color: var(--color-text-muted);
	}
	.prompt {
		font-size: calc(0.85rem * var(--font-scale));
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.placeholder {
		color: var(--color-text-muted);
		font-style: italic;
	}
	.slot.filled {
		background: color-mix(in srgb, var(--color-primary) 5%, var(--bg-primary));
	}
	.slot-actions {
		display: flex;
		gap: 2px;
		align-items: center;
	}
	.icon-btn {
		width: 1.75rem;
		height: 1.75rem;
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		background: var(--bg-surface);
		color: var(--color-text);
		font-family: var(--font-body);
		cursor: pointer;
		font-size: calc(0.85rem * var(--font-scale));
	}
	.icon-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.icon-btn:hover:not(:disabled) {
		background: var(--color-primary);
		color: var(--color-text-inverse);
	}
	.icon-btn.danger:hover {
		background: var(--color-danger);
		border-color: var(--color-danger);
		color: var(--color-text-inverse);
	}
	.foot {
		padding: var(--space-3);
		border-top: var(--border-width) var(--border-style) var(--border-color);
	}
</style>
