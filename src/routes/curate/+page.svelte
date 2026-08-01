<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';
	import ActionBar from '$lib/components/curate/ActionBar.svelte';
	import DraftsRail from '$lib/components/curate/DraftsRail.svelte';
	import QuestionEditor from '$lib/components/curate/QuestionEditor.svelte';
	import PreviewPane from '$lib/components/curate/PreviewPane.svelte';
	import QuestionPicker from '$lib/components/curate/QuestionPicker.svelte';
	import { toast } from '$lib/toast.svelte';

	let activeDraftId = $state(pool.drafts[0]!.id);
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

	function preview() {
		toast.success('Preview opened in a new tab (mock)');
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
		onNewQuestion={newQuestion}
		onValidate={validate}
		onPreview={preview}
		onSaveAll={saveAll}
	/>
	<div class="grid">
		<DraftsRail
			{activeDraftId}
			{activeCell}
			onSelectCell={(c) => (activeCell = c)}
			onSelectDraft={(id) => {
				activeDraftId = id;
				activeCell = null;
				activeQuestionId = null;
			}}
		/>
		<QuestionEditor
			{activeDraftId}
			{activeCell}
			{activeQuestionId}
			onPickQuestion={openPicker}
			onCreateNew={newQuestion}
			onDetach={detach}
		/>
		<PreviewPane {activeDraftId} {activeCell} {activeQuestionId} />
	</div>
</div>

{#if activeCell}
	<QuestionPicker
		bind:open={pickerOpen}
		{activeDraftId}
		{activeCell}
		{onPicked}
	/>
{/if}

<style>
	.page {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		gap: var(--space-2);
	}
	.grid {
		flex: 1;
		min-height: 0;
		display: grid;
		grid-template-columns: 20rem 1fr 22rem;
		gap: var(--space-2);
		padding: 0 var(--space-2) var(--space-2) var(--space-2);
	}
	@media (max-width: 1200px) {
		.grid {
			grid-template-columns: 16rem 1fr;
		}
		:global(.preview) {
			grid-column: 1 / -1;
			max-height: 30vh;
		}
	}
</style>
