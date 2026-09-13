/**
 * In-memory question pool + draft store, backed by the Quackster REST CRUD
 * (quackster-37). Svelte 5 rune state so /curate and /questions share live
 * updates; the backend is the source of truth and is reconciled on every
 * Save all click (and on page load via `init()`).
 *
 * Mutation model: every store change is local + sync (the UI updates
 * instantly). Each mutation also adds the affected item to a `dirty` set.
 * `saveAll()` is the only async path — it walks every dirty item and POSTs
 * new ones, PUTs edits, DELETEs removed ones.
 *
 * No seed fallback: if the backend is unreachable, the curate page surfaces
 * the load error and stays empty. Visitors without the backend don't get a
 * browse-only mode anymore (was a TODO niceness; not needed in v1).
 */

import { api } from '$lib/api';
import {
	wireToDraft,
	wireToPool,
	poolToWire,
	draftToWire,
	type BoardCellRef,
	type CurateDraft,
	type GridQuizBoard,
	type LinearBoard,
	type PoolQuestion,
	type QuestionVariant
} from '$lib/wire';
import { newDraftQuestionId } from './ids';
import { toast } from '$lib/toast.svelte';

const POINTS_PER_CATEGORY = 5;

function errorMessage(
	err: { kind: 'network' } | { kind: 'http'; status: number; body: string }
): string {
	if (err.kind === 'network') return 'network unreachable';
	// Best-effort: try to surface a structured message; fall back to raw body.
	try {
		const parsed = JSON.parse(err.body) as { message?: string; dependents?: unknown };
		if (parsed.message) return parsed.message;
		if (parsed.dependents)
			return `referenced by ${(parsed.dependents as { id: string }[]).length} item(s)`;
	} catch {
		/* not JSON */
	}
	return err.body || `HTTP ${err.status}`;
}

class PoolStore {
	questions: PoolQuestion[] = $state([]);
	drafts: CurateDraft[] = $state([]);

	/** IDs the server knows about — drives POST vs PUT on save. */
	private serverQuestionIds = new Set<string>();
	private serverDraftIds = new Set<string>();

	/** Items with local changes not yet pushed to the backend. */
	dirtyQuestions: Set<string> = $state(new Set());
	dirtyDrafts: Set<string> = $state(new Set());
	/** Items removed locally; saveAll flushes the DELETE. */
	deletedQuestions: Set<string> = $state(new Set());
	deletedDrafts: Set<string> = $state(new Set());

	loaded = $state(false);
	loading = $state(false);
	loadError: string | null = $state(null);
	saving = $state(false);

	// ── load ────────────────────────────────────────────────────────────────

	async init(): Promise<void> {
		if (this.loading || this.loaded) return;
		this.loading = true;
		this.loadError = null;
		const [qsRes, gsRes] = await Promise.all([
			api.questions.list(true),
			api.games.listFull({ includeDrafts: true })
		]);
		if (!qsRes.ok) {
			this.loadError = `questions: ${errorMessage(qsRes.error)}`;
			this.loading = false;
			return;
		}
		if (!gsRes.ok) {
			this.loadError = `games: ${errorMessage(gsRes.error)}`;
			this.loading = false;
			return;
		}
		this.questions = qsRes.value.map(wireToPool);
		this.serverQuestionIds = new Set(this.questions.map((q) => q.id));
		this.drafts = gsRes.value.map((g) => wireToDraft(g));
		this.serverDraftIds = new Set(this.drafts.map((d) => d.id));
		this.dirtyQuestions.clear();
		this.dirtyDrafts.clear();
		this.deletedQuestions.clear();
		this.deletedDrafts.clear();
		// Recompute progress after load — wire shape doesn't carry it.
		for (const d of this.drafts) {
			if (d.board.mode === 'grid_quiz') d.progress = gridQuizProgress(d.board);
		}
		this.loaded = true;
		this.loading = false;
	}

	// ── queries ─────────────────────────────────────────────────────────────

	getQuestion(id: string | null | undefined): PoolQuestion | null {
		if (!id) return null;
		return this.questions.find((q) => q.id === id) ?? null;
	}
	getDraft(id: string | null | undefined): CurateDraft | null {
		if (!id) return null;
		return this.drafts.find((d) => d.id === id) ?? null;
	}
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

	// ── question mutations ──────────────────────────────────────────────────

	createDraftQuestion(kind: 'text' | 'numeric' = 'text'): PoolQuestion {
		const id = newDraftQuestionId();
		const q: PoolQuestion = {
			id,
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
		this.dirtyQuestions.add(id);
		return q;
	}

	updateQuestion(id: string, patch: Partial<PoolQuestion>): PoolQuestion | null {
		const q = this.questions.find((x) => x.id === id);
		if (!q) return null;
		Object.assign(q, patch);
		this.dirtyQuestions.add(id);
		return q;
	}

	deleteQuestion(id: string): { ok: true } | { ok: false; reason: string } {
		const refs = this.referencedBy(id);
		if (refs.length) return { ok: false, reason: `Referenced by ${refs.length} draft(s)` };
		const i = this.questions.findIndex((q) => q.id === id);
		if (i < 0) return { ok: false, reason: 'Not found' };
		const [removed] = this.questions.splice(i, 1);
		if (!removed) return { ok: false, reason: 'Not found' };
		if (this.serverQuestionIds.has(id)) {
			this.deletedQuestions.add(id);
			this.dirtyQuestions.delete(id);
		} else {
			// Never made it to the server — drop without a DELETE roundtrip.
			this.dirtyQuestions.delete(id);
		}
		return { ok: true };
	}

	// ── board mutations ─────────────────────────────────────────────────────

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
		this.dirtyDrafts.add(draftId);
	}

	detachGridQuestion(draftId: string, categoryIdx: number, point: number): void {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'grid_quiz') return;
		const cat = d.board.categories[categoryIdx];
		if (!cat) return;
		cat.questions[point] = null;
		d.progress = gridQuizProgress(d.board);
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
		this.dirtyDrafts.add(draftId);
	}

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
		this.dirtyDrafts.add(draftId);
	}

	detachLinearSlot(draftId: string, slotIdx: number): void {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'linear') return;
		if (slotIdx < 0 || slotIdx >= d.board.items.length) return;
		d.board.items[slotIdx] = null;
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
		this.dirtyDrafts.add(draftId);
	}

	addLinearSlot(draftId: string): number | null {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'linear') return null;
		d.board.items.push(null);
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
		this.dirtyDrafts.add(draftId);
		return d.board.items.length - 1;
	}

	removeLinearSlot(draftId: string, slotIdx: number): void {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'linear') return;
		if (slotIdx < 0 || slotIdx >= d.board.items.length) return;
		d.board.items.splice(slotIdx, 1);
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
		this.dirtyDrafts.add(draftId);
	}

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
		this.dirtyDrafts.add(draftId);
	}

	removeGridCategory(draftId: string, categoryIdx: number): void {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'grid_quiz') return;
		if (categoryIdx < 0 || categoryIdx >= d.board.categories.length) return;
		d.board.categories.splice(categoryIdx, 1);
		d.progress = gridQuizProgress(d.board);
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
		this.dirtyDrafts.add(draftId);
	}

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
		this.dirtyDrafts.add(draftId);
	}

	removeGridPoint(draftId: string, point: number): void {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'grid_quiz') return;
		for (const c of d.board.categories) delete c.questions[point];
		d.progress = gridQuizProgress(d.board);
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
		this.dirtyDrafts.add(draftId);
	}

	moveLinearSlot(draftId: string, from: number, to: number): void {
		const d = this.getDraft(draftId);
		if (!d || d.board.mode !== 'linear') return;
		const items = d.board.items;
		if (from < 0 || from >= items.length || to < 0 || to >= items.length || from === to) return;
		const [moved] = items.splice(from, 1);
		items.splice(to, 0, moved ?? null);
		d.status = 'unsaved_changes';
		d.updated = new Date().toISOString();
		this.dirtyDrafts.add(draftId);
	}

	setDraftMode(draftId: string, mode: CurateDraft['board']['mode']): void {
		const d = this.getDraft(draftId);
		if (!d) return;
		if (d.board.mode === mode) return;
		d.board =
			mode === 'grid_quiz'
				? ({ mode: 'grid_quiz', categories: [] } as GridQuizBoard)
				: ({ mode: 'linear', items: [] } as LinearBoard);
		d.progress = 0;
		d.status = 'incomplete';
		d.updated = new Date().toISOString();
		this.dirtyDrafts.add(draftId);
	}

	updateDraft(id: string, patch: Partial<CurateDraft>): void {
		const d = this.drafts.find((x) => x.id === id);
		if (!d) return;
		Object.assign(d, patch);
		d.updated = new Date().toISOString();
		this.dirtyDrafts.add(id);
	}

	// ── save all ────────────────────────────────────────────────────────────

	/**
	 * Push every dirty + locally-deleted item to the backend in parallel.
	 * Returns `{ ok, errors }` for the curate page to surface in a toast.
	 */
	async saveAll(): Promise<{ ok: boolean; errors: string[]; saved: number }> {
		if (this.saving) return { ok: false, errors: ['already saving'], saved: 0 };
		this.saving = true;
		const errors: string[] = [];
		let saved = 0;

		// Deletes first — clears dependents that would block PUTs.
		for (const id of [...this.deletedDrafts]) {
			const res = await api.games.delete(id);
			if (res.ok) {
				this.deletedDrafts.delete(id);
				this.serverDraftIds.delete(id);
				saved++;
			} else {
				errors.push(`${id}: ${errorMessage(res.error)}`);
			}
		}
		for (const id of [...this.deletedQuestions]) {
			const res = await api.questions.delete(id);
			if (res.ok) {
				this.deletedQuestions.delete(id);
				this.serverQuestionIds.delete(id);
				saved++;
			} else {
				errors.push(`${id}: ${errorMessage(res.error)}`);
			}
		}

		// PATCH / PUT for dirty items.
		const questionTasks = [...this.dirtyQuestions].map(async (id) => {
			const q = this.questions.find((x) => x.id === id);
			if (!q) {
				this.dirtyQuestions.delete(id);
				return;
			}
			const wire = poolToWire(q);
			const res = this.serverQuestionIds.has(id)
				? await api.questions.update(id, wire)
				: await api.questions.create({ file: `questions/${id}.yaml`, item: wire });
			if (res.ok) {
				this.dirtyQuestions.delete(id);
				this.serverQuestionIds.add(id);
				saved++;
			} else {
				errors.push(`${id}: ${errorMessage(res.error)}`);
			}
		});
		const draftTasks = [...this.dirtyDrafts].map(async (id) => {
			const d = this.drafts.find((x) => x.id === id);
			if (!d) {
				this.dirtyDrafts.delete(id);
				return;
			}
			const wire = draftToWire(d);
			const res = this.serverDraftIds.has(id)
				? await api.games.update(id, wire)
				: await api.games.create({ file: `games/${id}.yaml`, item: wire });
			if (res.ok) {
				this.dirtyDrafts.delete(id);
				this.serverDraftIds.add(id);
				saved++;
			} else {
				errors.push(`${id}: ${errorMessage(res.error)}`);
			}
		});

		await Promise.all([...questionTasks, ...draftTasks]);

		this.saving = false;
		if (errors.length === 0) {
			toast.success(saved > 0 ? `Saved ${saved} item(s)` : 'Nothing to save');
		} else {
			toast.error(`Save failed: ${errors.length} error(s) — see console`);
			console.error('saveAll errors', errors);
		}
		return { ok: errors.length === 0, errors, saved };
	}
}

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
