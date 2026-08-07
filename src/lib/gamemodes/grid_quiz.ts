import BoardCanvas from '$lib/components/curate/BoardCanvas.svelte';
import { pool } from '$lib/data/pool.svelte';
import type { CurateDraft, GridQuizBoard } from '$lib/data/seed';
import type { GamemodeDescriptor } from './types';

export interface GridQuizCell {
	categoryIdx: number;
	point: number;
}

const POINTS_PER_CATEGORY = 5;

function isGrid(draft: CurateDraft): draft is CurateDraft & { board: GridQuizBoard } {
	return draft.board.mode === 'grid_quiz';
}

export const gridQuiz: GamemodeDescriptor<GridQuizCell> = {
	id: 'grid_quiz',
	title: 'Grid Quiz',
	description: 'Jeopardy-style NxM board of categories × point values.',
	BoardComponent: BoardCanvas,
	emptyBoard: () => ({ mode: 'grid_quiz', categories: [] }),
	computeProgress(draft) {
		if (!isGrid(draft)) return 0;
		const board = draft.board;
		const total = board.categories.length * POINTS_PER_CATEGORY;
		let filled = 0;
		for (const c of board.categories) {
			for (const p of Object.keys(c.questions)) {
				if (c.questions[+p]) filled++;
			}
		}
		return total ? filled / total : 0;
	},
	attach(draftId, pos, questionId, variant) {
		pool.attachGridQuestion(draftId, pos.categoryIdx, pos.point, questionId, variant);
	},
	detach(draftId, pos) {
		pool.detachGridQuestion(draftId, pos.categoryIdx, pos.point);
	},
	statusLine(draft) {
		if (!isGrid(draft)) return '';
		const cats = draft.board.categories.length;
		const total = cats * POINTS_PER_CATEGORY;
		const filled = draft.board.categories.reduce(
			(acc, c) => acc + Object.values(c.questions).filter(Boolean).length,
			0
		);
		return `${cats} categories · ${filled}/${total} cells`;
	},
	cellLabel(cell, draft) {
		if (!isGrid(draft) || !cell) return '';
		const cat = draft.board.categories[cell.categoryIdx]?.name ?? '—';
		return `${cell.point} pts · ${cat}`;
	},
	getCellRef(cell, draft) {
		if (!cell || !isGrid(draft)) return null;
		const slot = draft.board.categories[cell.categoryIdx]?.questions[cell.point] ?? null;
		return slot ?? null;
	}
};
