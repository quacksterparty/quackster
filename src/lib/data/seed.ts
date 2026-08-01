/**
 * Mock seed for curate + room browse. Will be replaced by API-backed
 * pool/draft/game stores when the runtime lands (quackster-26).
 *
 * - 250 questions in the pool (independent entities, referenced by ID)
 * - 4 drafts; each board cell references a question by ID
 * - 250 games for the host/browse surface
 *
 * Seeded RNG keeps the list stable across reloads.
 */

export type QuestionKind = 'text' | 'numeric' | 'order';
export type QuestionVariant = 'multiple_choice' | 'true_false' | 'open' | 'numeric_input' | 'range';
export type QuestionStatus = 'draft' | 'named' | 'referenced' | 'deprecated';

export type PoolQuestion = {
	id: string;
	kind: QuestionKind;
	prompt: string;
	answer: string;
	answerNumeric?: number;
	unit?: string;
	numericInput: { tolerance: number } | null;
	range: { min: number; max: number; step: number; tolerance: number } | null;
	explanation?: string;
	choices?: { id: string; text: string; correct: boolean }[];
	tags: string[];
	license: string;
	defaultLang: 'de' | 'en';
	status: QuestionStatus;
	variants: QuestionVariant[];
	previousIds?: string[];
	createdAt: string;
};

export type BoardCellRef = { questionId: string; variant?: QuestionVariant };

export type BoardCategory = {
	name: string;
	questions: Record<number, BoardCellRef | null>;
};

export type CurateDraft = {
	id: string;
	title: string;
	language: 'de' | 'en';
	audience: 'kids' | 'family' | 'adult' | 'expert';
	status: 'incomplete' | 'unsaved_changes' | 'saved' | 'invalid';
	progress: number;
	updated: string;
	do_not_delete?: boolean;
	board: { categories: BoardCategory[] };
	rules: { buzz_policy: string; scoring_mode: string; judge: string };
};

export type GameSeed = {
	id: string;
	slug: string;
	title: string;
	description: string;
	modes: string[];
	category: string;
	audience: 'kids' | 'family' | 'adult' | 'expert';
	region: 'de' | 'us' | 'uk' | 'global';
	language: 'de' | 'en' | 'mixed';
	tags: string[];
	license: string;
	questions: number;
	plays: number;
	rating: number;
	updated: string;
	trending?: boolean;
	featured?: boolean;
};

// ── seeded RNG so seed is stable ──
function mulberry32(a: number) {
	return function () {
		a |= 0;
		a = (a + 0x6d2b79f5) | 0;
		let t = a;
		t = Math.imul(t ^ (t >>> 15), t | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}
const rand = mulberry32(42);
const pick = <T>(arr: readonly T[]): T => arr[Math.floor(rand() * arr.length)] as T;
const pickN = <T>(arr: readonly T[], n: number): T[] => {
	const c = [...arr];
	const out: T[] = [];
	for (let i = 0; i < n && c.length; i++) out.push(c.splice(Math.floor(rand() * c.length), 1)[0] as T);
	return out;
};

const TAGS = [
	'subject:geography',
	'subject:science',
	'subject:history',
	'subject:music',
	'subject:film',
	'subject:sports',
	'subject:literature',
	'subject:food',
	'subject:technology',
	'subject:nature',
	'subject:language',
	'subject:games',
	'difficulty:general',
	'difficulty:hard',
	'difficulty:expert',
	'region:global',
	'region:de',
	'region:us',
	'region:uk',
	'format:visual',
	'format:audio',
	'format:short',
	'warning:spicy'
] as const;
const LICENSES = ['CC-BY-4.0', 'CC-BY-SA-4.0', 'CC0-1.0', 'ODbL-1.0'] as const;
const VARIANTS: Record<QuestionKind, QuestionVariant[]> = {
	text: ['multiple_choice', 'true_false', 'open'],
	numeric: ['multiple_choice', 'numeric_input', 'range'],
	order: []
};

const TEMPLATES_DE: Array<[string, string, QuestionKind, QuestionVariant[], [number, number, number]?]> = [
	['In welchem Jahr fiel die Berliner Mauer?', '1989', 'numeric', ['numeric_input', 'multiple_choice', 'range'], [1980, 2000, 1]],
	['Wie heißt die Hauptstadt von Australien?', 'Canberra', 'text', ['open', 'multiple_choice']],
	['Welches Element hat das Symbol Au?', 'Gold', 'text', ['open', 'multiple_choice']],
	['Wer schrieb "Faust"?', 'Goethe', 'text', ['open', 'multiple_choice']],
	['Welche Farbe hat das Smaragd?', 'Grün', 'text', ['multiple_choice', 'true_false']],
	['Wie viele Bundesländer hat Deutschland?', '16', 'numeric', ['numeric_input', 'multiple_choice', 'range'], [14, 18, 1]],
	['Welcher Planet ist der Sonne am nächsten?', 'Merkur', 'text', ['multiple_choice', 'open']],
	['Wer malte die Mona Lisa?', 'Leonardo da Vinci', 'text', ['open', 'multiple_choice']],
	['Welches Meer liegt östlich von Griechenland?', 'Ägäis', 'text', ['open', 'multiple_choice']],
	['Wie heißt der höchste Berg der Welt?', 'Mount Everest', 'text', ['open', 'multiple_choice']],
	['In welchem Jahr startete die Apollo 11 Mission?', '1969', 'numeric', ['numeric_input', 'multiple_choice', 'range'], [1960, 1980, 1]],
	['Welches Land hat die meisten Einwohner?', 'Indien', 'text', ['open', 'multiple_choice']],
	['Wie heißt der längste Fluss Europas?', 'Wolga', 'text', ['open', 'multiple_choice']],
	['Welches Metall ist flüssig bei Raumtemperatur?', 'Quecksilber', 'text', ['open', 'multiple_choice']],
	['Wer komponierte die 9. Symphonie?', 'Beethoven', 'text', ['open', 'multiple_choice']]
];
const TEMPLATES_EN: Array<[string, string, QuestionKind, QuestionVariant[], [number, number, number]?]> = [
	['In which year did the Berlin Wall fall?', '1989', 'numeric', ['numeric_input', 'multiple_choice', 'range'], [1980, 2000, 1]],
	['What is the capital of Australia?', 'Canberra', 'text', ['open', 'multiple_choice']],
	['Which element has the symbol Au?', 'Gold', 'text', ['open', 'multiple_choice']],
	['Who wrote "Faust"?', 'Goethe', 'text', ['open', 'multiple_choice']],
	['What color is an emerald?', 'Green', 'text', ['multiple_choice', 'true_false']],
	['How many US states are there?', '50', 'numeric', ['numeric_input', 'multiple_choice', 'range'], [48, 52, 1]],
	['Which planet is closest to the Sun?', 'Mercury', 'text', ['multiple_choice', 'open']],
	['Who painted the Mona Lisa?', 'Leonardo da Vinci', 'text', ['open', 'multiple_choice']],
	['Which sea lies east of Greece?', 'Aegean', 'text', ['open', 'multiple_choice']],
	['What is the highest mountain in the world?', 'Mount Everest', 'text', ['open', 'multiple_choice']],
	['In which year did Apollo 11 launch?', '1969', 'numeric', ['numeric_input', 'multiple_choice', 'range'], [1960, 1980, 1]],
	['Which country has the most people?', 'India', 'text', ['open', 'multiple_choice']],
	['What is the longest river in Europe?', 'Volga', 'text', ['open', 'multiple_choice']],
	['Which metal is liquid at room temperature?', 'Mercury', 'text', ['open', 'multiple_choice']],
	['Who composed the 9th Symphony?', 'Beethoven', 'text', ['open', 'multiple_choice']]
];

function makeQuestion(i: number, referencedIds: Set<string>): PoolQuestion {
	const lang: 'de' | 'en' = rand() < 0.55 ? 'de' : 'en';
	const tpl = pick(lang === 'de' ? TEMPLATES_DE : TEMPLATES_EN);
	const [prompt, answer, kind, variants, rangeTpl] = tpl;
	const subject = prompt.split(' ').slice(0, 3).join('_').toLowerCase().replace(/[^a-z0-9_]/g, '');
	const baseId = `q_${lang}_${subject}_${i.toString().padStart(3, '0')}`;
	const id = referencedIds.has(baseId) ? `${baseId}_${(rand() * 1e6) | 0}` : baseId;
	const choices =
		variants.includes('multiple_choice') || kind === 'text'
			? [
					{ id: 'a', text: answer, correct: true },
					{ id: 'b', text: `Option B ${i}`, correct: false },
					{ id: 'c', text: `Option C ${i}`, correct: false },
					{ id: 'd', text: `Option D ${i}`, correct: false }
				]
			: null;
	const numericFields =
		kind === 'numeric' && rangeTpl
			? {
					answerNumeric: Number(answer),
					numericInput: { tolerance: 0 } as { tolerance: number },
					range: { min: rangeTpl[0], max: rangeTpl[1], step: rangeTpl[2], tolerance: 0 } as {
						min: number;
						max: number;
						step: number;
						tolerance: number;
					}
				}
			: { numericInput: null, range: null };
	return {
		id,
		kind,
		prompt,
		answer,
		...(choices ? { choices } : {}),
		...numericFields,
		tags: pickN(TAGS, 2 + Math.floor(rand() * 3)),
		license: pick(LICENSES),
		defaultLang: lang,
		status: 'named',
		variants: variants.length ? variants : VARIANTS[kind],
		createdAt: new Date(Date.now() - Math.floor(rand() * 365) * 86_400_000).toISOString()
	};
}

const CATEGORIES_DE = ['Geografie', 'Wissenschaft', 'Geschichte', 'Musik', 'Film', 'Sport'];
const CATEGORIES_EN = ['Geography', 'Science', 'History', 'Music', 'Film', 'Sports'];
const POINTS = [100, 200, 300, 400, 500];

function makeDraft(
	id: string,
	title: string,
	progress: number,
	lang: 'de' | 'en',
	aud: CurateDraft['audience'],
	status: CurateDraft['status'],
	allQuestions: PoolQuestion[],
	do_not_delete = false
): CurateDraft {
	const cats = lang === 'de' ? CATEGORIES_DE : CATEGORIES_EN;
	const board: { categories: BoardCategory[] } = { categories: cats.map((name) => ({ name, questions: {} })) };
	for (const cat of board.categories) {
		for (const p of POINTS) {
			const shouldFill = rand() < progress;
			if (shouldFill) {
				const q = pick(allQuestions.filter((q) => q.defaultLang === lang));
				cat.questions[p] = { questionId: q.id, variant: pick(q.variants) };
			} else {
				cat.questions[p] = null;
			}
		}
	}
	return {
		id,
		title,
		language: lang,
		audience: aud,
		status,
		progress,
		updated: new Date(Date.now() - Math.floor(rand() * 14) * 86_400_000).toISOString(),
		do_not_delete,
		board,
		rules: {
			buzz_policy: pick(['open_floor', 'turn_order', 'broadcast']),
			scoring_mode: pick(['first_correct', 'cascading', 'flat']),
			judge: pick(['auto', 'moderator'])
		}
	};
}

// Build question pool first; drafts will reference by ID.
const _referenced = new Set<string>();
export const QUESTIONS: PoolQuestion[] = Array.from({ length: 250 }, (_, i) => makeQuestion(i, _referenced));
export const DRAFTS: CurateDraft[] = [
	makeDraft('draft_abc123', 'Pub Quiz Night #14', 0.7, 'de', 'adult', 'unsaved_changes', QUESTIONS),
	makeDraft('draft_def456', 'Klassenraum 7b — Europa', 0.34, 'de', 'family', 'incomplete', QUESTIONS),
	makeDraft('draft_ghi789', 'Friday Trivia Classics', 1.0, 'en', 'family', 'saved', QUESTIONS, true),
	makeDraft('draft_jkl012', '90s Music Throwdown', 0.18, 'en', 'adult', 'invalid', QUESTIONS)
];

// ── games for the host/browse surface ──
const GAME_MODES = ['grid_quiz', 'classic', 'battle_royale', 'survival', 'music_quiz', 'quiz_duel', 'millionaire', 'higher_lower', 'buzzer'];
const GAME_CATEGORIES = [
	'General Knowledge', 'Pop Culture', 'Music', 'Film & TV', 'Science', 'History', 'Geography',
	'Sports', 'Literature', 'Food & Drink', 'Technology', 'Nature', 'Language', 'Video Games'
] as const;
const GAME_AUDIENCES: GameSeed['audience'][] = ['kids', 'family', 'adult', 'expert'];
const GAME_REGIONS: GameSeed['region'][] = ['de', 'us', 'uk', 'global'];
const GAME_TAGS = [
	'beginner-friendly', 'trivia-night', 'pub-quiz', 'classroom', 'team-building',
	'90s', '2000s', 'deep-cut', 'visual', 'audio-required', 'no-media', 'short', 'long-form'
] as const;
const GAME_SUBJECTS_DE = [
	'Deutsche Popkultur', 'Europäische Hauptstädte', '90er Musik', 'Filmzitate', 'Wissenschaft Basics',
	'Weltgeschichte', 'Tierwelt', 'Bratengerichte', 'Computerspiele', 'Berühmte Erfinder',
	'Fußball-Legenden', 'Chemie für Anfänger', 'Astronomie', 'Mythologie der Antike'
];
const GAME_SUBJECTS_EN = [
	'US Pop Culture', 'World Capitals', '90s Music', 'Famous Movie Quotes', 'Science Basics',
	'World History', 'Wildlife', 'Comfort Food', 'Classic Video Games', 'Famous Inventors',
	'Soccer Legends', 'Chemistry 101', 'Astronomy', 'Greek Mythology'
];
const GAME_VARIANTS = ['Runde 1', 'Warm-up', 'Finale', 'Bonus Round', 'Sprint', 'Lightning'];

function makeGame(i: number): GameSeed {
	const lang: 'de' | 'en' | 'mixed' = rand() < 0.55 ? 'de' : rand() < 0.85 ? 'en' : 'mixed';
	const subject = pick(lang === 'de' ? GAME_SUBJECTS_DE : GAME_SUBJECTS_EN);
	const title = `${subject} — ${pick(GAME_VARIANTS)}`;
	const modeCount = 1 + Math.floor(rand() * 3);
	const modes = pickN(GAME_MODES, modeCount);
	const questions = 12 + Math.floor(rand() * 60);
	const plays = Math.floor(rand() * 50000);
	const rating = +(3 + rand() * 2).toFixed(1);
	const ageDays = Math.floor(rand() * 720);
	return {
		id: `game_${i.toString().padStart(4, '0')}_${subject.toLowerCase().replace(/[^a-z0-9]+/g, '_').slice(0, 24)}`,
		slug: `game-${i.toString().padStart(4, '0')}-${subject.toLowerCase().replace(/[^a-z0-9]+/g, '-').slice(0, 24)}`,
		title,
		description:
			lang === 'de'
				? `Ein ${pick(GAME_CATEGORIES)}-Quiz mit ${questions} Fragen für ${pick(GAME_AUDIENCES)}.`
				: `A ${pick(GAME_CATEGORIES).toLowerCase()} quiz with ${questions} questions.`,
		modes,
		category: pick(GAME_CATEGORIES),
		audience: pick(GAME_AUDIENCES),
		region: pick(GAME_REGIONS),
		language: lang,
		tags: pickN(GAME_TAGS, 2 + Math.floor(rand() * 3)),
		license: pick(LICENSES),
		questions,
		plays,
		rating,
		updated: new Date(Date.now() - ageDays * 86_400_000).toISOString(),
		trending: rand() < 0.1,
		featured: rand() < 0.05
	};
}

export const GAMES: GameSeed[] = Array.from({ length: 250 }, (_, i) => makeGame(i));

// ── helpers ──
export function formatPlays(n: number): string {
	if (n < 1000) return `${n}`;
	if (n < 10_000) return `${(n / 1000).toFixed(1)}k`;
	if (n < 1_000_000) return `${Math.round(n / 1000)}k`;
	return `${(n / 1_000_000).toFixed(1)}M`;
}
export function formatRelative(iso: string): string {
	const days = Math.floor((Date.now() - new Date(iso).getTime()) / 86_400_000);
	if (days < 1) return 'today';
	if (days === 1) return 'yesterday';
	if (days < 30) return `${days}d ago`;
	if (days < 365) return `${Math.floor(days / 30)}mo ago`;
	return `${Math.floor(days / 365)}y ago`;
}
