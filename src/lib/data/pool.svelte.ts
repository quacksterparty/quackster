/**
 * In-memory question pool + draft store. Svelte 5 rune state so /curate and
 * /questions share live updates without any backend.
 *
 * Will be replaced by the real backend in quackster-26. For now: seed data +
 * mutations that update reactively. Server write-through, debounced autosave,
 * revisions, TTL etc. all land with the backend.
 */

import {
	QUESTIONS,
	DRAFTS,
	type PoolQuestion,
	type CurateDraft,
	type BoardCellRef,
	type LinearBoard,
	type GridQuizBoard,
	type QuestionVariant
} from './seed';
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
		return this.drafts.filter((d) => {
			if (d.board.mode === 'grid_quiz') {
				return d.board.categories.some((c) =>
					Object.values(c.questions).some((cell) => cell?.questionId === qid)
				);
			}
			if (d.board.mode === 'linear') {
				return d.board.items.some((slot) => slot?.questionId === qid);
			}
			return false;
		});
	}
	/** All (draft, category, point) locations that reference a question. */
	referenceLocations(qid: string): { draft: CurateDraft; category: string; point: number }[] {
		const out: { draft: CurateDraft; category: string; point: number }[] = [];
		for (const d of this.drafts) {
			if (d.board.mode !== 'grid_quiz') continue;
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
		return this.questions[0]!;
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

	/** Attach an existing question to a grid cell. */
	attachGridQuestion(
		draftId: string,
		categoryIdx: number,
		point: number,
		questionId: string,
		variant?: QuestionVariant
	): void {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'grid_quiz') return;
		const cat = d.board.categories[categoryIdx];
		if (!cat) return;
		cat.questions[point] = { questionId, variant } as BoardCellRef;
		d.progress = gridQuizProgress(d.board);
		d.status = d.progress < 1 ? 'unsaved_changes' : 'saved';
		d.updated = new Date().toISOString();
	}

	/** Detach a grid cell (set to null). */
	detachGridQuestion(draftId: string, categoryIdx: number, point: number): void {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'grid_quiz') return;
		const cat = d.board.categories[categoryIdx];
		if (!cat) return;
		cat.questions[point] = null;
		d.progress = gridQuizProgress(d.board);
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
	}

	/** Attach a question to a linear slot (Kahoot-style ordered list). */
	attachLinearSlot(
		draftId: string,
		slotIdx: number,
		questionId: string,
		variant?: QuestionVariant
	): void {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'linear') return;
		if (slotIdx < 0 || slotIdx >= d.board.items.length) return;
		d.board.items[slotIdx] = { questionId, variant } as BoardCellRef;
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
	}

	/** Detach a linear slot (set to null). */
	detachLinearSlot(draftId: string, slotIdx: number): void {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'linear') return;
		if (slotIdx < 0 || slotIdx >= d.board.items.length) return;
		d.board.items[slotIdx] = null;
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
	}

	/** Append an empty linear slot and return its index. */
	addLinearSlot(draftId: string): number | null {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'linear') return null;
		d.board.items.push(null);
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
		return d.board.items.length - 1;
	}

	/** Drop a linear slot entirely. */
	removeLinearSlot(draftId: string, slotIdx: number): void {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'linear') return;
		if (slotIdx < 0 || slotIdx >= d.board.items.length) return;
		d.board.items.splice(slotIdx, 1);
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
	}

	/** Append a new (empty) grid category with a unique default name. */
	addGridCategory(draftId: string, name?: string): void {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'grid_quiz') return;
		const used = new Set(d.board.categories.map((c) => c.name));
		const base = name?.trim() || `Category ${d.board.categories.length + 1}`;
		let candidate = base;
		let n = 2;
		while (used.has(candidate)) candidate = `${base} (${n++})`;
		d.board.categories.push({ name: candidate, questions: {} });
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
	}

	/** Drop a grid category by index. Attached questions in that column become unreferenced. */
	removeGridCategory(draftId: string, categoryIdx: number): void {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'grid_quiz') return;
		if (categoryIdx < 0 || categoryIdx >= d.board.categories.length) return;
		d.board.categories.splice(categoryIdx, 1);
		d.progress = gridQuizProgress(d.board);
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
	}

	/** Append a new grid point value (max+100, or 100 if empty). Empty slots in every category. */
	addGridPoint(draftId: string): void {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'grid_quiz') return;
		const existing = new Set<number>();
		for (const c of d.board.categories) {
			for (const p of Object.keys(c.questions)) existing.add(+p);
		}
		const next = existing.size > 0 ? Math.max(...existing) + 100 : 100;
		for (const c of d.board.categories) c.questions[next] = null;
		d.progress = gridQuizProgress(d.board);
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
	}

	/** Drop a grid point value across every category. Attached questions become unreferenced. */
	removeGridPoint(draftId: string, point: number): void {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'grid_quiz') return;
		for (const c of d.board.categories) delete c.questions[point];
		d.progress = gridQuizProgress(d.board);
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
	}

	/** Move a linear slot within the list. No-op if out of bounds. */
	moveLinearSlot(draftId: string, from: number, to: number): void {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'linear') return;
		const items = d.board.items;
		if (from < 0 || from >= items.length || to < 0 || to >= items.length || from === to) return;
		const [moved] = items.splice(from, 1);
		items.splice(to, 0, moved ?? null);
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
	}

	/**
	 * Switch a draft to a different gamemode. Replaces the board with a fresh
	 * empty board of that mode — existing cells are lost.
	 */
	setDraftMode(draftId: string, mode: CurateDraft['board']['mode']): void {
		const d = this.getDraft(draftId);
		if (!d) return;
		if (d.board.mode === mode) return;
		d.board = mode === 'grid_quiz'
			? ({ mode: 'grid_quiz', categories: [] } as GridQuizBoard)
			: ({ mode: 'linear', items: [] } as LinearBoard);
		d.progress = 0;
		d.status = 'incomplete';
		d.updated = new Date().toISOString();
	}

	updateDraft(id: string, patch: Partial<CurateDraft>): void {
		const d = this.drafts.find((x) => x.id === id);
		if (!d) return;
		Object.assign(d, patch);
		d.updated = new Date().toISOString();
	}
}

const POINTS_PER_CATEGORY = 5;

function gridQuizProgress(board: GridQuizBoard): number {
	const total = board.categories.length * POINTS_PER_CATEGORY;
	let filled = 0;
	for (const c of board.categories) {
		for (const p of Object.keys(c.questions)) {
			if (c.questions[+p]) filled++;
		}
	}
	return total ? filled / total : 0;
}

export const pool = new PoolStore();
