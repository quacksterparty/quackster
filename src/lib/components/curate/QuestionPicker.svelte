<script lang="ts">
	import Drawer from '$lib/components/Drawer.svelte';
	import QuestionList from '$lib/components/QuestionList.svelte';
	import Button from '$lib/components/Button.svelte';
	import { pool } from '$lib/data/pool.svelte';
	import type { PoolQuestion } from '$lib/data/seed';

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
	</div>
	<QuestionList
		bind:query
		bind:kindFilter
		bind:statusFilter
		bind:langFilter
		max={80}
		actions={pickAction}
	/>
</Drawer>

{#snippet pickAction(q: PoolQuestion)}
	<button class="pick" onclick={() => pick(q.id)}>Pick</button>
{/snippet}

<style>
	.head {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-2);
	}
	.pick {
		align-self: flex-start;
		background: transparent;
		border: var(--border-width) var(--border-style) var(--color-primary);
		border-radius: var(--radius-sm);
		padding: 2px var(--space-2);
		font-family: var(--font-body);
		font-size: calc(0.75rem * var(--font-scale));
		color: var(--color-primary);
		cursor: pointer;
	}
	.pick:hover {
		background: var(--color-primary);
		color: var(--color-text-inverse);
	}
</style>
