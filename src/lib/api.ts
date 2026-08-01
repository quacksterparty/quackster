import type { CreateRoom, Room } from './bindings/Rooms';
import type { Game } from './bindings/Games';
import { currentLocale } from './i18n.svelte';
import { getSecret } from './secret';

const API = '/api';

export type Result<T, E = ApiError> = { ok: true; value: T } | { ok: false; error: E };

export type ApiError = { kind: 'network' } | { kind: 'http'; status: number; body: string };

async function send(path: string, init?: RequestInit): Promise<Result<Response>> {
	const headers = new Headers(init?.headers);
	headers.set('Accept-Language', currentLocale());
	const secret = getSecret();
	if (secret !== null) headers.set('Authorization', `Bearer ${secret}`);

	let res: Response;
	try {
		res = await fetch(API + path, { ...init, headers });
	} catch {
		return { ok: false, error: { kind: 'network' } };
	}
	if (!res.ok) {
		return { ok: false, error: { kind: 'http', status: res.status, body: await res.text() } };
	}
	return { ok: true, value: res };
}

async function req<T>(path: string, init?: RequestInit): Promise<Result<T>> {
	const res = await send(path, init);
	if (!res.ok) return res;
	return { ok: true, value: (await res.value.json()) as T };
}

async function post<T>(path: string, body: unknown, init?: RequestInit): Promise<Result<T>> {
	return await req<T>(path, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body),
		...init
	});
}

async function roomExists(code: string): Promise<Result<boolean>> {
	const res = await send(`/rooms/${code}`);
	if (res.ok) return { ok: true, value: true };
	if (res.error.kind === 'http' && res.error.status === 404) {
		return { ok: true, value: false };
	}
	return res;
}

export const api = {
	room: {
		exists: roomExists,
		create: (body: Omit<CreateRoom, 'secret'>) => post<Room>('/rooms', body)
	},
	games: {
		list: () => req<Game[]>('/games')
	}
};
