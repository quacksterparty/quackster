<script lang="ts">
	import type { PoolQuestion } from '$lib/data/seed';
	import { pool } from '$lib/data/pool.svelte';

	let { question }: { question: PoolQuestion } = $props();

	function toggleNumericInput(checked: boolean) {
		if (checked) {
			pool.updateQuestion(question.id, { numericInput: { tolerance: 0 } });
		} else {
			pool.updateQuestion(question.id, { numericInput: null });
		}
	}

	function setNumericTolerance(v: number) {
		pool.updateQuestion(question.id, {
			numericInput: { tolerance: Math.max(0, v) }
		});
	}

	function toggleRange(checked: boolean) {
		if (checked) {
			const v = question.answerNumeric ?? 0;
			pool.updateQuestion(question.id, {
				range: { min: v - 5, max: v + 5, step: 1, tolerance: 0 }
			});
		} else {
			pool.updateQuestion(question.id, { range: null });
		}
	}

	function setRangeField(
		key: keyof NonNullable<PoolQuestion['range']>,
		raw: string,
		clamp: (n: number) => number = (n) => n
	) {
		if (!question.range) return;
		pool.updateQuestion(question.id, {
			range: { ...question.range, [key]: clamp(Number(raw)) }
		});
	}
</script>

<section class="variants">
	<header><h3>Numeric answer shapes</h3></header>

	<div class="variant-row">
		<header class="v-head">
			<label class="v-toggle">
				<input
					type="checkbox"
					checked={!!question.numericInput}
					onchange={(e) => {
						toggleNumericInput((e.target as HTMLInputElement).checked);
					}}
				/>
				<strong>Numeric input (exact ± tolerance)</strong>
			</label>
		</header>
		{#if question.numericInput}
			<div class="v-body">
				<label>
					<span>Tolerance</span>
					<input
						type="number"
						step="any"
						min="0"
						value={question.numericInput.tolerance}
						oninput={(e) => {
							setNumericTolerance(Number((e.target as HTMLInputElement).value));
						}}
					/>
				</label>
				<p class="v-hint">
					Accept {question.answerNumeric ?? 0} ± {question.numericInput.tolerance}
					{question.unit ?? ''}
				</p>
			</div>
		{/if}
	</div>

	<div class="variant-row">
		<header class="v-head">
			<label class="v-toggle">
				<input
					type="checkbox"
					checked={!!question.range}
					onchange={(e) => {
						toggleRange((e.target as HTMLInputElement).checked);
					}}
				/>
				<strong>Range (min, max, step, tolerance)</strong>
			</label>
		</header>
		{#if question.range}
			<div class="v-body">
				<div class="v-row">
					<label>
						<span>Min</span>
						<input
							type="number"
							step="any"
							value={question.range.min}
							oninput={(e) => {
								setRangeField('min', e.currentTarget.value);
							}}
						/>
					</label>
					<label>
						<span>Max</span>
						<input
							type="number"
							step="any"
							value={question.range.max}
							oninput={(e) => {
								setRangeField('max', e.currentTarget.value);
							}}
						/>
					</label>
					<label>
						<span>Step</span>
						<input
							type="number"
							step="any"
							min="0"
							value={question.range.step}
							oninput={(e) => {
								setRangeField('step', e.currentTarget.value, (n) => Math.max(0, n));
							}}
						/>
					</label>
					<label>
						<span>Tolerance</span>
						<input
							type="number"
							step="any"
							min="0"
							value={question.range.tolerance}
							oninput={(e) => {
								setRangeField('tolerance', e.currentTarget.value, (n) => Math.max(0, n));
							}}
						/>
					</label>
				</div>
				{#if question.range.max <= question.range.min}
					<p class="v-err">⚠ max must be greater than min</p>
				{:else}
					<p class="v-hint">
						Accept any value in [{question.range.min}, {question.range.max}] (step {question.range
							.step}, ± {question.range.tolerance})
						{question.unit ?? ''}
					</p>
				{/if}
			</div>
		{/if}
	</div>
</section>

<style>
	.variants {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.variants > header h3 {
		margin: 0;
		font-family: var(--font-heading);
		font-size: calc(0.95rem * var(--font-scale));
	}
	.variant-row {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-3);
		background: var(--bg-primary);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
	}
	.v-head {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.v-toggle {
		display: flex;
		gap: var(--space-2);
		align-items: center;
		font-size: calc(0.85rem * var(--font-scale));
		cursor: pointer;
	}
	.v-body {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.v-row {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: var(--space-2);
	}
	label {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	label > span {
		font-size: calc(0.7rem * var(--font-scale));
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}
	input {
		padding: var(--space-2) var(--space-3);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		background: var(--bg-surface);
		color: var(--color-text);
		font-family: var(--font-body);
		font-size: calc(0.95rem * var(--font-scale));
	}
	.v-hint {
		margin: 0;
		font-size: calc(0.8rem * var(--font-scale));
		color: var(--color-text-muted);
		font-family: var(--font-mono);
	}
	.v-err {
		margin: 0;
		font-size: calc(0.8rem * var(--font-scale));
		color: var(--color-danger);
	}
</style>
