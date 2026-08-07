import type { Component } from 'svelte';
import type { CurateDraft, DraftBoard, QuestionVariant } from '$lib/data/seed';

/** Registry of gamemodes the curate UI knows how to edit. */
export type ModeId = 'grid_quiz' | 'linear';

/**
 * A gamemode descriptor is the contract between a board shape and the curate UI.
 * Each mode ships its own BoardComponent, cell-shape accessors, and progress logic.
 * Adding a new mode = drop a new descriptor in `gamemodes/index.ts`.
 */
export interface GamemodeDescriptor<TCell = unknown> {
	id: ModeId;
	title: string;
	description: string;

	/** The board renderer. Reads `draft.board` and calls `onSelectCell` on click. */
	BoardComponent: Component<{
		draft: CurateDraft;
		activeCell: TCell | null;
		onSelectCell: (cell: TCell | null) => void;
	}>;

	/** Factory for a fresh empty board of this mode. */
	emptyBoard: () => DraftBoard;

	/** Fill ratio 0..1 for the preview progress bar. */
	computeProgress: (draft: CurateDraft) => number;

	/** Attach a question to the cell at `position`. */
	attach: (
		draftId: string,
		position: TCell,
		questionId: string,
		variant?: QuestionVariant
	) => void;

	/** Detach the cell at `position` (sets to null, does not remove the slot). */
	detach: (draftId: string, position: TCell) => void;

	/** One-line summary for the preview pane ("3 categories · 12/25 cells"). */
	statusLine: (draft: CurateDraft) => string;

	/** Human-readable label for the selected cell ("Geography · 200 pts" / "Question 5"). */
	cellLabel: (cell: TCell, draft: CurateDraft) => string;

	/** Cell at `position` currently bound to a question? Used by editor to show "Attached to …". */
	getCellRef: (cell: TCell, draft: CurateDraft) => { questionId: string; variant?: QuestionVariant } | null;
}
