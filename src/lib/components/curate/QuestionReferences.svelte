<script lang="ts">
	import { pool } from '$lib/data/pool.svelte';

	let { questionId }: { questionId: string } = $props();
	const refs = $derived(pool.referenceLocations(questionId));
</script>

<footer class="refs">
	{#if refs.length}
		<h3>Referenced by</h3>
		<ul>
			{#each refs as r (r.draft.id + r.category + r.point)}
				<li><strong>{r.draft.title}</strong> · {r.category} · {r.point} pts</li>
			{/each}
		</ul>
	{:else}
		<p class="muted">Not yet referenced by any draft.</p>
	{/if}
</footer>

<style>
	.refs {
		border-top: var(--border-width) var(--border-style) var(--border-color);
		padding-top: var(--space-3);
	}
	.refs h3 {
		font-family: var(--font-body);
		font-size: calc(0.7rem * var(--font-scale));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		margin: 0 0 var(--space-2) 0;
	}
	.refs ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: calc(0.85rem * var(--font-scale));
	}
	.muted {
		color: var(--color-text-muted);
		font-size: calc(0.85rem * var(--font-scale));
		margin: 0;
	}
</style>