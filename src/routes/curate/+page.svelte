<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';
	import { getGamemode, type ModeId } from '$lib/gamemodes';
	import ActionBar from '$lib/components/curate/ActionBar.svelte';
	import QuestionEditor from '$lib/components/curate/QuestionEditor.svelte';
	import PreviewPane from '$lib/components/curate/PreviewPane.svelte';
	import QuestionPicker from '$lib/components/curate/QuestionPicker.svelte';
	import { toast } from '$lib/toast.svelte';

	let activeDraftId = $state(pool.drafts[0]?.id ?? '');
	// Cell shape is mode-owned: grid = {categoryIdx, point}, linear = number.
	// The orchestrator only knows it as opaque state.
	let activeCell = $state<unknown>(null);
	let activeQuestionId = $state<string | null>(null);
	let pickerOpen = $state(false);

	// Load from the backend on mount. The store starts empty until the first
	// fetch resolves — the template renders the loading/error state below.
	$effect(() => {
		void pool.init();
	});

	const draft = $derived(pool.getDraft(activeDraftId));
	const mode = $derived(draft ? getGamemode(draft.board.mode) : null);
	// Dynamic component: re-evaluated when the mode changes (e.g. grid → linear).
	const BoardComponent = $derived(mode?.BoardComponent);

	function newQuestion() {
		const q = pool.createDraftQuestion();
		activeQuestionId = q.id;
		activeCell = null;
		toast.success(`Created ${q.id} — Save all to persist to disk`);
	}

	function selectDraft(id: string) {
		activeDraftId = id;
		activeCell = null;
		activeQuestionId = null;
	}

	function selectCell(c: unknown) {
		activeCell = c;
	}

	function openPicker() {
		if (activeCell === null) return;
		pickerOpen = true;
	}

	function detach() {
		if (!mode || activeCell === null) return;
		mode.detach(activeDraftId, activeCell);
		activeQuestionId = null;
		toast.success('Question detached from cell');
	}

	function onPicked(qid: string) {
		activeQuestionId = qid;
	}

	function changeMode(next: ModeId) {
		if (!draft || draft.board.mode === next) return;
		// Wipes the board on swap — confirm before clobbering any work.
		const ok = confirm(
			`Switching to ${next} will clear this draft's board (categories/questions). Continue?`
		);
		if (!ok) return;
		pool.setDraftMode(activeDraftId, next);
		activeCell = null;
		activeQuestionId = null;
		toast.success(`Mode switched to ${next}`);
	}

	function validate() {
		const d = draft;
		const m = mode;
		if (!d || !m) return;
		const issues: string[] = [];
		if (!d.title.trim()) issues.push('title');
		if (m.computeProgress(d) < 1)
			issues.push(`${Math.round((1 - m.computeProgress(d)) * 100)}% board empty`);
		if (issues.length) toast.error(`Validation: ${issues.join(', ')}`);
		else toast.success('All checks pass');
	}

	async function saveAll() {
		await pool.saveAll();
	}
</script>

<svelte:head>
	<title>Curate — Quackster</title>
</svelte:head>

<div class="page">
	<ActionBar
		{activeDraftId}
		loading={pool.loading}
		loadError={pool.loadError}
		dirtyQuestions={pool.dirtyQuestions}
		dirtyDrafts={pool.dirtyDrafts}
		saving={pool.saving}
		onModeChange={changeMode}
		onSelectDraft={selectDraft}
		onNewQuestion={newQuestion}
		onValidate={validate}
		onSaveAll={saveAll}
	/>
	{#if !pool.loaded}
		<div class="status">
			{#if pool.loadError}
				<p class="err">Backend unreachable: {pool.loadError}</p>
				<p class="hint">
					Start the API on <code>:3000</code> (<code>cargo run</code> from <code>api/</code>) or
					check your admin secret in Settings.
				</p>
			{:else}
				<p class="hint">Loading questions and games from backend…</p>
			{/if}
		</div>
	{:else if BoardComponent && draft}
		<BoardComponent {draft} {activeCell} onSelectCell={selectCell} />
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
	{/if}
</div>

{#if activeCell !== null && mode}
	<QuestionPicker bind:open={pickerOpen} {activeDraftId} {activeCell} {onPicked} />
{/if}

<style>
	.page {
		display: flex;
		flex-direction: column;
		min-height: 100%;
		gap: var(--space-2);
		padding: 0 var(--space-2) var(--space-2) var(--space-2);
	}
	.body {
		flex: 1;
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
	.status {
		flex: 1;
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-8);
		text-align: center;
	}
	.status .err {
		color: var(--color-danger);
		font-family: var(--font-heading);
	}
	.status .hint {
		color: var(--color-text-muted);
		max-width: 32rem;
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
