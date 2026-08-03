<script lang="ts">
	import { playerColor, playerInitial } from '$lib/playerUi';

	let {
		name,
		size = 'md'
	}: {
		name: string;
		size?: 'sm' | 'md' | 'lg' | 'xl';
	} = $props();
</script>

<span class="avatar avatar-{size}" style:background={playerColor(name)} aria-hidden="true">
	{playerInitial(name)}
</span>

<style>
	.avatar {
		border-radius: var(--radius-full);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-inverse);
		font-weight: 700;
		font-family: var(--font-heading);
		flex-shrink: 0;
		text-transform: uppercase;
	}
	.avatar-sm {
		width: 1.5rem;
		height: 1.5rem;
		font-size: calc(0.7rem * var(--font-scale));
	}
	.avatar-md {
		width: 2rem;
		height: 2rem;
		font-size: calc(0.85rem * var(--font-scale));
	}
	.avatar-lg {
		width: clamp(1.75rem, 5cqi, 2.75rem);
		height: clamp(1.75rem, 5cqi, 2.75rem);
		font-size: clamp(0.8rem, 2.2cqi, 1.2rem);
	}
	.avatar-xl {
		width: clamp(2.25rem, 6cqi, 3.5rem);
		height: clamp(2.25rem, 6cqi, 3.5rem);
		font-size: clamp(0.95rem, 2.4cqi, 1.4rem);
	}
</style>
