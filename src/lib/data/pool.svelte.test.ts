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
		pool.attachQuestion('draft_test', 0, 100, q.id);
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

describe('PoolStore.attachQuestion / detachQuestion', () => {
	it('fills the cell and bumps progress', () => {
		pool.attachQuestion('draft_test', 0, 100, 'q_existing');
		expect(pool.drafts[0]!.board.categories[0]!.questions[100]).toEqual({
			questionId: 'q_existing'
		});
		expect(pool.drafts[0]!.progress).toBeGreaterThan(0);
		expect(pool.drafts[0]!.status).toBe('unsaved_changes');
	});

	it('marks status saved when fully filled', () => {
		for (const p of [100, 200, 300, 400, 500]) {
			pool.attachQuestion('draft_test', 0, p, 'q_existing');
		}
		expect(pool.drafts[0]!.progress).toBe(1);
		expect(pool.drafts[0]!.status).toBe('saved');
	});

	it('detach clears the cell', () => {
		pool.attachQuestion('draft_test', 0, 100, 'q_existing');
		pool.detachQuestion('draft_test', 0, 100);
		expect(pool.drafts[0]!.board.categories[0]!.questions[100]).toBeNull();
	});

	it('does nothing for an unknown draft', () => {
		const before = pool.drafts[0]!.board.categories[0]!.questions[100];
		pool.attachQuestion('nope', 0, 100, 'q_existing');
		expect(pool.drafts[0]!.board.categories[0]!.questions[100]).toBe(before);
	});
});

describe('PoolStore.referenceLocations', () => {
	it('finds every (draft, category, point) referencing a question', () => {
		pool.attachQuestion('draft_test', 0, 100, 'q_existing');
		pool.attachQuestion('draft_test', 0, 200, 'q_existing');
		const locs = pool.referenceLocations('q_existing');
		expect(locs).toHaveLength(2);
		expect(locs.every((l) => l.draft.id === 'draft_test')).toBe(true);
	});

	it('returns empty for an unreferenced question', () => {
		expect(pool.referenceLocations('q_existing')).toEqual([]);
	});
});

describe('PoolStore.deleteQuestion', () => {
	it('refuses to delete a referenced question', () => {
		pool.attachQuestion('draft_test', 0, 100, 'q_existing');
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

describe('PoolStore.computeDraftProgress', () => {
	it('is 0 for an empty draft', () => {
		expect(pool.computeDraftProgress(pool.drafts[0]!)).toBe(0);
	});

	it('is 1 when every cell is filled', () => {
		for (const p of [100, 200, 300, 400, 500]) {
			pool.attachQuestion('draft_test', 0, p, 'q_existing');
		}
		expect(pool.computeDraftProgress(pool.drafts[0]!)).toBe(1);
	});

	it('counts partial fills correctly', () => {
		pool.attachQuestion('draft_test', 0, 100, 'q_existing');
		pool.attachQuestion('draft_test', 0, 200, 'q_existing');
		expect(pool.computeDraftProgress(pool.drafts[0]!)).toBe(0.4);
	});
});
