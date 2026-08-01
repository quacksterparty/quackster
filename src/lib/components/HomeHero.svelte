<script lang="ts">
	import Logo from '$lib/components/Logo.svelte';
	import Dialog from '$lib/components/Dialog.svelte';
	import CodeInput from '$lib/components/CodeInput.svelte';
	import TextInput from '$lib/components/TextInput.svelte';
	import { m } from '$lib/paraglide/messages';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api';
	import { getSecret, setSecret } from '$lib/secret';
	import { toast } from '$lib/toast.svelte';
	import { lastSession, type RoomSession } from '$lib/session';
	import Button from './Button.svelte';
	import { onMount } from 'svelte';

	let joinOpen = $state(false);
	let joinCode = $state('');
	let recentSession: RoomSession | null = $state(null);

	let hostOpen = $state(false);
	let secretInput = $state('');

	async function join() {
		if (joinCode.length !== 6) return;

		const result = await api.room.exists(joinCode);

		if (!result.ok) {
			toast.error(m.common_error_generic());
			return;
		}
		if (!result.value) {
			toast.error(m.common_room_not_found());
			return;
		}

		await goto(resolve('/room/[code]', { code: joinCode }));
	}

	async function rejoin() {
		if (recentSession) await goto(resolve('/room/[code]', { code: recentSession.room }));
	}

	function openHost() {
		secretInput = getSecret() ?? '';
		hostOpen = true;
	}

	function host() {
		setSecret(secretInput.trim());
		hostOpen = false;
		void goto(resolve('/room', {}));
	}

	onMount(() => {
		recentSession = lastSession();
	});
</script>

<section class="hero">
	<Logo size="lg" stacked />
	<p class="tagline">{m.common_tagline()}</p>

	<div class="actions">
		<Button size="xl" onclick={() => (joinOpen = true)}>{m.common_join_game()}</Button>
		<Button variant="secondary" size="xl" onclick={openHost}>{m.common_host()}</Button>
	</div>
</section>

<Dialog bind:open={hostOpen} title={m.common_admin_secret()}>
	<form
		onsubmit={(e) => {
			e.preventDefault();
			host();
		}}
	>
		<TextInput bind:value={secretInput} placeholder="Secret" type="password" />
	</form>
	<Button onclick={host}>{m.common_continue()}</Button>
</Dialog>

<Dialog bind:open={joinOpen} title={m.common_join_game()} description={m.common_enter_join_code()}>
	<CodeInput bind:value={joinCode} onComplete={join} />
	<Button disabled={joinCode.length !== 6} onclick={join}>
		{m.common_join()}
	</Button>
	{#if recentSession}
		<Button variant="secondary" onclick={rejoin}>{m.common_rejoin()} {recentSession.room}</Button>
	{/if}
</Dialog>

<style>
	.hero {
		min-height: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-6);
		text-align: center;
		padding: var(--space-12) var(--space-6);
	}
	.tagline {
		margin: 0;
		color: var(--color-text-muted);
		font-size: calc(1rem * var(--font-scale));
	}
	.actions {
		display: flex;
		gap: var(--space-4);
		flex-wrap: wrap;
		justify-content: center;
	}
</style>
