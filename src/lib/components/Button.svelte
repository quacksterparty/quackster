<script lang="ts">
	import { Button } from 'bits-ui';
	import type { Snippet } from 'svelte';

	type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';
	type Size = 'sm' | 'md' | 'lg' | 'xl';

	let {
		variant = 'primary',
		size = 'md',
		disabled = false,
		children,
		onclick
	}: {
		variant?: Variant;
		size?: Size;
		disabled?: boolean;
		children?: Snippet;
		onclick?: (e: MouseEvent) => void;
	} = $props();
</script>

<Button.Root class="bits-btn bits-btn-{variant} bits-btn-{size}" {disabled} {onclick}>
	{@render children?.()}
</Button.Root>

<style>
	:global(.bits-btn) {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		font-family: var(--font-body);
		font-weight: 600;
		cursor: pointer;
		border: var(--border-width) var(--border-style) transparent;
		border-radius: var(--radius-md);
		transition:
			background-color var(--duration-fast) var(--easing),
			color var(--duration-fast) var(--easing),
			border-color var(--duration-fast) var(--easing),
			box-shadow var(--duration-fast) var(--easing),
			transform var(--duration-fast) var(--easing);
		white-space: nowrap;
		user-select: none;
	}
	:global(.bits-btn:focus-visible) {
		outline: none;
		box-shadow: var(--focus-ring);
	}
	:global(.bits-btn:disabled) {
		opacity: 0.5;
		cursor: not-allowed;
	}
	:global(.bits-btn:not(:disabled):active) {
		transform: scale(0.98);
	}

	/* ── Variants ── */
	:global(.bits-btn-primary) {
		background: var(--color-primary);
		color: var(--color-text-inverse);
	}
	:global(.bits-btn-primary:not(:disabled):hover) {
		background: var(--color-primary-hover);
	}
	:global(.bits-btn-secondary) {
		background: var(--color-secondary);
		color: var(--color-text-inverse);
	}
	:global(.bits-btn-secondary:not(:disabled):hover) {
		filter: brightness(0.9);
	}
	:global(.bits-btn-ghost) {
		background: transparent;
		color: var(--color-text);
		border-color: var(--border-color);
	}
	:global(.bits-btn-ghost:not(:disabled):hover) {
		background: var(--bg-surface-elevated);
	}
	:global(.bits-btn-danger) {
		background: var(--color-danger);
		color: var(--color-text-inverse);
	}
	:global(.bits-btn-danger:not(:disabled):hover) {
		filter: brightness(0.9);
	}

	/* ── Sizes ── */
	:global(.bits-btn-sm) {
		padding: var(--space-1) var(--space-3);
		font-size: calc(0.875rem * var(--font-scale));
	}
	:global(.bits-btn-md) {
		padding: var(--space-2) var(--space-4);
		font-size: calc(1rem * var(--font-scale));
	}
	:global(.bits-btn-lg) {
		padding: var(--space-3) var(--space-6);
		font-size: calc(1.125rem * var(--font-scale));
	}
	:global(.bits-btn-xl) {
		padding: var(--space-4) var(--space-8);
		font-size: calc(1.375rem * var(--font-scale));
	}
</style>