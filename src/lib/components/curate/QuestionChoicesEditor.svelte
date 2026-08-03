<script lang="ts">
	import type { PoolQuestion } from '$lib/data/seed';
	import { pool } from '$lib/data/pool.svelte';
	import Button from '../Button.svelte';

	let { question }: { question: PoolQuestion } = $props();

	function addChoice() {
		const next = [
			...(question.choices ?? []),
			{ id: `c${(question.choices?.length ?? 0) + 1}`, text: '', correct: false }
		];
		pool.updateQuestion(question.id, { choices: next });
	}

	function removeChoice(i: number) {
		if (!question.choices) return;
		pool.updateQuestion(question.id, {
			choices: question.choices.filter((_, idx) => idx !== i)
		});
	}

	function toggleCorrect(i: number) {
		if (!question.choices) return;
		pool.updateQuestion(question.id, {
			choices: question.choices.map((x, idx) => (idx === i ? { ...x, correct: !x.correct } : x))
		});
	}

	function setChoiceText(i: number, text: string) {
		if (!question.choices) return;
		pool.updateQuestion(question.id, {
			choices: question.choices.map((x, idx) => (idx === i ? { ...x, text } : x))
		});
	}
</script>

{#if question.choices}
	<section class="choices">
		<header class="ch-head">
			<h3>Choices</h3>
			<Button size="sm" variant="ghost" onclick={addChoice}>+ Add</Button>
		</header>
		<ul class="ch-list">
			{#each question.choices as c, i (c.id)}
				<li class="ch">
					<button
						class="ch-correct"
						class:on={c.correct}
						aria-label={c.correct ? 'Mark as incorrect' : 'Mark as correct'}
						onclick={() => {
							toggleCorrect(i);
						}}
					>
						{c.correct ? '✓' : '○'}
					</button>
					<input
						value={c.text}
						oninput={(e) => {
							setChoiceText(i, e.currentTarget.value);
						}}
						placeholder="Choice text"
					/>
					<button
						class="ch-rm"
						aria-label="Remove"
						onclick={() => {
							removeChoice(i);
						}}>✕</button
					>
				</li>
			{/each}
		</ul>
	</section>
{/if}

<style>
	.choices {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.ch-head {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
	}
	.ch-head h3 {
		margin: 0;
		font-family: var(--font-heading);
		font-size: calc(0.95rem * var(--font-scale));
	}
	.ch-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.ch {
		display: grid;
		grid-template-columns: 2.5rem 1fr 2rem;
		gap: var(--space-2);
		align-items: center;
	}
	.ch-correct {
		background: var(--bg-primary);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		width: 2.25rem;
		height: 2.25rem;
		cursor: pointer;
		font-family: var(--font-mono);
		color: var(--color-text-muted);
	}
	.ch-correct.on {
		background: var(--color-success);
		color: var(--color-text-inverse);
		border-color: var(--color-success);
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
	.ch-rm {
		background: transparent;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
	}
</style>
