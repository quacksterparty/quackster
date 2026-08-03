<script lang="ts">
	import { PinInput } from 'bits-ui';

	let {
		value = $bindable(''),
		length = 6,
		onComplete
	}: {
		value?: string;
		length?: number;
		onComplete?: (code: string) => void;
	} = $props();

	// Join codes are uppercase; sanitize typed and pasted input alike.
	const sanitize = (t: string) => t.toUpperCase().replace(/[^A-Z0-9]/g, '');
</script>

<PinInput.Root
	bind:value
	maxlength={length}
	textalign="center"
	pasteTransformer={sanitize}
	onValueChange={(v) => (value = sanitize(v))}
	onComplete={() => onComplete?.(value)}
	inputmode="text"
	autocapitalize="characters"
	autocomplete="one-time-code"
	class="bits-pin-input"
>
	{#snippet children({ cells })}
		{#each cells as cell (cell)}
			<PinInput.Cell {cell} class="bits-pin-cell">
				{#if cell.char}
					{cell.char}
				{:else if cell.hasFakeCaret}
					<span class="bits-pin-caret"></span>
				{/if}
			</PinInput.Cell>
		{/each}
	{/snippet}
</PinInput.Root>

<style>
	:global(.bits-pin-input) {
		display: flex;
		gap: var(--space-2);
		justify-content: center;
	}
	:global(.bits-pin-cell) {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 3rem;
		height: 3.5rem;
		font-family: var(--font-mono);
		font-size: calc(1.5rem * var(--font-scale));
		color: var(--color-text);
		background: var(--bg-surface);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		transition: border-color var(--duration-fast) var(--easing);
	}
	:global(.bits-pin-cell[data-active]) {
		border-color: var(--color-primary);
		box-shadow: var(--focus-ring);
	}
	:global(.bits-pin-caret) {
		width: 1px;
		height: 1.5rem;
		background: var(--color-text);
		animation: bits-pin-blink 1s step-end infinite;
	}
	@keyframes bits-pin-blink {
		50% {
			opacity: 0;
		}
	}
</style>
