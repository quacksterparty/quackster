<script lang="ts">
	import { Dialog } from 'bits-ui';
	import type { Snippet } from 'svelte';

	let {
		open = $bindable(false),
		title,
		description,
		trigger,
		children,
		onInteractOutside
	}: {
		open?: boolean;
		title?: string;
		description?: string;
		trigger?: Snippet;
		children?: Snippet;
		onInteractOutside?: (e: PointerEvent) => void;
	} = $props();

	// bits-ui's Dialog.Content types onInteractOutside as non-optional under
	// exactOptionalPropertyTypes; default to noop so we can forward it cleanly.
	const handleInteractOutside = $derived(onInteractOutside ?? (() => {}));
</script>

<Dialog.Root open={open} onOpenChange={(v) => (open = v)}>
	{#if trigger}
		<Dialog.Trigger>{@render trigger()}</Dialog.Trigger>
	{/if}
	<Dialog.Portal>
		<Dialog.Overlay class="bits-modal-overlay" />
		<Dialog.Content class="bits-modal-content" onInteractOutside={handleInteractOutside}>
			{#if title}
				<Dialog.Title class="bits-modal-title">{title}</Dialog.Title>
			{/if}
			{#if description}
				<Dialog.Description class="bits-modal-description">{description}</Dialog.Description>
			{/if}
			{@render children?.()}
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>

<style>
	:global(.bits-modal-overlay) {
		position: fixed;
		inset: 0;
		background: var(--bg-overlay);
		z-index: 50;
	}
	:global(.bits-modal-content) {
		position: fixed;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		z-index: 51;
		width: min(90vw, 28rem);
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding: var(--space-8);
		background: var(--bg-surface-elevated);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-lg);
	}
	/* form mirrors the modal's column layout so its inputs/buttons keep the
	   same spacing instead of collapsing when a submit button lives inside it */
	:global(.bits-modal-content form) {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	:global(.bits-modal-title) {
		margin: 0;
		font-family: var(--font-heading);
		font-size: calc(1.5rem * var(--font-scale));
		color: var(--color-text);
	}
	:global(.bits-modal-description) {
		margin: 0;
		color: var(--color-text-muted);
		font-size: calc(1rem * var(--font-scale));
	}
</style>