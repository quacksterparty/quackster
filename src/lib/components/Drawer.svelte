<script lang="ts">
	import { Dialog } from 'bits-ui';
	import type { Snippet } from 'svelte';
	import { m } from '$lib/paraglide/messages';

	let {
		open = $bindable(false),
		title,
		header,
		children
	}: {
		/** Controlled open state — toggle from a navbar button, bind:open both ways. */
		open?: boolean;
		/** Accessible title. Rendered visibly unless `header` overrides it. */
		title: string;
		/** Optional visible header (tabs, custom title). When given, `title` is sr-only. */
		header?: Snippet;
		children: Snippet;
	} = $props();
</script>

<Dialog.Root {open} onOpenChange={(v) => (open = v)}>
	<Dialog.Portal>
		<Dialog.Overlay class="bits-drawer-overlay" />
		<Dialog.Content class="bits-drawer-content">
			<div class="drawer-head">
				{#if header}
					{@render header()}
					<Dialog.Title class="sr-only">{title}</Dialog.Title>
				{:else}
					<Dialog.Title class="bits-drawer-title">{title}</Dialog.Title>
				{/if}
				<Dialog.Close class="bits-drawer-close" aria-label={m.common_close()}>✕</Dialog.Close>
			</div>
			<div class="drawer-body">
				{@render children()}
			</div>
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>

<style>
	:global(.bits-drawer-overlay) {
		position: fixed;
		inset: 0;
		background: var(--bg-overlay);
		z-index: 50;
	}
	:global(.bits-drawer-content) {
		position: fixed;
		top: 0;
		right: 0;
		bottom: 0;
		width: min(20rem, 85vw);
		background: var(--bg-surface-elevated);
		border-left: var(--border-width) var(--border-style) var(--border-color);
		box-shadow: var(--shadow-lg);
		padding: var(--space-6);
		z-index: 50;
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}
	:global(.bits-drawer-content[data-state='open']) {
		animation: bits-drawer-in var(--duration-fast) var(--easing);
	}
	@keyframes bits-drawer-in {
		from {
			transform: translateX(100%);
		}
		to {
			transform: translateX(0);
		}
	}
	.drawer-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		flex-shrink: 0;
	}
	.drawer-body {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}
	:global(.bits-drawer-title) {
		font-family: var(--font-heading);
		font-size: calc(1.25rem * var(--font-scale));
		margin: 0;
	}
	:global(.bits-drawer-close) {
		background: none;
		border: none;
		font-size: 1.25rem;
		color: var(--color-text-muted);
		cursor: pointer;
		flex-shrink: 0;
	}
	:global(.bits-drawer-close:hover) {
		color: var(--color-text);
	}
</style>
