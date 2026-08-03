/**
 * Mock seed for curate + room browse. Will be replaced by API-backed
 * pool/draft stores when the runtime lands (quackster-26).
 *
 * - 250 questions in the pool (independent entities, referenced by ID)
 * - 4 drafts; each board cell references a question by ID
 *
 * Seeded RNG keeps the list stable across reloads.
 */

export type QuestionKind = 'text' | 'numeric' | 'order';
export type QuestionVariant = 'multiple_choice' | 'true_false' | 'open' | 'numeric_input' | 'range';
export type QuestionStatus = 'draft' | 'named' | 'referenced' | 'deprecated';

export interface PoolQuestion {
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
}

export interface BoardCellRef {
	questionId: string;
	variant?: QuestionVariant;
}

export interface BoardCategory {
	name: string;
	questions: Record<number, BoardCellRef | null>;
}

export interface CurateDraft {
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
}

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
	for (let i = 0; i < n && c.length; i++)
		out.push(c.splice(Math.floor(rand() * c.length), 1)[0] as T);
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

const TEMPLATES_DE: [string, string, QuestionKind, QuestionVariant[], [number, number, number]?][] =
	[
		[
			'In welchem Jahr fiel die Berliner Mauer?',
			'1989',
			'numeric',
			['numeric_input', 'multiple_choice', 'range'],
			[1980, 2000, 1]
		],
		['Wie heißt die Hauptstadt von Australien?', 'Canberra', 'text', ['open', 'multiple_choice']],
		['Welches Element hat das Symbol Au?', 'Gold', 'text', ['open', 'multiple_choice']],
		['Wer schrieb "Faust"?', 'Goethe', 'text', ['open', 'multiple_choice']],
		['Welche Farbe hat das Smaragd?', 'Grün', 'text', ['multiple_choice', 'true_false']],
		[
			'Wie viele Bundesländer hat Deutschland?',
			'16',
			'numeric',
			['numeric_input', 'multiple_choice', 'range'],
			[14, 18, 1]
		],
		['Welcher Planet ist der Sonne am nächsten?', 'Merkur', 'text', ['multiple_choice', 'open']],
		['Wer malte die Mona Lisa?', 'Leonardo da Vinci', 'text', ['open', 'multiple_choice']],
		['Welches Meer liegt östlich von Griechenland?', 'Ägäis', 'text', ['open', 'multiple_choice']],
		['Wie heißt der höchste Berg der Welt?', 'Mount Everest', 'text', ['open', 'multiple_choice']],
		[
			'In welchem Jahr startete die Apollo 11 Mission?',
			'1969',
			'numeric',
			['numeric_input', 'multiple_choice', 'range'],
			[1960, 1980, 1]
		],
		['Welches Land hat die meisten Einwohner?', 'Indien', 'text', ['open', 'multiple_choice']],
		['Wie heißt der längste Fluss Europas?', 'Wolga', 'text', ['open', 'multiple_choice']],
		[
			'Welches Metall ist flüssig bei Raumtemperatur?',
			'Quecksilber',
			'text',
			['open', 'multiple_choice']
		],
		['Wer komponierte die 9. Symphonie?', 'Beethoven', 'text', ['open', 'multiple_choice']]
	];
const TEMPLATES_EN: [string, string, QuestionKind, QuestionVariant[], [number, number, number]?][] =
	[
		[
			'In which year did the Berlin Wall fall?',
			'1989',
			'numeric',
			['numeric_input', 'multiple_choice', 'range'],
			[1980, 2000, 1]
		],
		['What is the capital of Australia?', 'Canberra', 'text', ['open', 'multiple_choice']],
		['Which element has the symbol Au?', 'Gold', 'text', ['open', 'multiple_choice']],
		['Who wrote "Faust"?', 'Goethe', 'text', ['open', 'multiple_choice']],
		['What color is an emerald?', 'Green', 'text', ['multiple_choice', 'true_false']],
		[
			'How many US states are there?',
			'50',
			'numeric',
			['numeric_input', 'multiple_choice', 'range'],
			[48, 52, 1]
		],
		['Which planet is closest to the Sun?', 'Mercury', 'text', ['multiple_choice', 'open']],
		['Who painted the Mona Lisa?', 'Leonardo da Vinci', 'text', ['open', 'multiple_choice']],
		['Which sea lies east of Greece?', 'Aegean', 'text', ['open', 'multiple_choice']],
		[
			'What is the highest mountain in the world?',
			'Mount Everest',
			'text',
			['open', 'multiple_choice']
		],
		[
			'In which year did Apollo 11 launch?',
			'1969',
			'numeric',
			['numeric_input', 'multiple_choice', 'range'],
			[1960, 1980, 1]
		],
		['Which country has the most people?', 'India', 'text', ['open', 'multiple_choice']],
		['What is the longest river in Europe?', 'Volga', 'text', ['open', 'multiple_choice']],
		['Which metal is liquid at room temperature?', 'Mercury', 'text', ['open', 'multiple_choice']],
		['Who composed the 9th Symphony?', 'Beethoven', 'text', ['open', 'multiple_choice']]
	];

function makeQuestion(i: number, referencedIds: Set<string>): PoolQuestion {
	const lang: 'de' | 'en' = rand() < 0.55 ? 'de' : 'en';
	const tpl = pick(lang === 'de' ? TEMPLATES_DE : TEMPLATES_EN);
	const [prompt, answer, kind, variants, rangeTpl] = tpl;
	const subject = prompt
		.split(' ')
		.slice(0, 3)
		.join('_')
		.toLowerCase()
		.replace(/[^a-z0-9_]/g, '');
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
					numericInput: { tolerance: 0 },
					range: { min: rangeTpl[0], max: rangeTpl[1], step: rangeTpl[2], tolerance: 0 }
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
	const board: { categories: BoardCategory[] } = {
		categories: cats.map((name) => ({ name, questions: {} }))
	};
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
export const QUESTIONS: PoolQuestion[] = Array.from({ length: 250 }, (_, i) =>
	makeQuestion(i, _referenced)
);
export const DRAFTS: CurateDraft[] = [
	makeDraft('draft_abc123', 'Pub Quiz Night #14', 0.7, 'de', 'adult', 'unsaved_changes', QUESTIONS),
	makeDraft(
		'draft_def456',
		'Klassenraum 7b — Europa',
		0.34,
		'de',
		'family',
		'incomplete',
		QUESTIONS
	),
	makeDraft(
		'draft_ghi789',
		'Friday Trivia Classics',
		1.0,
		'en',
		'family',
		'saved',
		QUESTIONS,
		true
	),
	makeDraft('draft_jkl012', '90s Music Throwdown', 0.18, 'en', 'adult', 'invalid', QUESTIONS)
];

// ── helpers ──
export function formatRelative(iso: string): string {
	const days = Math.floor((Date.now() - new Date(iso).getTime()) / 86_400_000);
	if (days < 1) return 'today';
	if (days === 1) return 'yesterday';
	if (days < 30) return `${days}d ago`;
	if (days < 365) return `${Math.floor(days / 30)}mo ago`;
	return `${Math.floor(days / 365)}y ago`;
}
