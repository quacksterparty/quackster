/**
 * ID generation for curate.
 *
 * Project convention (data-model.md): question IDs are public API, `q_<slug>`,
 * no rename once a question is referenced. New questions start as `q_draft_<rand6>`
 * and auto-rename to `q_<slug>` when the prompt has text and the question is
 * still unreferenced. Once a draft references it, the ID is locked.
 */

const RAND6 = 'abcdefghijklmnopqrstuvwxyz0123456789';

/** Mint a fresh draft ID — used for new questions before they get a real slug. */
export function newDraftQuestionId(): string {
	let s = '';
	for (let i = 0; i < 6; i++) s += RAND6[Math.floor(Math.random() * RAND6.length)] ?? '';
	return `q_draft_${s}`;
}

/** Slugify free-form text into a safe ID suffix. */
export function slugify(input: string): string {
	return input
		.toLowerCase()
		.normalize('NFKD')
		.replace(/[\u0300-\u036f]/g, '') // strip diacritics
		.replace(/[^a-z0-9]+/g, '_')
		.replace(/^_+|_+$/g, '')
		.slice(0, 48);
}

/** Build a real question ID from a slug; appends `-XXXX` on collision. */
export function namedQuestionId(slug: string, existing: ReadonlySet<string>): string {
	const base = `q_${slug || 'unnamed'}`;
	if (!existing.has(base)) return base;
	const chars = (n: number): string => {
		let s = '';
		for (let j = 0; j < n; j++) s += RAND6[Math.floor(Math.random() * RAND6.length)] ?? '';
		return s;
	};
	for (let i = 0; i < 100; i++) {
		const candidate = `${base}_${chars(4)}`;
		if (!existing.has(candidate)) return candidate;
	}
	return `${base}_${Date.now().toString(36)}`;
}

/** True if this is a placeholder `q_draft_` ID that should auto-rename. */
export function isDraftId(id: string): boolean {
	return id.startsWith('q_draft_');
}
