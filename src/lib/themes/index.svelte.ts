import { browser } from '$app/environment';

export type ThemeId =
	| 'modern'
	| 'modern-dark'
	| 'retro'
	| 'medieval'
	| 'neon'
	| 'chalkboard'
	| 'kawaii'
	| 'western'
	| 'wizard';

export interface ThemeMeta {
	id: ThemeId;
	/** Emojis floating in the background */
	emojis: string[];
}

export const themes: Record<ThemeId, ThemeMeta> = {
	modern: {
		id: 'modern',
		emojis: ['🦆', '❓', '💡', '🧠', '🎯', '✨', '📚', '🏆']
	},
	'modern-dark': {
		id: 'modern-dark',
		emojis: ['🌙', '🦆', '❓', '💡', '🧠', '🎯', '✨', '🔭']
	},
	retro: {
		id: 'retro',
		emojis: ['👾', '🕹️', '👾', '⬛', '🟦', '🍄', '⭐', '🟡']
	},
	medieval: {
		id: 'medieval',
		emojis: ['⚔️', '🛡️', '🏰', '🐉', '👑', '📜', '🏹', '🔮']
	},
	neon: {
		id: 'neon',
		emojis: ['🌆', '⚡', '💊', '🔫', '🤖', '🧬', '💿', '🔮']
	},
	chalkboard: {
		id: 'chalkboard',
		emojis: ['✏️', '📐', '📖', '🧪', '🌍', '🗑️', '🎒', '🔬']
	},
	kawaii: {
		id: 'kawaii',
		emojis: ['🌸', '🦋', '🌈', '🐱', '🧁', '🎀', '☁️', '💕']
	},
	western: {
		id: 'western',
		emojis: ['🤠', '🐎', '🌵', '🔫', '💰', '🏜️', '🐄', '🎻']
	},
	wizard: {
		id: 'wizard',
		emojis: ['🧙', '🔮', '✨', '📖', '⚡', '🦉', '⚗️', '🪄']
	}
};

const STORAGE_KEY = 'quackster-theme';

/** Reactive theme id — write via `setTheme` / `setSystemTheme`, read everywhere else. */
export const themeState = $state<{ id: ThemeId; usingSystem: boolean }>({
	id: 'modern',
	usingSystem: true
});

function systemPrefersDark(): boolean {
	if (!browser) return false;
	return window.matchMedia('(prefers-color-scheme: dark)').matches;
}

function applyToDom(id: ThemeId): void {
	if (!browser) return;
	document.documentElement.setAttribute('data-theme', id === 'modern' ? '' : id);
}

export function getTheme(): ThemeId {
	return themeState.id;
}

export function isUsingSystemTheme(): boolean {
	return themeState.usingSystem;
}

export function setTheme(id: ThemeId): void {
	if (!browser) return;
	themeState.id = id;
	themeState.usingSystem = false;
	localStorage.setItem(STORAGE_KEY, id);
	applyToDom(id);
}

export function setSystemTheme(): void {
	if (!browser) return;
	themeState.usingSystem = true;
	localStorage.removeItem(STORAGE_KEY);
	const id = systemPrefersDark() ? 'modern-dark' : 'modern';
	themeState.id = id;
	applyToDom(id);
}

export function hasStoredTheme(): boolean {
	if (!browser) return false;
	return localStorage.getItem(STORAGE_KEY) !== null;
}

export function initTheme(): void {
	if (!browser) return;
	const stored = localStorage.getItem(STORAGE_KEY) as ThemeId | null;
	if (stored && stored in themes) {
		themeState.id = stored;
		themeState.usingSystem = false;
	} else {
		themeState.id = systemPrefersDark() ? 'modern-dark' : 'modern';
		themeState.usingSystem = true;
	}
	applyToDom(themeState.id);
}