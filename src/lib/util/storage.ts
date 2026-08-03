/** Type guard: `v` is a non-null object (lets the caller narrow `JSON.parse`). */
export function isObject(v: unknown): v is Record<string, unknown> {
	return typeof v === 'object' && v !== null && !Array.isArray(v);
}

/** Type guard: `v` is a string (and not empty). */
export function isNonEmptyString(v: unknown): v is string {
	return typeof v === 'string' && v.length > 0;
}

/** Type guard: `v` is a finite number. */
export function isNumber(v: unknown): v is number {
	return typeof v === 'number' && Number.isFinite(v);
}