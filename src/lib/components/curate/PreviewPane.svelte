<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';

	let {
		activeDraftId,
		activeCell,
		activeQuestionId,
		class: className = ''
	}: {
		activeDraftId: string;
		activeCell: { categoryIdx: number; point: number } | null;
		activeQuestionId: string | null;
		class?: string;
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

	const errors = $derived.by(() => {
		const e: string[] = [];
		if (!draft) return e;
		if (!draft.title.trim()) e.push('Draft title is required');
		if (draft.board.categories.some((c) => Object.values(c.questions).every((cell) => !cell)))
			e.push('At least one cell in every category should have a question');
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

	const progress = $derived(draft ? draft.progress : 0);
</script>

<aside class={['preview-pane', className]} aria-label="Preview and validation">
	<header class="pv-head">
		<h2>Preview &amp; validation</h2>
	</header>

	<section class="pv-section">
		<h3>Draft</h3>
		{#if draft}
			<p class="meta">
				<strong>{draft.title}</strong> · {draft.language.toUpperCase()} · {draft.audience}
			</p>
			<div class="progress-bar">
				<div class="progress-fill" style:width="{progress * 100}%"></div>
			</div>
			<p class="ct">{Math.round(progress * 100)}% board filled</p>
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
			<div class="player-view">
				<div class="pv-pts">
					{draft?.board.categories[activeCell?.categoryIdx ?? 0]?.name ?? 'Question'}{activeCell
						? ` · ${activeCell.point} pts`
						: ''}
				</div>
				<div class="pv-prompt">{question.prompt || '(empty prompt)'}</div>
				{#if question.choices}
					<ul class="pv-choices">
						{#each question.choices as c (c.id)}
							<li class:correct={c.correct}>{c.text || '—'}</li>
						{/each}
					</ul>
				{:else if question.kind === 'numeric'}
					{#if question.range}
						<div class="pv-range">
							<div class="pv-range-label">Range</div>
							<div class="pv-range-band">
								[{question.range.min}, {question.range.max}] {question.unit ?? ''}
							</div>
							<div class="pv-range-meta">
								step {question.range.step} · tolerance ± {question.range.tolerance}
							</div>
						</div>
					{/if}
					{#if question.numericInput}
						<div class="pv-range">
							<div class="pv-range-label">Numeric input</div>
							<div class="pv-range-band">
								{question.answerNumeric ?? 0} ± {question.numericInput.tolerance}
								{question.unit ?? ''}
							</div>
						</div>
					{/if}
					{#if !question.range && !question.numericInput}
						<input class="pv-input" placeholder="Enter number" disabled />
					{/if}
				{:else}
					<input class="pv-input" placeholder="Type your answer" disabled />
				{/if}
				{#if question.explanation}
					<details class="pv-expl">
						<summary>Explanation</summary>
						<p>{question.explanation}</p>
					</details>
				{/if}
			</div>
		</section>
	{/if}
</aside>

<style>
	.preview-pane {
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: var(--bg-surface);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		overflow-y: auto;
	}
	.pv-head {
		padding: var(--space-3) var(--space-4);
		border-bottom: var(--border-width) var(--border-style) var(--border-color);
	}
	.pv-head h2 {
		margin: 0;
		font-family: var(--font-heading);
		font-size: calc(0.95rem * var(--font-scale));
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
	.player-view {
		background: var(--bg-primary);
		padding: var(--space-3);
		border-radius: var(--radius-md);
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.pv-pts {
		font-size: calc(0.7rem * var(--font-scale));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
	}
	.pv-prompt {
		font-family: var(--font-heading);
		font-size: calc(1.1rem * var(--font-scale));
	}
	.pv-choices {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-2);
	}
	.pv-choices li {
		padding: var(--space-2) var(--space-3);
		background: var(--bg-surface);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		font-family: var(--font-body);
		font-size: calc(0.9rem * var(--font-scale));
	}
	.pv-choices li.correct {
		border-color: var(--color-success);
		color: var(--color-success);
	}
	.pv-input {
		padding: var(--space-2) var(--space-3);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		background: var(--bg-surface);
		color: var(--color-text);
	}
	.pv-range {
		padding: var(--space-2) var(--space-3);
		background: var(--bg-surface);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.pv-range-label {
		font-size: calc(0.7rem * var(--font-scale));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
	}
	.pv-range-band {
		font-family: var(--font-mono);
		font-size: calc(1rem * var(--font-scale));
	}
	.pv-range-meta {
		font-size: calc(0.7rem * var(--font-scale));
		color: var(--color-text-muted);
		font-family: var(--font-mono);
	}
	.pv-expl {
		font-size: calc(0.8rem * var(--font-scale));
		color: var(--color-text-muted);
	}
</style>
