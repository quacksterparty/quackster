/**
 * Curate-local view types + adapters between the auto-generated wire types
 * (`$lib/bindings/*`) and the flat shapes the Svelte components work against.
 *
 * The wire types are nested (`content.variants.multiple_choice.choices[]`) and
 * discriminated by `kind` per the Rust enum; the UI components want a flat
 * shape (`prompt`, `answer`, `choices[]`) for ergonomic input bindings. These
 * adapters translate both ways.
 *
 * `pool.init()` calls `api.questions.list()` / `api.games.list()`, hands each
 * item through `wireTo*` to populate the in-memory store. `pool.saveAll()`
 * walks dirty items and sends them through `*ToWire` to the API.
 */

import type {
	Question,
	Choice,
	TextVariants,
	NumericVariants,
	OrderItem
} from '$lib/bindings/Questions';
import type { GameConfig as ApiGameConfig, GameMode } from '$lib/bindings/GameConfigs';
import type { Tag as ApiTag } from '$lib/bindings/Tags';

// ── curate-local types (what the components use) ───────────────────────────

export type QuestionKind = 'text' | 'numeric' | 'order';
export type QuestionVariant = 'multiple_choice' | 'true_false' | 'open' | 'numeric_input' | 'range';

/** Mirror of `ContentStatus` plus UI-only `incomplete` (board not yet built). */
export type QuestionStatus = 'draft' | 'named' | 'referenced' | 'deprecated' | 'incomplete';

export interface BoardCellRef {
	questionId: string;
	variant?: QuestionVariant | undefined;
}

export interface BoardCategory {
	name: string;
	questions: Record<number, BoardCellRef | null>;
}

export interface GridQuizBoard {
	mode: 'grid_quiz';
	categories: BoardCategory[];
}

export interface LinearBoard {
	mode: 'linear';
	items: (BoardCellRef | null)[];
}

export type DraftBoard = GridQuizBoard | LinearBoard;

export interface PoolQuestion {
	id: string;
	kind: QuestionKind;
	prompt: string;
	/** Text answer (kind === 'text'). */
	answer: string;
	/** Numeric answer (kind === 'numeric'). */
	answerNumeric?: number | undefined;
	unit?: string | undefined;
	numericInput: { tolerance: number } | null;
	range: { min: number; max: number; step: number; tolerance: number } | null;
	explanation?: string | undefined;
	choices?: { id: string; text: string; correct: boolean }[] | undefined;
	tags: string[];
	license: string;
	defaultLang: 'de' | 'en';
	status: QuestionStatus;
	variants: QuestionVariant[];
	previousIds?: string[] | undefined;
	createdAt: string;
}

export type DraftStatus = 'incomplete' | 'unsaved_changes' | 'saved' | 'invalid';

export interface CurateDraft {
	id: string;
	title: string;
	language: 'de' | 'en';
	audience: 'kids' | 'family' | 'adult' | 'expert';
	status: DraftStatus;
	progress: number;
	updated: string;
	do_not_delete?: boolean | undefined;
	board: DraftBoard;
	/** Only the three rules the UI exposes — full set sent on save. */
	rules: { buzz_policy: string; scoring_mode: string; judge: string };
	/** True for drafts we created locally and haven't POSTed yet. */
	pendingCreate?: boolean | undefined;
}

// ── small util ─────────────────────────────────────────────────────────────

/** "today" / "Nd ago" — used by ActionBar / question detail for ISO timestamps. */
export function formatRelative(iso: string): string {
	const days = Math.floor((Date.now() - new Date(iso).getTime()) / 86_400_000);
	if (days < 1) return 'today';
	if (days === 1) return 'yesterday';
	if (days < 30) return `${days}d ago`;
	if (days < 365) return `${Math.floor(days / 30)}mo ago`;
	return `${Math.floor(days / 365)}y ago`;
}

function variantNames(variants: {
	multiple_choice?: unknown;
	open?: unknown;
	true_false?: unknown;
	numeric_input?: unknown;
	range?: unknown;
}): QuestionVariant[] {
	const out: QuestionVariant[] = [];
	if (variants.multiple_choice) out.push('multiple_choice');
	if (variants.open) out.push('open');
	if (variants.true_false) out.push('true_false');
	if (variants.numeric_input) out.push('numeric_input');
	if (variants.range) out.push('range');
	return out;
}

function wireStatusToLocal(status: Question['status']): QuestionStatus {
	if (status === 'deprecated') return 'deprecated';
	return 'named'; // both 'published' and 'draft' on disk are 'named' in UI until referenced
}

// ── wire → local adapters ──────────────────────────────────────────────────

export function wireToPool(q: Question): PoolQuestion {
	const createdAt = new Date().toISOString();
	const base = {
		id: q.id,
		tags: q.tags,
		license: q.license ?? 'CC-BY-4.0',
		defaultLang: (q.lang_locked ?? q.content.default_lang) as 'de' | 'en',
		status: wireStatusToLocal(q.status),
		previousIds: undefined,
		createdAt
	};

	if (q.kind === 'text') {
		const c = q.content;
		const variants = variantNames(c.variants);
		return {
			...base,
			kind: 'text',
			prompt: c.prompt.text,
			answer: c.answer,
			explanation: c.explanation ?? undefined,
			choices: c.variants.multiple_choice?.choices.map((cc) => ({
				id: cc.id,
				text: cc.text,
				correct: cc.correct === true
			})),
			numericInput: null,
			range: null,
			variants
		};
	}

	if (q.kind === 'numeric') {
		const c = q.content;
		const variants = variantNames(c.variants);
		return {
			...base,
			kind: 'numeric',
			prompt: c.prompt.text,
			answer: '',
			answerNumeric: c.answer,
			unit: c.unit ?? undefined,
			explanation: c.explanation ?? undefined,
			choices: c.variants.multiple_choice?.choices.map((cc) => ({
				id: cc.id,
				text: cc.text,
				correct: cc.correct === true
			})),
			numericInput: c.variants.numeric_input
				? { tolerance: c.variants.numeric_input.tolerance }
				: null,
			range: c.variants.range
				? {
						min: c.variants.range.min,
						max: c.variants.range.max,
						step: c.variants.range.step,
						tolerance: c.variants.range.tolerance
					}
				: null,
			variants
		};
	}

	// 'order'
	const c = q.content;
	return {
		...base,
		kind: 'order',
		prompt: c.prompt.text,
		answer: c.items.map((it) => it.text).join(' › '),
		explanation: c.explanation ?? undefined,
		numericInput: null,
		range: null,
		variants: [],
		choices: c.items.map((it) => ({
			id: it.id,
			text: `${it.position}. ${it.text}`,
			correct: true
		}))
	};
}

export function wireToDraft(g: ApiGameConfig): CurateDraft {
	const first = g.games[0];
	const mode = first?.mode.kind ?? 'grid_quiz';
	let board: DraftBoard;
	if (mode === 'grid_quiz' && first?.mode.kind === 'grid_quiz') {
		const gridMode = first.mode;
		board = {
			mode: 'grid_quiz',
			categories: gridMode.board.categories.map((cat) => {
				const questions: Record<number, BoardCellRef | null> = {};
				if (cat.question_ids) {
					for (const [point, cell] of Object.entries(cat.question_ids)) {
						questions[Number(point)] = {
							questionId: cell.id,
							variant: cell.variant ?? undefined
						};
					}
				}
				for (const p of gridMode.board.points) {
					if (!(p in questions)) questions[p] = null;
				}
				return { name: cat.name, questions };
			})
		};
	} else if (first?.mode.kind === 'linear' && first.mode.questions.source === 'questions') {
		board = {
			mode: 'linear',
			items: first.mode.questions.question_ids.map((id) => ({ questionId: id }))
		};
	} else {
		// Pack / filter sources — curate-local can't render those yet. Start empty linear.
		board = { mode: 'linear', items: [] };
	}
	return {
		id: g.id,
		title: g.title,
		language: 'en',
		audience: 'family',
		status: g.status === 'draft' ? 'incomplete' : 'saved',
		progress: 0, // recomputed by the consumer
		updated: new Date().toISOString(),
		board,
		rules: {
			buzz_policy: first?.rules.buzz_policy ?? 'open_floor',
			scoring_mode: first?.rules.scoring_mode ?? 'first_correct',
			judge: first?.rules.judge ?? 'auto'
		}
	};
}

// ── local → wire adapters ──────────────────────────────────────────────────

export function poolToWire(q: PoolQuestion): Question {
	const base = {
		id: q.id,
		tags: q.tags,
		deprecated: null,
		lang_locked: q.defaultLang,
		license: (q.license ?? 'CC-BY-4.0') as Question['license'],
		sources: null,
		preferred_variant: null,
		status: (q.status === 'deprecated'
			? 'deprecated'
			: q.status === 'draft'
				? 'draft'
				: 'published') as Question['status']
	};
	if (q.kind === 'text') {
		const v: TextVariants = { multiple_choice: null, open: null, true_false: null };
		if (q.variants.includes('multiple_choice') && q.choices) {
			v.multiple_choice = {
				choices: q.choices.map((c): Choice => ({
					id: c.id,
					text: c.text,
					correct: c.correct,
					media: null
				}))
			};
		}
		if (q.variants.includes('open')) {
			v.open = { accepted: q.answer ? [q.answer] : [], normalize: null };
		}
		if (q.variants.includes('true_false')) {
			v.true_false = { correct: true };
		}
		return {
			kind: 'text',
			...base,
			content: {
				default_lang: q.defaultLang,
				prompt: { text: q.prompt, media: null },
				answer: q.answer,
				explanation: q.explanation ?? null,
				variants: v
			}
		};
	}
	if (q.kind === 'numeric') {
		const v: NumericVariants = { multiple_choice: null, numeric_input: null, range: null };
		if (q.variants.includes('multiple_choice') && q.choices) {
			v.multiple_choice = {
				choices: q.choices.map((c): Choice => ({
					id: c.id,
					text: c.text,
					correct: c.correct,
					media: null
				}))
			};
		}
		if (q.variants.includes('numeric_input') && q.numericInput) {
			v.numeric_input = { tolerance: q.numericInput.tolerance };
		}
		if (q.variants.includes('range') && q.range) {
			v.range = {
				min: q.range.min,
				max: q.range.max,
				step: q.range.step,
				tolerance: q.range.tolerance
			};
		}
		return {
			kind: 'numeric',
			...base,
			content: {
				default_lang: q.defaultLang,
				prompt: { text: q.prompt, media: null },
				answer: q.answerNumeric ?? 0,
				unit: q.unit ?? null,
				explanation: q.explanation ?? null,
				variants: v
			}
		};
	}
	// 'order' — choices[] holds the items
	const items: OrderItem[] = (q.choices ?? []).map((c, i) => {
		const m = /^([^.\s]+)\.\s*(.*)$/.exec(c.text);
		return { id: m ? m[1]! : c.id, text: m ? m[2]! : c.text, position: i + 1, media: null };
	});
	return {
		kind: 'order',
		...base,
		content: {
			default_lang: q.defaultLang,
			prompt: { text: q.prompt, media: null },
			items,
			explanation: q.explanation ?? null
		}
	};
}

function collectPoints(d: GridQuizBoard): number[] {
	const set = new Set<number>();
	for (const c of d.categories) for (const p of Object.keys(c.questions)) set.add(Number(p));
	return [...set].sort((a, b) => a - b);
}

export function draftToWire(d: CurateDraft): ApiGameConfig {
	const mode: GameMode =
		d.board.mode === 'grid_quiz'
			? {
					kind: 'grid_quiz',
					grid_rules: { picker_mode: 'winner_picks', reveal_auto_advance_secs: null },
					board: {
						points: collectPoints(d.board),
						categories: d.board.categories.map((cat) => ({
							name: cat.name,
							question_ids: Object.fromEntries(
								Object.entries(cat.questions)
									.filter(([, cell]) => cell !== null)
									.map(([p, cell]) => [p, { id: cell!.questionId, variant: null }])
							),
							pack_ref: null,
							filter: null
						})),
						difficulty_map: null
					}
				}
			: {
					kind: 'linear',
					questions: {
						source: 'questions',
						question_ids: d.board.items.filter((i) => i !== null).map((i) => i!.questionId)
					}
				};

	return {
		id: d.id,
		title: d.title,
		description: '',
		auto_advance: false,
		games: [
			{
				title: 'Round 1',
				rules: {
					buzz_policy: d.rules
						.buzz_policy as ApiGameConfig['games'][number]['rules']['buzz_policy'],
					scoring_mode: d.rules
						.scoring_mode as ApiGameConfig['games'][number]['rules']['scoring_mode'],
					lockout_policy: 'none',
					steal_policy: 'none',
					judge: d.rules.judge as ApiGameConfig['games'][number]['rules']['judge'],
					question_timer_secs: 30,
					answer_timer_secs: 15
				},
				mode
			}
		],
		status: d.status === 'saved' ? 'published' : 'draft'
	};
}

export function wireToTag(t: ApiTag): { id: string; label: string; defaultLang: string } {
	return { id: t.id, label: t.label, defaultLang: t.default_lang };
}
