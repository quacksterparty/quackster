<script lang="ts">
	import type { Snippet } from 'svelte';
	import Logo from '$lib/components/Logo.svelte';
	import { m } from '$lib/paraglide/messages';
	import { resolve } from '$app/paths';
	import { room, has } from '$lib/room.svelte';

	let {
		settingsOpen = $bindable(false),
		playersOpen = $bindable(false),
		modOpen = $bindable(false),
		playersCount = 0,
		qr
	}: {
		settingsOpen?: boolean;
		playersOpen?: boolean;
		modOpen?: boolean;
		playersCount?: number;
		qr?: Snippet;
	} = $props();
</script>

<header class="navbar">
	<a class="brand" href={resolve('/', {})}>
		<Logo showWordmark size="sm" />
	</a>
	<div class="nav-actions">
		{#if room.code}
			<!-- RoomQR button slot — uses bits-icon-btn (defined globally) -->
			{#if qr}{@render qr()}{/if}
			<button
				class="bits-icon-btn"
				aria-label={m.common_players_and_scoreboard()}
				aria-expanded={playersOpen}
				onclick={() => (playersOpen = !playersOpen)}
			>
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
					<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
					<circle cx="9" cy="7" r="4" />
					<path d="M22 21v-2a4 4 0 0 0-3-3.87" />
					<path d="M16 3.13a4 4 0 0 1 0 7.75" />
				</svg>
				{#if playersCount}
					<span class="badge">{playersCount}</span>
				{/if}
			</button>
		{/if}
		{#if room.code && has('Moderate')}
			<button
				class="bits-icon-btn"
				aria-label={m.room_mod_actions()}
				aria-expanded={modOpen}
				onclick={() => (modOpen = !modOpen)}
			>
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
					<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
				</svg>
			</button>
		{/if}
		<button
			class="bits-icon-btn"
			aria-label={m.common_settings()}
			aria-expanded={settingsOpen}
			onclick={() => (settingsOpen = !settingsOpen)}
		>
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
				<circle cx="9" cy="9" r="3" />
				<path
					d="M9 2.8v1.7M9 13.5v1.7M2.8 9h1.7M13.5 9h1.7M4.6 4.6l1.2 1.2M12.2 12.2l1.2 1.2M13.4 4.6l-1.2 1.2M5.8 12.2l-1.2 1.2"
				/>
				<circle cx="18" cy="17" r="2" />
				<path
					d="M18 12.6v1.4M18 20v1.4M13.6 17h1.4M21 17h1.4M14.9 13.9l1 1M20.1 19.1l1 1M21.1 13.9l-1 1M15.9 19.1l-1 1"
				/>
			</svg>
		</button>
	</div>
</header>

<style>
	.navbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--space-3) var(--space-6);
		border-bottom: var(--border-width) var(--border-style) var(--border-color);
		background: var(--bg-surface);
		position: sticky;
		top: 0;
		z-index: 10;
	}
	.brand {
		display: flex;
		align-items: center;
		text-decoration: none;
	}
	.nav-actions {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.badge {
		position: absolute;
		top: -0.3rem;
		right: -0.3rem;
		min-width: 1.15rem;
		height: 1.15rem;
		padding: 0 0.3rem;
		border-radius: var(--radius-full);
		background: var(--color-primary);
		color: var(--color-text-inverse);
		font-size: calc(0.7rem * var(--font-scale));
		font-weight: 700;
		display: flex;
		align-items: center;
		justify-content: center;
		border: 2px solid var(--bg-surface);
	}
	@media (max-width: 480px) {
		.navbar {
			padding: var(--space-3);
		}
	}
</style>