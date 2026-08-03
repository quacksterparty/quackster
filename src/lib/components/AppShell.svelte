<script lang="ts">
	import type { Snippet } from 'svelte';
	import Navbar from '$lib/components/Navbar.svelte';
	import SettingsDrawer from '$lib/components/SettingsDrawer.svelte';
	import PlayersDrawer from '$lib/components/PlayersDrawer.svelte';
	import ModDrawer from '$lib/components/ModDrawer.svelte';
	import RoomQR from '$lib/components/RoomQR.svelte';
	import { room } from '$lib/room.svelte';

	let { children }: { children: Snippet } = $props();

	let settingsOpen = $state(false);
	let playersOpen = $state(false);
	let modOpen = $state(false);

	const playersCount = $derived(Object.keys(room.view?.players ?? {}).length);
</script>

<div class="shell">
	<Navbar bind:settingsOpen bind:playersOpen bind:modOpen {playersCount}>
		{#snippet qr()}
			<RoomQR />
		{/snippet}
	</Navbar>
	<main class="main">
		{@render children()}
	</main>
</div>

<SettingsDrawer bind:open={settingsOpen} />
<PlayersDrawer bind:open={playersOpen} />
<ModDrawer bind:open={modOpen} />

<style>
	.shell {
		height: 100dvh;
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}
	.main {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
	}
</style>