const KEY = 'admin:secret';

function storage(): Storage | null {
	// adapter-static pre-renders pages server-side; `localStorage` doesn't exist there.
	return typeof localStorage === 'undefined' ? null : localStorage;
}

export function getSecret(): string | null {
	return storage()?.getItem(KEY) ?? null;
}

export function setSecret(value: string): void {
	storage()?.setItem(KEY, value);
}

export function clearSecret(): void {
	storage()?.removeItem(KEY);
}
