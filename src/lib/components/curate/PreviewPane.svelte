<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';
	import { getGamemode } from '$lib/gamemodes';
	import QuestionPrompt from '$lib/components/game/QuestionPrompt.svelte';
	import type { PoolQuestion } from '$lib/wire';
	import type { QuestionView } from '$lib/bindings/Protocol';

	/** Adapt a draft question into the runtime `QuestionView` so we can render
	 * it through the same component the player sees. The runtime deliberately
	 * hides correct markers + numeric tolerance — curator-only debug lands in
	 * the `<details>` block below. */
	function toQuestionView(q: PoolQuestion): QuestionView {
		const prompt = { text: q.prompt, media: null };
		if (q.kind === 'numeric') {
			if (q.range) {
				return {
					prompt,
					variant: { kind: 'Range', min: q.range.min, max: q.range.max, step: q.range.step },
					answer: null
				};
			}
			return { prompt, variant: { kind: 'NumericInput' }, answer: null };
		}
		if (q.kind === 'text' && q.choices && q.choices.length > 0) {
			return {
				prompt,
				variant: {
					kind: 'MultipleChoice',
					choices: q.choices.map((c) => ({ id: c.id, text: c.text, media: null }))
				},
				answer: null
			};
		}
		if (q.kind === 'text' && q.variants.includes('true_false')) {
			return { prompt, variant: { kind: 'TrueFalse' }, answer: null };
		}
		return { prompt, variant: { kind: 'Open' }, answer: null };
	}

	let {
		activeDraftId,
		activeCell,
		activeQuestionId
	}: {
		activeDraftId: string;
		activeCell: unknown;
		activeQuestionId: string | null;
	} = $props();

	const draft = $derived(pool.getDraft(activeDraftId));
	const mode = $derived(draft ? getGamemode(draft.board.mode) : null);
	const cellRef = $derived(draft && mode ? mode.getCellRef(activeCell, draft) : null);
	const question = $derived(
		cellRef
			? pool.getQuestion(cellRef.questionId)
			: activeQuestionId
				? pool.getQuestion(activeQuestionId)
				: null
	);

	const errors = $derived.by(() => {
		const e: string[] = [];
		if (!draft || !mode) return e;
		if (!draft.title.trim()) e.push('Draft title is required');
		if (question) {
			if (!question.prompt.trim()) e.push('Question prompt is empty');
			if (question.kind === 'text' && !question.answer.trim()) e.push('Question answer is empty');
			if (question.kind === 'numeric') {
				if (question.answerNumeric === undefined) e.push('Question exact answer is empty');
				if (!question.numericInput && !question.range)
					e.push('Numeric question needs at least one answer shape (numeric_input or range)');
				if (question.range && question.range.max <= question.range.min)
					e.push('Range max must be greater than min');
			}
			if (question.choices?.filter((c) => c.correct).length === 0)
				e.push('Multiple-choice question needs at least one correct answer');
		}
		return e;
	});

	const warnings = $derived.by(() => {
		const w: string[] = [];
		if (question?.tags.length === 0) w.push('Question has no tags — hard to find in the pool');
		if (!question?.explanation) w.push("No explanation — players won't learn from wrong answers");
		return w;
	});

	const progress = $derived(draft && mode ? mode.computeProgress(draft) : 0);
	const statusLine = $derived(draft && mode ? mode.statusLine(draft) : '');
	const cellLabel = $derived(
		draft && mode && activeCell !== null ? mode.cellLabel(activeCell, draft) : ''
	);
</script>

<div class="preview-body">
	<section class="pv-section">
		<h3>Draft</h3>
		{#if draft}
			<p class="meta">
				<strong>{draft.title}</strong> · {mode?.title ?? draft.board.mode} · {draft.language.toUpperCase()}
				· {draft.audience}
			</p>
			{#if statusLine}
				<p class="ct">{statusLine}</p>
			{/if}
			<div class="progress-bar">
				<div class="progress-fill" style:width="{progress * 100}%"></div>
			</div>
			<p class="ct">{Math.round(progress * 100)}% filled</p>
		{/if}
	</section>

	<section class="pv-section">
		<h3>Errors <span class="ct">({errors.length})</span></h3>
		{#if errors.length === 0}
			<p class="ok">No blocking errors. ✓</p>
		{:else}
			<ul class="list err">
				{#each errors as e (e)}
					<li>⨯ {e}</li>
				{/each}
			</ul>
		{/if}
	</section>

	<section class="pv-section">
		<h3>Warnings <span class="ct">({warnings.length})</span></h3>
		{#if warnings.length === 0}
			<p class="muted">None.</p>
		{:else}
			<ul class="list warn">
				{#each warnings as w (w)}
					<li>⚠ {w}</li>
				{/each}
			</ul>
		{/if}
	</section>

	{#if question}
		<section class="pv-section">
			<h3>How a player sees it</h3>
			<div class="player-frame">
				<QuestionPrompt question={toQuestionView(question)} />
			</div>
			<details class="author-debug">
				<summary>Author debug</summary>
				{#if question.choices}
					<ul class="dbg-choices">
						{#each question.choices as c (c.id)}
							<li class:correct={c.correct}>
								{c.correct ? '✓' : '✗'}
								{c.text || '—'}
							</li>
						{/each}
					</ul>
				{:else if question.kind === 'numeric'}
					{#if question.range}
						<p class="dbg-meta">
							[{question.range.min}, {question.range.max}] {question.unit ?? ''} · step {question
								.range.step} · tolerance ± {question.range.tolerance}
						</p>
					{/if}
					{#if question.numericInput}
						<p class="dbg-meta">
							numeric input: {question.answerNumeric ?? '?'} ± {question.numericInput.tolerance}
							{question.unit ?? ''}
						</p>
					{/if}
				{:else}
					<p class="dbg-meta">answer: {question.answer || '(empty)'}</p>
				{/if}
				{#if question.explanation}
					<p class="dbg-meta">explanation: {question.explanation}</p>
				{/if}
			</details>
		</section>
	{/if}
</div>

<style>
	.preview-body {
		display: flex;
		flex-direction: column;
		min-height: 0;
	}
	.pv-section {
		padding: var(--space-3) var(--space-4);
		border-bottom: var(--border-width) var(--border-style) var(--border-color);
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.pv-section h3 {
		margin: 0;
		font-family: var(--font-body);
		font-size: calc(0.7rem * var(--font-scale));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
	}
	.meta {
		margin: 0;
		font-size: calc(0.85rem * var(--font-scale));
	}
	.ct {
		font-family: var(--font-mono);
		font-size: calc(0.75rem * var(--font-scale));
		color: var(--color-text-muted);
	}
	.progress-bar {
		height: 6px;
		background: var(--border-color);
		border-radius: var(--radius-full);
		overflow: hidden;
	}
	.progress-fill {
		height: 100%;
		background: var(--color-primary);
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: calc(0.8rem * var(--font-scale));
	}
	.list.err li {
		color: var(--color-danger);
	}
	.list.warn li {
		color: var(--color-warning);
	}
	.ok {
		color: var(--color-success);
		margin: 0;
		font-size: calc(0.85rem * var(--font-scale));
	}
	.muted {
		color: var(--color-text-muted);
		font-size: calc(0.8rem * var(--font-scale));
		margin: 0;
	}
	.author-debug {
		font-size: calc(0.8rem * var(--font-scale));
		color: var(--color-text-muted);
	}
	/* Scope QuestionPrompt's `cqi` font-scale to this pane — without an
	 * inline-size container the cqi resolves against the viewport and the
	 * prompt renders at room-stage size (3.5rem h2, etc.). */
	.player-frame {
		container-type: inline-size;
	}
	.author-debug summary {
		cursor: pointer;
		font-family: var(--font-body);
	}
	.dbg-choices {
		list-style: none;
		margin: var(--space-2) 0 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.dbg-choices li {
		padding: var(--space-1) var(--space-2);
		border-left: 3px solid var(--border-color);
	}
	.dbg-choices li.correct {
		border-left-color: var(--color-success);
		color: var(--color-success);
	}
	.dbg-meta {
		margin: var(--space-1) 0 0;
		font-family: var(--font-mono);
		font-size: calc(0.75rem * var(--font-scale));
	}
</style>
