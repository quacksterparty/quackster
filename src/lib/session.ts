import { browser } from '$app/environment';
import { isNonEmptyString, isNumber, isObject } from '$lib/util/storage';

const PREFIX = 'session:';
const LAST_KEY = 'session:last';
const MAX_AGE_MS = 12 * 60 * 60 * 1000;

export interface RoomSession {
	room: string;
	token?: string;
	player?: string;
}

type Stored = RoomSession & { timestamp: number };

function parse(raw: string | null): Stored | null {
	if (!raw) return null;
	let value: unknown;
	try {
		value = JSON.parse(raw);
	} catch {
		return null;
	}
	if (!isObject(value)) return null;
	if (!isNonEmptyString(value['room'])) return null;
	if (!isNumber(value['timestamp'])) return null;

	const session: Stored = { room: value['room'], timestamp: value['timestamp'] };
	if (typeof value['token'] === 'string') session.token = value['token'];
	if (typeof value['player'] === 'string') session.player = value['player'];
	return session;
}

export function readSession(room: string): RoomSession | null {
	return parse(localStorage.getItem(PREFIX + room));
}

export function saveSession(session: RoomSession): void {
	localStorage.setItem(
		PREFIX + session.room,
		JSON.stringify({ ...session, timestamp: Date.now() })
	);
	localStorage.setItem(LAST_KEY, session.room);
	prune();
}

export function clearSession(room: string): void {
	localStorage.removeItem(PREFIX + room);
}

/** Most recently saved session, for the home-page rejoin button. */
export function lastSession(): RoomSession | null {
	if (!browser) return null;
	const room = localStorage.getItem(LAST_KEY);
	return room ? readSession(room) : null;
}

function prune(): void {
	// legacy single-key session from before per-room keys
	localStorage.removeItem('room');
	for (const key of Object.keys(localStorage)) {
		if (!key.startsWith(PREFIX) || key === LAST_KEY) continue;
		const entry = parse(localStorage.getItem(key));
		if (!entry || Date.now() - entry.timestamp > MAX_AGE_MS) localStorage.removeItem(key);
	}
}
