/**
 * In-memory question pool + draft store. Svelte 5 rune state so /curate and
 * /questions share live updates without any backend.
 *
 * Will be replaced by the real backend in quackster-26. For now: seed data +
 * mutations that update reactively. Server write-through, debounced autosave,
 * revisions, TTL etc. all land with the backend.
 */

import { QUESTIONS, DRAFTS, type PoolQuestion, type CurateDraft, type BoardCellRef, type QuestionVariant } from './seed';
import { newDraftQuestionId, namedQuestionId, slugify, isDraftId } from './ids';

class PoolStore {
	questions: PoolQuestion[] = $state([...QUESTIONS]);
	drafts: CurateDraft[] = $state([...DRAFTS]);

	// ── queries ──
	getQuestion(id: string | null | undefined): PoolQuestion | null {
		if (!id) return null;
		return this.questions.find((q) => q.id === id) ?? null;
	}
	getDraft(id: string | null | undefined): CurateDraft | null {
		if (!id) return null;
		return this.drafts.find((d) => d.id === id) ?? null;
	}
	/** Drafts that reference a given question. */
	referencedBy(qid: string): CurateDraft[] {
		return this.drafts.filter((d) =>
			d.board.categories.some((c) => Object.values(c.questions).some((cell) => cell?.questionId === qid))
		);
	}
	/** All (draft, category, point) locations that reference a question. */
	referenceLocations(qid: string): Array<{ draft: CurateDraft; category: string; point: number }> {
		const out: Array<{ draft: CurateDraft; category: string; point: number }> = [];
		for (const d of this.drafts) {
			for (const c of d.board.categories) {
				for (const [p, cell] of Object.entries(c.questions)) {
					if (cell?.questionId === qid) out.push({ draft: d, category: c.name, point: +p });
				}
			}
		}
		return out;
	}

	// ── mutations ──
	updateQuestion(id: string, patch: Partial<PoolQuestion>): PoolQuestion | null {
		const q = this.questions.find((x) => x.id === id);
		if (!q) return null;
		Object.assign(q, patch);
		this.tryRename(q);
		return q;
	}

	/** Create a fresh draft question with a `q_draft_…` id, return it. */
	createDraftQuestion(kind: 'text' | 'numeric' = 'text'): PoolQuestion {
		const q: PoolQuestion = {
			id: newDraftQuestionId(),
			kind,
			prompt: '',
			answer: '',
			numericInput: kind === 'numeric' ? { tolerance: 0 } : null,
			range: null,
			...(kind === 'numeric' ? { answerNumeric: 0 } : {}),
			tags: [],
			license: 'CC-BY-4.0',
			defaultLang: 'en',
			status: 'draft',
			variants: kind === 'numeric' ? ['numeric_input'] : ['open'],
			createdAt: new Date().toISOString()
		};
		this.questions.unshift(q);
		return q;
	}

	/**
	 * If a question is still in `q_draft_…` and has a slugifiable prompt, rename
	 * to `q_<slug>`. Locked once referenced by any draft cell, or already named.
	 */
	tryRename(q: PoolQuestion): void {
		if (!isDraftId(q.id)) return;
		if (this.referencedBy(q.id).length > 0) return;
		if (!q.prompt.trim()) return;
		const slug = slugify(q.prompt);
		if (!slug) return;
		const taken = new Set(this.questions.map((x) => x.id));
		const next = namedQuestionId(slug, taken);
		if (next !== q.id) {
			q.previousIds = [...(q.previousIds ?? []), q.id];
			q.id = next;
			q.status = 'named';
		}
	}

	/** Drop a question from the pool. Refuses if any draft references it. */
	deleteQuestion(id: string): { ok: true } | { ok: false; reason: string } {
		const refs = this.referencedBy(id);
		if (refs.length) return { ok: false, reason: `Referenced by ${refs.length} draft(s)` };
		const i = this.questions.findIndex((q) => q.id === id);
		if (i < 0) return { ok: false, reason: 'Not found' };
		this.questions.splice(i, 1);
		return { ok: true };
	}

	/** Attach an existing question to a board cell. */
	attachQuestion(draftId: string, categoryIdx: number, point: number, questionId: string, variant?: QuestionVariant): void {
		const d = this.getDraft(draftId);
		if (!d) return;
		const cat = d.board.categories[categoryIdx];
		if (!cat) return;
		cat.questions[point] = { questionId, variant } as BoardCellRef;
		d.progress = this.computeDraftProgress(d);
		d.status = d.progress < 1 ? 'unsaved_changes' : 'saved';
		d.updated = new Date().toISOString();
	}

	/** Detach a cell (set to null). */
	detachQuestion(draftId: string, categoryIdx: number, point: number): void {
		const d = this.getDraft(draftId);
		if (!d) return;
		const cat = d.board.categories[categoryIdx];
		if (!cat) return;
		cat.questions[point] = null;
		d.progress = this.computeDraftProgress(d);
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
	}

	updateDraft(id: string, patch: Partial<CurateDraft>): void {
		const d = this.drafts.find((x) => x.id === id);
		if (!d) return;
		Object.assign(d, patch);
		d.updated = new Date().toISOString();
	}

	computeDraftProgress(d: CurateDraft): number {
		const total = d.board.categories.length * POINTS_PER_CATEGORY;
		let filled = 0;
		for (const c of d.board.categories) {
			for (const p of Object.keys(c.questions)) {
				if (c.questions[+p]) filled++;
			}
		}
		return total ? filled / total : 0;
	}
}

const POINTS_PER_CATEGORY = 5;
export const pool = new PoolStore();
