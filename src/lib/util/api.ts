import type { ApiError, Result } from '$lib/api';

/** Awaits a `Result`, runs `onError` on failure, returns value or null. */
export async function unwrap<T>(
	promise: Promise<Result<T>>,
	onError: (e: ApiError) => void
): Promise<T | null> {
	const r = await promise;
	if (r.ok) return r.value;
	onError(r.error);
	return null;
}