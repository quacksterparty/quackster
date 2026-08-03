<script lang="ts">
	import type { Game } from '$lib/bindings/Games';
	import Card from './Card.svelte';

	let {
		game,
		selected = false,
		onclick
	}: { game: Game; selected?: boolean; onclick?: (e: MouseEvent) => void } = $props();
</script>

<Card {selected} {onclick}>
	<div class="body">
		<h3 class="title">{game.title}</h3>
		<p class="desc">{game.description}</p>
		<div class="meta">
			{#if game.modes.length}
				<span class="m"
					>{(game.modes[0] ?? '').replace('_', ' ')}{#if game.modes.length > 1}
						+{game.modes.length - 1}{/if}</span
				>
				<span class="dot">·</span>
			{/if}
			<span class="m">{game.question_count ?? '?'}q</span>
		</div>
		{#if game.tags.length}
			<div class="tags">
				{#each game.tags.slice(0, 3) as t (t.id)}
					<span class="t-tag">{t.label}</span>
				{/each}
			</div>
		{/if}
	</div>
</Card>

<style>
	/* layout on an element GameCard owns, so scoped styles apply */
	.body {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		height: 100%;
	}
	.title {
		font-family: var(--font-heading);
		font-size: calc(1.125rem * var(--font-scale));
		color: var(--color-text);
	}
	.desc {
		color: var(--color-text-muted);
		font-size: calc(0.875rem * var(--font-scale));
		flex: 1;
	}
	.meta {
		display: flex;
		gap: var(--space-1);
		align-items: center;
		font-size: calc(0.75rem * var(--font-scale));
		color: var(--color-text-muted);
	}
	.m {
		text-transform: capitalize;
	}
	.dot {
		opacity: 0.5;
	}
	.tags {
		display: flex;
		gap: 4px;
		flex-wrap: wrap;
	}
	.t-tag {
		font-size: calc(0.7rem * var(--font-scale));
		padding: 1px var(--space-2);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-full);
		color: var(--color-text-muted);
	}
</style>