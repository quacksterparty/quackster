const KEY = 'admin:secret';

export function getSecret(): string | null {
	return localStorage.getItem(KEY);
}

export function setSecret(value: string): void {
	localStorage.setItem(KEY, value);
}

export function clearSecret(): void {
	localStorage.removeItem(KEY);
}
