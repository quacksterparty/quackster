<script lang="ts">
	import Logo from '$lib/components/Logo.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import CodeInput from '$lib/components/CodeInput.svelte';
	import TextInput from '$lib/components/TextInput.svelte';
	import { m } from '$lib/paraglide/messages';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api';
	import { getSecret, setSecret } from '$lib/secret';
	import { toast } from '$lib/toast.svelte';
	import { lastSession } from '$lib/session';
	import { unwrap } from '$lib/util/api';
	import Button from './Button.svelte';

	let joinOpen = $state(false);
	let joinCode = $state('');
	// Read synchronously — localStorage is sync; `lastSession()` returns null on SSR.
	const recentSession = lastSession();

	let hostOpen = $state(false);
	let secretInput = $state('');

	async function join() {
		if (joinCode.length !== 6) return;

		const exists = await unwrap(api.room.exists(joinCode), () =>
			toast.error(m.common_error_generic())
		);
		if (exists === null) return;
		if (!exists) {
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
</script>

<section class="hero">
	<Logo size="lg" stacked />
	<p class="tagline">{m.common_tagline()}</p>

	<div class="actions">
		<Button size="xl" onclick={() => (joinOpen = true)}>{m.common_join_game()}</Button>
		<Button variant="secondary" size="xl" onclick={openHost}>{m.common_host()}</Button>
	</div>
</section>

<Modal bind:open={hostOpen} title={m.common_admin_secret()}>
	<form
		onsubmit={(e) => {
			e.preventDefault();
			host();
		}}
	>
		<TextInput bind:value={secretInput} placeholder="Secret" type="password" />
	</form>
	<Button onclick={host}>{m.common_continue()}</Button>
</Modal>

<Modal bind:open={joinOpen} title={m.common_join_game()} description={m.common_enter_join_code()}>
	<CodeInput bind:value={joinCode} onComplete={join} />
	<Button disabled={joinCode.length !== 6} onclick={join}>
		{m.common_join()}
	</Button>
	{#if recentSession}
		<Button variant="secondary" onclick={rejoin}>{m.common_rejoin()} {recentSession.room}</Button>
	{/if}
</Modal>

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
