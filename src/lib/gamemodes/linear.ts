import LinearBoard from '$lib/components/curate/LinearBoard.svelte';
import { pool } from '$lib/data/pool.svelte';
import type { CurateDraft, LinearBoard as LinearBoardShape } from '$lib/wire';
import type { GamemodeDescriptor } from './types';

/** Linear cell is just the slot index. */
export type LinearCell = number;

function isLinear(draft: CurateDraft): draft is CurateDraft & { board: LinearBoardShape } {
	return draft.board.mode === 'linear';
}

export const linear: GamemodeDescriptor<LinearCell> = {
	id: 'linear',
	title: 'Linear Quiz',
	description: 'Kahoot-style ordered sequence of questions.',
	BoardComponent: LinearBoard,
	emptyBoard: () => ({ mode: 'linear', items: [] }),
	computeProgress(draft) {
		if (!isLinear(draft)) return 0;
		const total = draft.board.items.length;
		if (!total) return 0;
		const filled = draft.board.items.filter(Boolean).length;
		return filled / total;
	},
	attach(draftId, slotIdx, questionId, variant) {
		pool.attachLinearSlot(draftId, slotIdx, questionId, variant);
	},
	detach(draftId, slotIdx) {
		pool.detachLinearSlot(draftId, slotIdx);
	},
	statusLine(draft) {
		if (!isLinear(draft)) return '';
		const total = draft.board.items.length;
		const filled = draft.board.items.filter(Boolean).length;
		return `${filled}/${total} questions`;
	},
	cellLabel(cell) {
		return `Question ${cell + 1}`;
	},
	getCellRef(cell, draft) {
		if (cell === null || cell === undefined || !isLinear(draft)) return null;
		return draft.board.items[cell as number] ?? null;
	}
};
