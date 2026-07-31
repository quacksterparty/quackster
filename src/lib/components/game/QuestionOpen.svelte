<script lang="ts">
	import type { GridQuizView, QuestionView } from '$lib/bindings/Protocol';
	import { playerColor, playerInitial } from '$lib/playerUi';
	import { room, has } from '$lib/room.svelte';
	import { m } from '$lib/paraglide/messages';
	import { correctnessText, correctnessValueText } from './correctness';
	import Button from '$lib/components/Button.svelte';
	import QuestionPrompt from './QuestionPrompt.svelte';

	let { view, question }: { view: GridQuizView; question: QuestionView | null } = $props();

	const floored = $derived(view.floored);
	const amFloored = $derived(floored !== null && floored === room.player);
	const amLockedOut = $derived(room.player !== null && view.locked_out.includes(room.player));
	const modAnswer = $derived(
		has('Moderate') && question?.answer ? correctnessText(question.answer, question.variant) : null
	);
	const canonicalAnswer = $derived(
		has('Moderate') && question?.answer?.canonical_correctness
			? correctnessValueText(question.answer.canonical_correctness, question.variant)
			: null
	);
	const playableMedia = $derived(
		question?.prompt.media != null && question.prompt.media.kind !== 'Image'
	);

	// Transient disable after buzzing, cleared when the snapshot reflects a floor.
	let buzzed = $state(false);
	$effect(() => {
		void floored;
		buzzed = false;
	});

	function buzz() {
		if (buzzed || floored !== null) return;
		buzzed = true;
		room.send?.({ kind: 'Buzz' });
	}
</script>

<section class="q">
	{#if question}
		<QuestionPrompt {question} playNonce={view.media_play_count} />
	{/if}

	{#if modAnswer}
		<p class="mod-peek">✓ [{question?.answer?.locale}] {modAnswer}</p>
		{#if canonicalAnswer}
			<p class="mod-peek">✓ [{question?.answer?.canonical_locale}] {canonicalAnswer}</p>
		{/if}
	{/if}

	{#if amFloored}
		<div class="floor-you">🎯 {m.you_have_floor()}</div>
	{:else if floored}
		<div class="floor-other">
			<span class="avatar" style:background={playerColor(floored)}>{playerInitial(floored)}</span>
			<strong>{m.player_answering({ name: floored })}</strong>
		</div>
	{:else if amLockedOut}
		<p class="muted center">{m.locked_out_wait()}</p>
	{:else if has('Play')}
		<button class="buzz" disabled={buzzed} onclick={buzz}>
			{buzzed ? m.buzzing() : m.buzz()}
		</button>
	{:else}
		<p class="muted center">{m.waiting_for_buzz()}</p>
	{/if}

	{#if has('Moderate') && playableMedia}
		<Button variant="ghost" onclick={() => room.send?.({ kind: 'PlayMedia' })}>
			▶ {view.media_play_count > 0 ? m.replay_media() : m.play_media()}
		</Button>
	{/if}

	{#if has('Moderate')}
		{#if floored}
			<div class="rules">
				<Button
					variant="danger"
					onclick={() => room.send?.({ kind: 'Rule', verdict: 'incorrect' })}
				>
					✗ {m.rule_wrong()}
				</Button>
				<Button variant="ghost" onclick={() => room.send?.({ kind: 'Rule', verdict: 'void' })}>
					⊘ {m.rule_void()}
				</Button>
				<Button variant="primary" onclick={() => room.send?.({ kind: 'Rule', verdict: 'correct' })}>
					✓ {m.rule_right()}
				</Button>
			</div>
		{:else}
			<Button variant="ghost" onclick={() => room.send?.({ kind: 'CloseQuestion' })}>
				{m.close_question()}
			</Button>
		{/if}
	{/if}
</section>

<style>
	.q {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-6);
		width: 100%;
	}
	.muted {
		color: var(--color-text-muted);
		font-size: clamp(0.95rem, 2.2cqi, 1.4rem);
	}
	.center {
		text-align: center;
	}
	.mod-peek {
		color: var(--color-success);
		font-weight: 600;
		margin: 0;
		font-size: clamp(1rem, 2.6cqi, 1.5rem);
	}
	.floor-you {
		font-family: var(--font-heading);
		font-size: clamp(1.6rem, 5cqi, 3rem);
		color: var(--color-success);
		text-align: center;
	}
	.floor-other {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		font-size: clamp(1.2rem, 3.5cqi, 2rem);
	}
	.avatar {
		width: clamp(1.75rem, 5cqi, 2.75rem);
		height: clamp(1.75rem, 5cqi, 2.75rem);
		border-radius: var(--radius-full);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-inverse);
		font-weight: 700;
		font-family: var(--font-heading);
		font-size: clamp(0.8rem, 2.2cqi, 1.2rem);
		flex-shrink: 0;
		text-transform: uppercase;
	}
	/* Big blind-tap slab, subtle radius — slappable on mobile without looking. */
	.buzz {
		font-family: var(--font-heading);
		font-size: clamp(2.5rem, 9cqi, 5rem);
		font-weight: 800;
		letter-spacing: 0.08em;
		width: min(92cqi, 40rem);
		min-height: 18rem;
		border: none;
		border-radius: var(--radius-md);
		background: var(--color-danger);
		color: var(--color-text-inverse);
		cursor: pointer;
		box-shadow: var(--shadow-lg);
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.buzz:not(:disabled):hover {
		filter: brightness(1.1);
	}
	.buzz:not(:disabled):active {
		transform: scale(0.99);
	}
	.buzz:disabled {
		opacity: 0.6;
		cursor: default;
	}
	.rules {
		display: flex;
		gap: var(--space-2);
		flex-wrap: wrap;
		justify-content: center;
	}
</style>
