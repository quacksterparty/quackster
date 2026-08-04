<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';
	import ActionBar from '$lib/components/curate/ActionBar.svelte';
	import BoardCanvas from '$lib/components/curate/BoardCanvas.svelte';
	import QuestionEditor from '$lib/components/curate/QuestionEditor.svelte';
	import PreviewPane from '$lib/components/curate/PreviewPane.svelte';
	import QuestionPicker from '$lib/components/curate/QuestionPicker.svelte';
	import { toast } from '$lib/toast.svelte';

	let activeDraftId = $state(pool.drafts[0]?.id ?? '');
	let activeCell = $state<{ categoryIdx: number; point: number } | null>(null);
	let activeQuestionId = $state<string | null>(null);
	let pickerOpen = $state(false);

	const draft = $derived(pool.getDraft(activeDraftId));

	function newQuestion() {
		const q = pool.createDraftQuestion();
		activeQuestionId = q.id;
		activeCell = null;
		toast.success(`Created ${q.id} — fill in the prompt to auto-rename`);
	}

	function selectDraft(id: string) {
		activeDraftId = id;
		activeCell = null;
		activeQuestionId = null;
	}

	function selectCell(c: { categoryIdx: number; point: number } | null) {
		activeCell = c;
	}

	function openPicker() {
		if (!activeCell) return;
		pickerOpen = true;
	}

	function detach() {
		if (!activeCell) return;
		pool.detachQuestion(activeDraftId, activeCell.categoryIdx, activeCell.point);
		activeQuestionId = null;
		toast.success('Question detached from cell');
	}

	function onPicked(qid: string) {
		activeQuestionId = qid;
	}

	function validate() {
		const d = draft;
		if (!d) return;
		const issues: string[] = [];
		if (!d.title.trim()) issues.push('title');
		if (d.progress < 1) issues.push(`${Math.round((1 - d.progress) * 100)}% board empty`);
		if (issues.length) toast.error(`Validation: ${issues.join(', ')}`);
		else toast.success('All checks pass');
	}

	function saveAll() {
		const d = draft;
		if (!d) return;
		pool.updateDraft(d.id, { status: 'saved', updated: new Date().toISOString() });
		toast.success('Saved (mock — backend save is a separate task)');
	}
</script>

<svelte:head>
	<title>Curate — Quackster</title>
</svelte:head>

<div class="page">
	<ActionBar
		{activeDraftId}
		onSelectDraft={selectDraft}
		onNewQuestion={newQuestion}
		onValidate={validate}
		onSaveAll={saveAll}
	/>
	<BoardCanvas {activeDraftId} {activeCell} onSelectCell={selectCell} />
	<div class="body">
		<QuestionEditor
			{activeDraftId}
			{activeCell}
			{activeQuestionId}
			onPickQuestion={openPicker}
			onCreateNew={newQuestion}
			onDetach={detach}
		/>
		<aside class="preview-side" aria-label="Preview and validation">
			<PreviewPane {activeDraftId} {activeCell} {activeQuestionId} />
		</aside>
	</div>
</div>

{#if activeCell}
	<QuestionPicker bind:open={pickerOpen} {activeDraftId} {activeCell} {onPicked} />
{/if}

<style>
	.page {
		display: flex;
		flex-direction: column;
		/* at least viewport, can grow so the page scrolls instead of clipping the canvas */
		min-height: 100%;
		gap: var(--space-2);
		padding: 0 var(--space-2) var(--space-2) var(--space-2);
	}
	.body {
		flex: 1;
		/* keep the editor usable even when the canvas is huge -> page scrolls */
		min-height: 30rem;
		display: flex;
		gap: var(--space-2);
	}
	.preview-side {
		flex: 0 0 22rem;
		min-height: 0;
		background: var(--bg-surface);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		overflow-y: auto;
	}
	@media (max-width: 900px) {
		.body {
			flex-direction: column;
		}
		.preview-side {
			flex: 0 0 auto;
			max-height: 30vh;
		}
	}
</style>
