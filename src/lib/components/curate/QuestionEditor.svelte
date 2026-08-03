<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';
	import { isDraftId } from '$lib/data/ids';
	import Button from '../Button.svelte';
	import QuestionMetaEditor from './QuestionMetaEditor.svelte';
	import QuestionNumericVariants from './QuestionNumericVariants.svelte';
	import QuestionChoicesEditor from './QuestionChoicesEditor.svelte';
	import QuestionReferences from './QuestionReferences.svelte';

	let {
		activeDraftId,
		activeCell,
		activeQuestionId,
		onPickQuestion,
		onCreateNew,
		onDetach
	}: {
		activeDraftId: string;
		activeCell: { categoryIdx: number; point: number } | null;
		activeQuestionId: string | null;
		onPickQuestion: () => void;
		onCreateNew: () => void;
		onDetach: () => void;
	} = $props();

	const draft = $derived(pool.getDraft(activeDraftId));
	const cell = $derived(
		activeCell && draft
			? (draft.board.categories[activeCell.categoryIdx]?.questions[activeCell.point] ?? null)
			: null
	);
	const question = $derived(
		cell
			? pool.getQuestion(cell.questionId)
			: activeQuestionId
				? pool.getQuestion(activeQuestionId)
				: null
	);
</script>

<section class="editor">
	<header class="ed-head">
		<div class="h-left">
			<div class="kicker">Question</div>
			{#if question && activeCell && cell?.questionId === question.id && draft}
				{@const cat = draft.board.categories[activeCell.categoryIdx]?.name ?? ''}
				<div class="attach-row">
					<span class="attach-info">
						Attached to <strong>{cat}</strong> · {activeCell.point} pts
					</span>
					<span class="sep">·</span>
					<button class="link" onclick={onPickQuestion}>Replace</button>
					<span class="sep">·</span>
					<button class="link link-danger" onclick={onDetach}>Detach</button>
				</div>
			{/if}
			{#if question}
				<div class="id-row">
					<code class="qid" class:draft={isDraftId(question.id)}>{question.id}</code>
					{#if isDraftId(question.id)}
						<span
							class="badge-draft"
							title="Auto-renames to q_&lt;slug&gt; when prompt has text and the question is unreferenced"
						>
							draft id · will rename
						</span>
					{/if}
					{#if question.previousIds?.length}
						<span class="prev" title="Previous ids">
							renamed from {question.previousIds[question.previousIds.length - 1]}
						</span>
					{/if}
				</div>
			{/if}
		</div>
		{#if question}
			<div class="status status-{question.status}">{question.status}</div>
		{/if}
	</header>

	<div class="ed-body">
		{#if !question}
			{#if activeCell}
				<div class="empty">
					<h3>Empty cell</h3>
					<p>
						This board cell has no question. Pick an existing one from the pool, or create a new
						question.
					</p>
					<div class="empty-actions">
						<Button onclick={onPickQuestion}>Pick existing question</Button>
						<Button variant="secondary" onclick={onCreateNew}>+ Create new question</Button>
					</div>
				</div>
			{:else}
				<div class="empty">
					<h3>No question selected</h3>
					<p>Select a board cell on the left, or create a new question from the top bar.</p>
					<div class="empty-actions">
						<Button onclick={onCreateNew}>+ Create new question</Button>
					</div>
				</div>
			{/if}
		{:else}
			<QuestionMetaEditor {question} />
			{#if question.kind === 'numeric'}
				<QuestionNumericVariants {question} />
			{/if}
			{#if question.choices}
				<QuestionChoicesEditor {question} />
			{/if}
			<QuestionReferences questionId={question.id} />
		{/if}
	</div>
</section>

<style>
	.editor {
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: var(--bg-surface);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		overflow: hidden;
	}
	.ed-head {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		padding: var(--space-3) var(--space-4);
		border-bottom: var(--border-width) var(--border-style) var(--border-color);
		gap: var(--space-3);
	}
	.kicker {
		font-size: calc(0.7rem * var(--font-scale));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
	}
	.id-row {
		display: flex;
		gap: var(--space-2);
		align-items: center;
		margin-top: 2px;
		flex-wrap: wrap;
	}
	.qid {
		font-family: var(--font-mono);
		font-size: calc(0.85rem * var(--font-scale));
		background: var(--bg-primary);
		padding: 2px var(--space-2);
		border-radius: var(--radius-sm);
		color: var(--color-text);
	}
	.qid.draft {
		background: color-mix(in srgb, var(--color-accent) 18%, var(--bg-primary));
		color: var(--color-accent);
	}
	.badge-draft {
		font-size: calc(0.7rem * var(--font-scale));
		color: var(--color-accent);
		background: var(--bg-accent-soft);
		padding: 1px var(--space-2);
		border-radius: var(--radius-full);
	}
	.prev {
		font-size: calc(0.7rem * var(--font-scale));
		color: var(--color-text-muted);
		font-family: var(--font-mono);
	}
	.status {
		font-size: calc(0.7rem * var(--font-scale));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 2px var(--space-2);
		border-radius: var(--radius-full);
	}
	.status-draft {
		background: var(--bg-accent-soft);
		color: var(--color-accent);
	}
	.status-named {
		background: var(--bg-success-soft);
		color: var(--color-success);
	}
	.status-referenced {
		background: var(--bg-primary-soft);
		color: var(--color-primary);
	}
	.status-deprecated {
		background: var(--bg-danger-soft);
		color: var(--color-danger);
	}
	.ed-body {
		flex: 1;
		overflow-y: auto;
		padding: var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
	}
	.h-left {
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 0;
	}
	.attach-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: calc(0.8rem * var(--font-scale));
		color: var(--color-text-muted);
		flex-wrap: wrap;
	}
	.attach-info strong {
		color: var(--color-text);
	}
	.attach-row .sep {
		opacity: 0.5;
	}
	.link {
		background: transparent;
		border: none;
		color: var(--color-primary);
		font-family: var(--font-body);
		font-size: calc(0.8rem * var(--font-scale));
		font-weight: 600;
		cursor: pointer;
		padding: 0;
	}
	.link:hover {
		text-decoration: underline;
	}
	.link-danger {
		color: var(--color-danger);
	}
	.empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: center;
		text-align: center;
		gap: var(--space-3);
		padding: var(--space-8);
	}
	.empty h3 {
		font-family: var(--font-heading);
		margin: 0;
	}
	.empty p {
		color: var(--color-text-muted);
		max-width: 32rem;
		margin: 0;
	}
	.empty-actions {
		display: flex;
		gap: var(--space-2);
		margin-top: var(--space-3);
	}
</style>
