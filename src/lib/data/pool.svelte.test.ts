import { describe, it, expect, beforeEach } from 'vitest';
import { pool } from './pool.svelte';
import type { CurateDraft, PoolQuestion } from './seed';

function freshDrafts(): CurateDraft[] {
	return [
		{
			id: 'draft_test',
			title: 'Test draft',
			language: 'en',
			audience: 'family',
			status: 'incomplete',
			progress: 0,
			updated: new Date().toISOString(),
			board: {
				mode: 'grid_quiz',
				categories: [
					{ name: 'Cat A', questions: { 100: null, 200: null, 300: null, 400: null, 500: null } }
				]
			},
			rules: { buzz_policy: 'open_floor', scoring_mode: 'first_correct', judge: 'auto' }
		}
	];
}

function freshQuestions(): PoolQuestion[] {
	return [
		{
			id: 'q_existing',
			kind: 'text',
			prompt: 'Existing prompt',
			answer: 'existing answer',
			numericInput: null,
			range: null,
			tags: [],
			license: 'CC-BY-4.0',
			defaultLang: 'en',
			status: 'named',
			variants: ['open'],
			createdAt: new Date().toISOString()
		}
	];
}

function grid(d: CurateDraft) {
	if (d.board.mode !== 'grid_quiz') throw new Error('expected grid board');
	return d.board;
}
function linear(d: CurateDraft) {
	if (d.board.mode !== 'linear') throw new Error('expected linear board');
	return d.board;
}

beforeEach(() => {
	pool.questions = freshQuestions();
	pool.drafts = freshDrafts();
});

describe('PoolStore.createDraftQuestion', () => {
	it('mints a q_draft_ id and defaults status to draft', () => {
		const q = pool.createDraftQuestion();
		expect(q.id.startsWith('q_draft_')).toBe(true);
		expect(q.status).toBe('draft');
		expect(q.kind).toBe('text');
	});

	it('seeds numeric answers when kind is numeric', () => {
		const q = pool.createDraftQuestion('numeric');
		expect(q.kind).toBe('numeric');
		expect(q.answerNumeric).toBe(0);
		expect(q.numericInput).toEqual({ tolerance: 0 });
		expect(q.variants).toContain('numeric_input');
	});

	it('prepends new questions so they show first', () => {
		const before = pool.questions.length;
		pool.createDraftQuestion();
		expect(pool.questions.length).toBe(before + 1);
		expect(pool.questions[0]!.id.startsWith('q_draft_')).toBe(true);
	});
});

describe('PoolStore.tryRename', () => {
	it('renames a draft id to q_<slug> once the prompt has text', () => {
		const q = pool.createDraftQuestion();
		pool.updateQuestion(q.id, { prompt: 'Hello World' });
		expect(q.id).toBe('q_hello_world');
		expect(q.status).toBe('named');
		expect(q.previousIds?.[0]).toMatch(/^q_draft_/);
	});

	it('stays as draft id when prompt is empty', () => {
		const q = pool.createDraftQuestion();
		expect(q.id.startsWith('q_draft_')).toBe(true);
	});

	it('does not rename once the question is referenced by a draft', () => {
		const q = pool.createDraftQuestion();
		pool.attachGridQuestion('draft_test', 0, 100, q.id);
		const idBefore = q.id;
		pool.updateQuestion(q.id, { prompt: 'Should Not Rename' });
		expect(q.id).toBe(idBefore);
	});

	it('appends a suffix when the slug collides', () => {
		pool.questions.push({
			id: 'q_hello_world',
			kind: 'text',
			prompt: 'collision',
			answer: '',
			numericInput: null,
			range: null,
			tags: [],
			license: 'CC-BY-4.0',
			defaultLang: 'en',
			status: 'named',
			variants: ['open'],
			createdAt: new Date().toISOString()
		});
		const q = pool.createDraftQuestion();
		pool.updateQuestion(q.id, { prompt: 'Hello World' });
		expect(q.id.startsWith('q_hello_world_')).toBe(true);
		expect(q.id).not.toBe('q_hello_world');
	});
});

describe('PoolStore.attachGridQuestion / detachGridQuestion', () => {
	it('fills the cell and bumps progress', () => {
		pool.attachGridQuestion('draft_test', 0, 100, 'q_existing');
		expect(grid(pool.drafts[0]!).categories[0]!.questions[100]).toEqual({
			questionId: 'q_existing'
		});
		expect(pool.drafts[0]!.progress).toBeGreaterThan(0);
		expect(pool.drafts[0]!.status).toBe('unsaved_changes');
	});

	it('marks status saved when fully filled', () => {
		for (const p of [100, 200, 300, 400, 500]) {
			pool.attachGridQuestion('draft_test', 0, p, 'q_existing');
		}
		expect(pool.drafts[0]!.progress).toBe(1);
		expect(pool.drafts[0]!.status).toBe('saved');
	});

	it('detach clears the cell', () => {
		pool.attachGridQuestion('draft_test', 0, 100, 'q_existing');
		pool.detachGridQuestion('draft_test', 0, 100);
		expect(grid(pool.drafts[0]!).categories[0]!.questions[100]).toBeNull();
	});

	it('does nothing for an unknown draft', () => {
		pool.attachGridQuestion('draft_test', 0, 100, 'q_existing');
		const before = grid(pool.drafts[0]!).categories[0]!.questions[100];
		pool.attachGridQuestion('nope', 0, 100, 'q_existing');
		expect(grid(pool.drafts[0]!).categories[0]!.questions[100]).toBe(before);
	});

	it('refuses to attach when board is not grid_quiz', () => {
		pool.setDraftMode('draft_test', 'linear');
		pool.attachGridQuestion('draft_test', 0, 100, 'q_existing');
		expect(linear(pool.drafts[0]!).items).toEqual([]);
	});
});

describe('PoolStore.linear slots', () => {
	it('addLinearSlot appends and returns the new index', () => {
		pool.setDraftMode('draft_test', 'linear');
		expect(pool.addLinearSlot('draft_test')).toBe(0);
		pool.addLinearSlot('draft_test');
		expect(linear(pool.drafts[0]!).items).toEqual([null, null]);
	});

	it('attachLinearSlot fills and detachLinearSlot clears', () => {
		pool.setDraftMode('draft_test', 'linear');
		pool.addLinearSlot('draft_test');
		pool.addLinearSlot('draft_test');
		pool.attachLinearSlot('draft_test', 1, 'q_existing');
		expect(linear(pool.drafts[0]!).items[1]).toEqual({ questionId: 'q_existing' });
		pool.detachLinearSlot('draft_test', 1);
		expect(linear(pool.drafts[0]!).items[1]).toBeNull();
	});

	it('removeLinearSlot splices and moveLinearSlot reorders', () => {
		pool.setDraftMode('draft_test', 'linear');
		pool.addLinearSlot('draft_test');
		pool.addLinearSlot('draft_test');
		pool.addLinearSlot('draft_test');
		pool.attachLinearSlot('draft_test', 0, 'q_existing');
		pool.attachLinearSlot('draft_test', 2, 'q_existing');
		pool.moveLinearSlot('draft_test', 2, 0);
		// [q0, null1, q2] → move slot 2 to position 0 → [q2, q0, null2]
		expect(linear(pool.drafts[0]!).items[0]).toEqual({ questionId: 'q_existing' });
		expect(linear(pool.drafts[0]!).items[2]).toBeNull();
		pool.moveLinearSlot('draft_test', 99, 0); // no-op
		expect(linear(pool.drafts[0]!).items).toHaveLength(3);
		pool.removeLinearSlot('draft_test', 0);
		expect(linear(pool.drafts[0]!).items).toHaveLength(2);
	});

	it('linear operations are no-ops on a grid board', () => {
		expect(pool.addLinearSlot('draft_test')).toBeNull();
		expect(pool.drafts[0]!.board.mode).toBe('grid_quiz');
	});
});

describe('PoolStore.addGridCategory / removeGridCategory', () => {
	it('addGridCategory appends with a unique default name', () => {
		pool.addGridCategory('draft_test');
		expect(grid(pool.drafts[0]!).categories).toHaveLength(2);
		expect(grid(pool.drafts[0]!).categories[1]!.name).toBe('Category 2');
	});

	it('addGridCategory auto-suffixes on name collision', () => {
		pool.addGridCategory('draft_test', 'Cat A');
		expect(grid(pool.drafts[0]!).categories.at(-1)!.name).toBe('Cat A (2)');
	});

	it('removeGridCategory drops the category and recomputes progress', () => {
		for (const p of [100, 200]) pool.attachGridQuestion('draft_test', 0, p, 'q_existing');
		expect(pool.drafts[0]!.progress).toBeCloseTo(0.4);
		pool.removeGridCategory('draft_test', 0);
		expect(grid(pool.drafts[0]!).categories).toHaveLength(0);
		expect(pool.drafts[0]!.progress).toBe(0);
		expect(pool.drafts[0]!.status).toBe('unsaved_changes');
	});

	it('is a no-op for an unknown draft or out-of-bounds index', () => {
		expect(() => pool.addGridCategory('nope')).not.toThrow();
		expect(() => pool.removeGridCategory('draft_test', 99)).not.toThrow();
		expect(grid(pool.drafts[0]!).categories).toHaveLength(1);
	});

	it('refuses on a linear board', () => {
		pool.setDraftMode('draft_test', 'linear');
		pool.addGridCategory('draft_test');
		expect(linear(pool.drafts[0]!).items).toEqual([]);
	});
});

describe('PoolStore.addGridPoint / removeGridPoint', () => {
	it('addGridPoint uses max+100 as the next value', () => {
		pool.addGridPoint('draft_test');
		const points = Object.keys(grid(pool.drafts[0]!).categories[0]!.questions)
			.map(Number)
			.sort((a, b) => a - b);
		expect(points).toEqual([100, 200, 300, 400, 500, 600]);
	});

	it('addGridPoint seeds 100 when the board has no points yet', () => {
		pool.removeGridCategory('draft_test', 0);
		pool.addGridCategory('draft_test');
		pool.addGridPoint('draft_test');
		expect(grid(pool.drafts[0]!).categories[0]!.questions[100]).toBeNull();
	});

	it('addGridPoint adds an empty slot in every category', () => {
		pool.addGridCategory('draft_test');
		pool.addGridPoint('draft_test');
		for (const c of grid(pool.drafts[0]!).categories) {
			expect(c.questions[600]).toBeNull();
		}
	});

	it('removeGridPoint drops the point from every category', () => {
		pool.attachGridQuestion('draft_test', 0, 200, 'q_existing');
		pool.removeGridPoint('draft_test', 200);
		expect(grid(pool.drafts[0]!).categories[0]!.questions[200]).toBeUndefined();
		expect(pool.drafts[0]!.progress).toBe(0);
	});

	it('refuses on a linear board', () => {
		pool.setDraftMode('draft_test', 'linear');
		pool.addGridPoint('draft_test');
		pool.removeGridPoint('draft_test', 100);
		expect(linear(pool.drafts[0]!).items).toEqual([]);
	});
});

describe('PoolStore.setDraftMode', () => {
	it('wipes the board and resets progress', () => {
		pool.attachGridQuestion('draft_test', 0, 100, 'q_existing');
		pool.setDraftMode('draft_test', 'linear');
		expect(pool.drafts[0]!.board).toEqual({ mode: 'linear', items: [] });
		expect(pool.drafts[0]!.progress).toBe(0);
		expect(pool.drafts[0]!.status).toBe('incomplete');
	});

	it('is a no-op when the mode is unchanged', () => {
		pool.attachGridQuestion('draft_test', 0, 100, 'q_existing');
		const before = pool.drafts[0]!.board;
		pool.setDraftMode('draft_test', 'grid_quiz');
		expect(pool.drafts[0]!.board).toBe(before);
	});

	it('does nothing for an unknown draft', () => {
		expect(() => pool.setDraftMode('nope', 'linear')).not.toThrow();
	});
});

describe('PoolStore.referenceLocations', () => {
	it('finds every (draft, category, point) referencing a question', () => {
		pool.attachGridQuestion('draft_test', 0, 100, 'q_existing');
		pool.attachGridQuestion('draft_test', 0, 200, 'q_existing');
		const locs = pool.referenceLocations('q_existing');
		expect(locs).toHaveLength(2);
		expect(locs.every((l) => l.draft.id === 'draft_test')).toBe(true);
	});

	it('returns empty for an unreferenced question', () => {
		expect(pool.referenceLocations('q_existing')).toEqual([]);
	});

	it('finds linear references too', () => {
		pool.setDraftMode('draft_test', 'linear');
		pool.addLinearSlot('draft_test');
		pool.attachLinearSlot('draft_test', 0, 'q_existing');
		expect(pool.referencedBy('q_existing')).toHaveLength(1);
	});
});

describe('PoolStore.deleteQuestion', () => {
	it('refuses to delete a referenced question', () => {
		pool.attachGridQuestion('draft_test', 0, 100, 'q_existing');
		const result = pool.deleteQuestion('q_existing');
		expect(result.ok).toBe(false);
	});

	it('removes an unreferenced question', () => {
		const result = pool.deleteQuestion('q_existing');
		expect(result.ok).toBe(true);
		expect(pool.questions.find((q) => q.id === 'q_existing')).toBeUndefined();
	});

	it('returns not-found for unknown id', () => {
		const result = pool.deleteQuestion('q_nope');
		expect(result.ok).toBe(false);
	});
});

describe('grid progress (stored on draft.progress)', () => {
	it('is 0 for an empty draft', () => {
		expect(pool.drafts[0]!.progress).toBe(0);
	});

	it('is 1 when every cell is filled', () => {
		for (const p of [100, 200, 300, 400, 500]) {
			pool.attachGridQuestion('draft_test', 0, p, 'q_existing');
		}
		expect(pool.drafts[0]!.progress).toBe(1);
	});

	it('counts partial fills correctly', () => {
		pool.attachGridQuestion('draft_test', 0, 100, 'q_existing');
		pool.attachGridQuestion('draft_test', 0, 200, 'q_existing');
		expect(pool.drafts[0]!.progress).toBeCloseTo(0.4);
	});
});
