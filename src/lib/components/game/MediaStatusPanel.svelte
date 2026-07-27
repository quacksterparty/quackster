<script lang="ts">
	import type { MediaFetchStatus } from '$lib/bindings/Protocol';
	import { room } from '$lib/room.svelte';
	import { m } from '$lib/paraglide/messages';
	import Button from '$lib/components/Button.svelte';

	let { media }: { media: Record<string, MediaFetchStatus> | null } = $props();

	const entries = $derived(Object.entries(media ?? {}));

	let open = $state(false);

	const total = $derived(entries.length);
	const ready = $derived(entries.filter(([, s]) => s.kind === 'Ready').length);
	const failed = $derived(entries.filter(([, s]) => s.kind === 'Failed').length);
	const downloading = $derived(entries.filter(([, s]) => s.kind === 'Downloading').length);
	const all_ready = $derived(total > 0 && ready === total);
	const has_failed = $derived(failed > 0);

	function ref_label(ref: string): string {
		return ref.replace(/^youtube:/, '');
	}

	function state_label(status: MediaFetchStatus): string {
		switch (status.kind) {
			case 'Pending':
				return m.media_state_pending();
			case 'Downloading':
				return m.media_state_downloading();
			case 'Ready':
				return m.media_state_ready();
			case 'Failed':
				return m.media_state_failed();
		}
	}

	function retry_failed() {
		room.send?.({ kind: 'RetryMediaFetch' });
	}

	function summary(): string {
		if (all_ready) return m.media_status_all_ready();
		return m.media_status({ ready, total });
	}
</script>

<aside class="panel" data-state={all_ready ? 'ready' : failed > 0 ? 'failed' : 'loading'}>
	<button
		class="toggle"
		type="button"
		aria-expanded={open}
		aria-label={m.media_status({ ready, total })}
		onclick={() => (open = !open)}
	>
		<span class="icon" aria-hidden="true">ℹ</span>
		<span class="summary">{summary()}</span>
		{#if downloading > 0}
			<span class="dot downloading" aria-hidden="true"></span>
		{:else if failed > 0}
			<span class="dot failed" aria-hidden="true">⚠</span>
		{:else if all_ready}
			<span class="dot ready" aria-hidden="true">✓</span>
		{/if}
	</button>
	{#if open}
		<div class="body">
			<ul class="media-list">
				{#each entries as [ref, status] (ref)}
					<li class="media-row" data-state={status.kind}>
						<span class="media-ref" title={ref}>{m.media_ref_short({ ref: ref_label(ref) })}</span>
						<span class="media-state">{state_label(status)}</span>
						{#if status.kind === 'Failed'}
							<span class="media-err" title={status.message}>⚠</span>
						{/if}
					</li>
				{/each}
			</ul>
			{#if has_failed}
				<Button size="sm" onclick={retry_failed}>{m.media_retry_all()}</Button>
			{/if}
		</div>
	{/if}
</aside>

<style>
	.panel {
		position: absolute;
		top: var(--space-3);
		right: var(--space-3);
		font-size: clamp(0.85rem, 2.2cqi, 1.2rem);
		z-index: 10;
	}
	.toggle {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-full);
		background: var(--bg-surface);
		color: inherit;
		font: inherit;
		cursor: pointer;
		box-shadow: var(--shadow-sm, 0 1px 3px rgb(0 0 0 / 0.1));
	}
	.toggle:hover {
		filter: brightness(1.05);
	}
	.icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1.25em;
		height: 1.25em;
		border-radius: var(--radius-full);
		background: var(--color-text-muted);
		color: var(--bg-surface);
		font-size: 0.85em;
		font-weight: 700;
	}
	.summary {
		white-space: nowrap;
	}
	.dot {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 1.25em;
		height: 1.25em;
		border-radius: var(--radius-full);
		font-size: 0.85em;
		font-weight: 700;
	}
	.dot.downloading {
		background: var(--color-accent);
		color: var(--bg-surface);
		animation: pulse 1.2s ease-in-out infinite;
	}
	.dot.failed {
		background: var(--color-danger, var(--color-accent));
		color: var(--bg-surface);
	}
	.dot.ready {
		background: var(--color-success, var(--color-accent));
		color: var(--bg-surface);
	}
	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.4;
		}
	}
	.panel[data-state='ready'] .toggle {
		border-color: var(--color-success, var(--color-accent));
	}
	.panel[data-state='failed'] .toggle {
		border-color: var(--color-danger, var(--color-accent));
	}
	.body {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-3);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		background: var(--bg-surface);
		min-width: max(20rem, 50%);
		box-shadow: var(--shadow-md, 0 4px 12px rgb(0 0 0 / 0.15));
		position: absolute;
		top: 100%;
		right: 0;
		margin-top: var(--space-2);
	}
	.media-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.media-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}
	.media-ref {
		flex: 1;
		font-family: ui-monospace, monospace;
		opacity: 0.7;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.media-state {
		min-width: 6rem;
		text-align: right;
	}
	.media-row[data-state='Ready'] .media-state {
		color: var(--color-success, var(--color-accent));
	}
	.media-row[data-state='Failed'] .media-state {
		color: var(--color-danger, var(--color-accent));
	}
	.media-err {
		cursor: help;
	}
</style>
