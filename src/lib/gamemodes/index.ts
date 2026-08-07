import { gridQuiz } from './grid_quiz';
import { linear } from './linear';
import type { GamemodeDescriptor, ModeId } from './types';

// ponytail: cast through `any` because Svelte's `Component<Props>` generic is
// invariant in Props. Each mode owns a different cell shape; the registry is
// intentionally lossy. Components narrow `unknown` back to their own TCell.
const GAMEMODES = {
	grid_quiz: gridQuiz as GamemodeDescriptor<unknown>,
	linear: linear as GamemodeDescriptor<unknown>
} satisfies Record<ModeId, GamemodeDescriptor<unknown>>;

export function getGamemode(id: ModeId): GamemodeDescriptor<unknown> {
	return GAMEMODES[id];
}

export function listGamemodes(): GamemodeDescriptor<unknown>[] {
	return Object.values(GAMEMODES);
}

export type { GamemodeDescriptor, ModeId };
