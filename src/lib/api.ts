/**
 * REST client for the Quackster backend (quackster-37 CRUD layer).
 *
 * Wire types are auto-generated from the Rust types via ts-rs and live in
 * `src/lib/bindings/`. The single source of truth for resource shapes is the
 * Rust enum / struct in `api/src/data/types/*` — never hand-edit a binding.
 *
 * Every method returns `Result<T, ApiError>`. Writes (`post`, `put`, `patch`,
 * `del`) also surface the server's JSON error envelope (`ValidationError` /
 * `DependentsError`) when the status is 4xx — the `ApiError` body is the raw
 * string so callers can `JSON.parse` the relevant shape.
 *
 * Auth: when an admin secret is configured on the server, the UI collects it
 * from the Settings drawer and stashes it in `localStorage`; `send()` adds the
 * `Authorization: Bearer <secret>` header automatically. Reads are public.
 */

import type { CreateRoom, Room } from './bindings/Rooms';
import type { Game } from './bindings/Games';
import type { Question } from './bindings/Questions';
import type { GameConfig } from './bindings/GameConfigs';
import type { Tag } from './bindings/Tags';
import type { Pack } from './bindings/Packs';
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

async function postReq<T>(path: string, body: unknown): Promise<Result<T>> {
	return await req<T>(path, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body)
	});
}

async function putReq<T>(path: string, body: unknown): Promise<Result<T>> {
	return await req<T>(path, {
		method: 'PUT',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body)
	});
}

async function patchReq<T>(path: string, body: unknown): Promise<Result<T>> {
	return await req<T>(path, {
		method: 'PATCH',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body)
	});
}

async function del(path: string): Promise<Result<true>> {
	const res = await send(path, { method: 'DELETE' });
	if (!res.ok) return res;
	return { ok: true, value: true };
}

async function roomExists(code: string): Promise<Result<boolean>> {
	const res = await send(`/rooms/${code}`);
	if (res.ok) return { ok: true, value: true };
	if (res.error.kind === 'http' && res.error.status === 404) {
		return { ok: true, value: false };
	}
	return res;
}

/** Body for `POST /api/questions` — caller picks the file path (no auto-placement). */
export interface CreateQuestionBody {
	file: string;
	item: Question;
}

/** Body for `POST /api/games` — same shape. */
export interface CreateGameBody {
	file: string;
	item: GameConfig;
}

export const api = {
	room: {
		exists: roomExists,
		create: (body: Omit<CreateRoom, 'secret'>) => postReq<Room>('/rooms', body)
	},
	games: {
		list: (opts?: { full?: false; includeDrafts?: boolean }): Promise<Result<Game[]>> => {
			const q = new URLSearchParams();
			if (opts?.includeDrafts) q.set('include_drafts', 'true');
			const qs = q.toString();
			return req<Game[]>(`/games${qs ? `?${qs}` : ''}`);
		},
		listFull: (opts?: { includeDrafts?: boolean }): Promise<Result<GameConfig[]>> => {
			const q = new URLSearchParams();
			q.set('full', 'true');
			if (opts?.includeDrafts) q.set('include_drafts', 'true');
			return req<GameConfig[]>(`/games?${q.toString()}`);
		},
		byId: (id: string, opts?: { full?: false; includeDrafts?: boolean }): Promise<Result<Game>> => {
			const q = new URLSearchParams();
			if (opts?.includeDrafts) q.set('include_drafts', 'true');
			const qs = q.toString();
			return req<Game>(`/games/${id}${qs ? `?${qs}` : ''}`);
		},
		byIdFull: (id: string, opts?: { includeDrafts?: boolean }): Promise<Result<GameConfig>> => {
			const q = new URLSearchParams();
			q.set('full', 'true');
			if (opts?.includeDrafts) q.set('include_drafts', 'true');
			return req<GameConfig>(`/games/${id}?${q.toString()}`);
		},
		create: (body: CreateGameBody) => postReq<GameConfig>('/games', body),
		update: (id: string, item: GameConfig) => putReq<GameConfig>(`/games/${id}`, item),
		patch: (id: string, body: object) => patchReq<GameConfig>(`/games/${id}`, body),
		delete: (id: string) => del(`/games/${id}`)
	},
	questions: {
		list: (includeDrafts?: boolean) =>
			req<Question[]>(`/questions${includeDrafts ? '?include_drafts=true' : ''}`),
		byId: (id: string, includeDrafts?: boolean) =>
			req<Question>(`/questions/${id}${includeDrafts ? '?include_drafts=true' : ''}`),
		create: (body: CreateQuestionBody) => postReq<Question>('/questions', body),
		update: (id: string, item: Question) => putReq<Question>(`/questions/${id}`, item),
		patch: (id: string, body: object) => patchReq<Question>(`/questions/${id}`, body),
		delete: (id: string) => del(`/questions/${id}`)
	},
	packs: {
		list: (includeDrafts?: boolean) =>
			req<Pack[]>(`/packs${includeDrafts ? '?include_drafts=true' : ''}`),
		byId: (id: string, includeDrafts?: boolean) =>
			req<Pack>(`/packs/${id}${includeDrafts ? '?include_drafts=true' : ''}`)
	},
	tags: {
		list: () => req<Tag[]>('/tags'),
		byId: (id: string) => req<Tag>(`/tags/${id}`)
	}
} as const;
