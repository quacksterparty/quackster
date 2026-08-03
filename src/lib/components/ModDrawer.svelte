<script lang="ts">
	import Drawer from '$lib/components/Drawer.svelte';
	import Button from '$lib/components/Button.svelte';
	import { m } from '$lib/paraglide/messages';
	import { toast } from '$lib/toast.svelte';
	import { room } from '$lib/room.svelte';

	let { open = $bindable(false) }: { open?: boolean } = $props();

	function endGame() {
		room.send?.({ kind: 'EndGame' });
		toast.success(m.room_end_game_sent());
		open = false;
	}
</script>

<Drawer bind:open title={m.room_mod_actions()}>
	<div class="section">
		<Button variant="danger" onclick={endGame}>{m.room_end_game()}</Button>
	</div>
	<!-- overrule/revision judgments + grant management slots land when #15 is wired -->
</Drawer>

<style>
	.section {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
</style>
