<script lang="ts">
	import type { PoolQuestion } from '$lib/data/seed';
	import { pool } from '$lib/data/pool.svelte';

	let { question }: { question: PoolQuestion } = $props();

	function setField<K extends keyof PoolQuestion>(key: K, value: PoolQuestion[K]): void {
		pool.updateQuestion(question.id, { [key]: value });
	}

	function onKindChange(next: PoolQuestion['kind']) {
		setField('kind', next);
		// First-time switch to numeric — populate the answer + shape from current text answer.
		if (next === 'numeric' && question.answerNumeric === undefined) {
			setField('answerNumeric', Number(question.answer) || 0);
			setField('numericInput', { tolerance: 0 });
		}
	}
</script>

<div class="grid-2">
	<label>
		<span>Prompt</span>
		<textarea
			rows="3"
			value={question.prompt}
			oninput={(e) => {
				setField('prompt', e.currentTarget.value);
			}}
			placeholder="e.g. In welchem Jahr fiel die Berliner Mauer?"></textarea>
	</label>
	<label>
		<span>Kind</span>
		<select
			value={question.kind}
			onchange={(e) => {
				onKindChange(e.currentTarget.value as PoolQuestion['kind']);
			}}
		>
			<option value="text">Text</option>
			<option value="numeric">Numeric</option>
			<option value="order">Order</option>
		</select>
	</label>
	{#if question.kind === 'numeric'}
		<label>
			<span>Exact answer</span>
			<input
				type="number"
				step="any"
				value={question.answerNumeric ?? 0}
				oninput={(e) => {
					setField('answerNumeric', Number(e.currentTarget.value));
				}}
			/>
		</label>
		<label>
			<span>Unit <small>(optional)</small></span>
			<input
				value={question.unit ?? ''}
				oninput={(e) => {
					setField('unit', e.currentTarget.value);
				}}
				placeholder="e.g. km, °C"
			/>
		</label>
	{:else}
		<label>
			<span>Answer</span>
			<input
				value={question.answer}
				oninput={(e) => {
					setField('answer', e.currentTarget.value);
				}}
				placeholder="e.g. 1989"
			/>
		</label>
	{/if}
	<label>
		<span>Default language</span>
		<select
			value={question.defaultLang}
			onchange={(e) => {
				setField('defaultLang', e.currentTarget.value as PoolQuestion['defaultLang']);
			}}
		>
			<option value="de">Deutsch</option>
			<option value="en">English</option>
		</select>
	</label>
	<label class="full">
		<span>Explanation <small>(optional)</small></span>
		<textarea
			rows="2"
			value={question.explanation ?? ''}
			oninput={(e) => {
				setField('explanation', e.currentTarget.value);
			}}></textarea>
	</label>
</div>

<style>
	.grid-2 {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-3);
	}
	label {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	label.full {
		grid-column: 1 / -1;
	}
	label > span {
		font-size: calc(0.7rem * var(--font-scale));
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}
	label small {
		text-transform: none;
		opacity: 0.7;
	}
	input,
	textarea,
	select {
		padding: var(--space-2) var(--space-3);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		background: var(--bg-primary);
		color: var(--color-text);
		font-family: var(--font-body);
		font-size: calc(0.95rem * var(--font-scale));
	}
</style>
