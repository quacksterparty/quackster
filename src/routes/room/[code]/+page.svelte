<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { api } from '$lib/api';
	import type { ClientMessage, ClientView, ServerMessage } from '$lib/bindings/Protocol';
	import Button from '$lib/components/Button.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import GameStage from '$lib/components/game/GameStage.svelte';
	import TextInput from '$lib/components/TextInput.svelte';
	import { currentLocale } from '$lib/i18n.svelte';
	import { m } from '$lib/paraglide/messages';
	import { clearSession, lastSession, readSession, saveSession } from '$lib/session';
	import { room, clearRoom } from '$lib/room.svelte';
	import { toast } from '$lib/toast.svelte';
	import { unwrap } from '$lib/util/api';
	import { onMount } from 'svelte';

	let nameOpen = $state(false);
	let name = $state('');
	let snapshot = $state<ClientView>();
	// server sent an Error and will close the socket; skip generic onclose handling
	let serverError = false;

	const code = $derived(page.params['code']);
	let ws: WebSocket | null | undefined;

	async function handleWebsocket() {
		if (code === undefined) {
			await goto(resolve('/', {}));
			return;
		}
		const exists = await unwrap(api.room.exists(code), () => toast.error(m.common_error_generic()));
		if (exists === null) {
			await goto(resolve('/', {}));
			return;
		}
		if (!exists) {
			toast.error(m.common_room_not_found());
			await goto(resolve('/', {}));
			return;
		}

		ws = new WebSocket(`/ws/${code}`);
		room.send = (cmd) => {
			const session = readSession(code);
			if (session?.token) send({ kind: 'Authed', token: session.token, cmd });
		};
		ws.onopen = () => {
			const stored = readSession(code);
			if (stored?.token) {
				room.player = stored.player ?? null;
				send({ kind: 'Reconnect', token: stored.token, locale: currentLocale() });
			} else {
				nameOpen = true;
			}
		};
		ws.onmessage = (ev) => {
			const serverMsg = JSON.parse(String(ev.data)) as ServerMessage;
			switch (serverMsg.kind) {
				case 'Joined':
					nameOpen = false;
					room.player = name;
					saveSession({ room: code, token: serverMsg.token, player: name });
					break;
				case 'Snapshot': {
					const prevPlayers = new Set(Object.keys(snapshot?.players ?? {}));
					const joined = Object.keys(serverMsg.players).filter((p) => !prevPlayers.has(p));
					joined.forEach((p) => toast.success(`${p} joined`));

					room.code = code;
					room.view = serverMsg;
					snapshot = serverMsg;
					break;
				}
				case 'Error':
					// TODO: all current server errors mean the token is absent/dead, so
					// clearing is safe. Once non-fatal errors exist (e.g. server-side name
					// validation), add a structured Error kind and only clear on auth-fatal
					// ones — see todo 6 (kick handling).
					toast.error(serverMsg.message);
					serverError = true;
					clearSession(code);
					clearRoom();
					nameOpen = true;
					break;
				default: {
					console.error(`unhandled ServerMessage`);
				}
			}
		};
		ws.onerror = () => {
			toast.error(m.common_error_generic());
		};
		ws.onclose = () => {
			if (serverError) {
				serverError = false;
				// server closes after every Error; reopen so the name dialog has a live socket
				void handleWebsocket();
				return;
			}
			toast.error(m.common_error_generic());
			void goto(resolve('/', {}));
		};
	}

	onMount(() => {
		if (code === undefined) return;
		name = readSession(code)?.player ?? lastSession()?.player ?? '';
		void handleWebsocket();
		return () => {
			if (ws) {
				ws.onmessage = ws.onerror = ws.onclose = null;
				ws.close();
			}
			clearRoom();
		};
	});

	function send(msg: ClientMessage) {
		ws?.send(JSON.stringify(msg));
	}

	function join() {
		send({ kind: 'Join', name, locale: currentLocale() });
	}
</script>

{#if snapshot}
	<GameStage view={snapshot} />
{/if}

<Modal
	bind:open={nameOpen}
	title="Username"
	onInteractOutside={(e: PointerEvent) => e.preventDefault()}
>
	<form
		onsubmit={(e) => {
			e.preventDefault();
			join();
		}}
	>
		<TextInput bind:value={name} placeholder="Karl" />
		<Button disabled={!name}>{m.common_join()}</Button>
	</form>
</Modal>
