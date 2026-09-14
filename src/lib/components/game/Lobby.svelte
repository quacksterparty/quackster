<script lang="ts">
	import type { MediaFetchStatus, PlayerView } from '$lib/bindings/Protocol';
	import PlayerAvatar from '$lib/components/PlayerAvatar.svelte';
	import { room, has } from '$lib/room.svelte';
	import { m } from '$lib/paraglide/messages';
	import { toast } from '$lib/toast.svelte';
	import Button from '$lib/components/Button.svelte';
	import Modal from '$lib/components/Modal.svelte';

	let {
		players,
		media_status
	}: {
		players: Record<string, PlayerView>;
		media_status?: Record<string, MediaFetchStatus> | null;
	} = $props();

	const playerEntries = $derived(Object.entries(players));
	const canStart = $derived(
		playerEntries.some(([, p]) => p.connected && p.grants.includes('Play'))
	);

	let revealed = $state(false);
	let confirmOpen = $state(false);
	let copied = $state(false);
	let copyResetTimer: ReturnType<typeof setTimeout> | null = null;

	async function copyCode() {
		if (!room.code) return;
		try {
			await navigator.clipboard.writeText(room.code);
			copied = true;
			toast.success(m.room_copy_code_success());
			if (copyResetTimer) clearTimeout(copyResetTimer);
			copyResetTimer = setTimeout(() => (copied = false), 1500);
		} catch {
			toast.error(m.room_copy_code_failed());
		}
	}

	// Used by the StartGame confirm dialog. The visible status panel lives in
	// GameStage (top-right corner).
	const mediaTotal = $derived(media_status ? Object.keys(media_status).length : 0);
	const mediaAllReady = $derived(
		!mediaTotal || Object.values(media_status ?? {}).every((s) => s.kind === 'Ready')
	);

	function startGame() {
		if (!canStart) return;
		if (mediaAllReady) {
			room.send?.({ kind: 'StartGame' });
		} else {
			confirmOpen = true;
		}
	}

	function confirmStart() {
		confirmOpen = false;
		room.send?.({ kind: 'StartGame' });
	}
</script>

<section class="lobby">
	<header class="head">
		<h1>{m.lobby_label()}</h1>
		<p class="muted">{m.lobby_players_joined({ count: playerEntries.length })}</p>
		{#if room.code}
			<div class="code-row">
				<button
					class="code"
					type="button"
					onclick={() => (revealed = !revealed)}
					aria-pressed={revealed}
					aria-label={revealed ? m.room_hide_code() : m.room_show_code()}
				>
					<span class="code-label">{m.room_code()}:</span>
					<span class="code-value" aria-hidden="true">{revealed ? room.code : '••••••'}</span>
				</button>
				<button
					class="copy-btn"
					class:copied
					type="button"
					onclick={copyCode}
					aria-label={m.room_copy_code()}
				>
					{#if copied}
						<svg
							class="icon"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2.5"
							stroke-linecap="round"
							stroke-linejoin="round"
							aria-hidden="true"
						>
							<polyline points="5 12 10 17 19 7" />
						</svg>
					{:else}
						<svg
							class="icon"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
							aria-hidden="true"
						>
							<rect x="8" y="8" width="12" height="12" rx="2" />
							<path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2" />
						</svg>
					{/if}
				</button>
			</div>
		{/if}
	</header>

	<ul class="roster">
		{#each playerEntries as [name, p] (name)}
			<li class="row" class:dim={!p.connected}>
				<PlayerAvatar {name} size="xl" />
				<span class="name">
					{name}
					{#if name === room.player}<em>({m.common_you()})</em>{/if}
				</span>
				{#if p.grants.includes('Moderate')}
					<span class="tag">
						🛡️{p.grants.includes('Play') ? '🎮' : ''}
						{m.room_mod_actions()}
					</span>
				{/if}
			</li>
		{/each}
	</ul>

	{#if has('Moderate')}
		<Button size="lg" disabled={!canStart} onclick={startGame}>
			▶ {m.lobby_start()}
		</Button>
	{:else}
		<p class="muted center">{m.lobby_waiting_for_host()}</p>
	{/if}
</section>

<Modal bind:open={confirmOpen} title={m.lobby_start()} description={m.lobby_start_confirm()}>
	<Button onclick={confirmStart}>{m.lobby_start()}</Button>
</Modal>

<style>
	.lobby {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-6);
		width: 100%;
	}
	.head {
		text-align: center;
	}
	.head h1 {
		font-family: var(--font-heading);
		margin: 0;
		font-size: clamp(2rem, 7cqi, 4rem);
	}
	.muted {
		color: var(--color-text-muted);
		font-size: clamp(0.95rem, 2.2cqi, 1.4rem);
	}
	.code {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		background: var(--bg-surface);
		color: inherit;
		font: inherit;
		cursor: pointer;
	}
	.code-row {
		margin-top: var(--space-2);
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
	}
	.copy-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2.4rem;
		height: 2.4rem;
		padding: 0;
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		background: var(--bg-surface);
		color: var(--color-text-muted);
		cursor: pointer;
		transition:
			color var(--duration-fast) var(--easing),
			background var(--duration-fast) var(--easing);
	}
	.copy-btn:hover {
		color: var(--color-text);
	}
	.copy-btn.copied {
		color: var(--color-success, #2ecc71);
	}
	.copy-btn .icon {
		width: 1.1rem;
		height: 1.1rem;
	}
	.code-label {
		color: var(--color-text-muted);
	}
	.code-value {
		font-family: ui-monospace, monospace;
		font-weight: 700;
		letter-spacing: 0.1em;
	}
	.center {
		text-align: center;
	}
	.roster {
		list-style: none;
		margin: 0;
		padding: 0;
		width: min(40rem, 100%);
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.row {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		padding: var(--space-3) var(--space-4);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		background: var(--bg-surface);
		font-size: clamp(1rem, 2.8cqi, 1.6rem);
	}
	.dim {
		opacity: 0.4;
	}
	.name {
		flex: 1;
	}
	.name em {
		color: var(--color-text-muted);
		font-style: normal;
	}
	.tag {
		font-size: clamp(0.75rem, 1.8cqi, 1rem);
		color: var(--color-accent);
		border: var(--border-width) var(--border-style) var(--color-accent);
		border-radius: var(--radius-full);
		padding: 0 0.4rem;
	}
</style>
