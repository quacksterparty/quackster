import type { ClientView, Command } from './bindings/Protocol';
import type { Grant } from './bindings/Grants';

/**
 * Shared room state. The room page owns the WebSocket and writes snapshots
 * here; the AppShell reads it to render the players drawer. Reactive across
 * modules because $state signals are shared by reference.
 */
export const room = $state<{
	code: string | null;
	player: string | null;
	view: ClientView | null;
	send: ((cmd: Command) => void) | null;
}>({
	code: null,
	player: null,
	view: null,
	send: null
});

export function clearRoom(): void {
	room.code = null;
	room.player = null;
	room.view = null;
	room.send = null;
}

/** Own slot (grants, score, connected), or null before Join resolves. */
export function me() {
	if (!room.player) return null;
	return room.view?.players[room.player] ?? null;
}

/** Grant check for templates: `{#if has('Moderate')}…`. Reads $state, reactive. */
export function has(g: Grant): boolean {
	const grants = me()?.grants ?? [];
	return grants.includes(g);
}