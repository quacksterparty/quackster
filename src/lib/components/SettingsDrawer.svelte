<script lang="ts">
	import Drawer from '$lib/components/Drawer.svelte';
	import {
		themes,
		setTheme,
		setSystemTheme,
		themeState,
		type ThemeId
	} from '$lib/themes/index.svelte';
	import { locales, setLocale, type Locale } from '$lib/paraglide/runtime';
	import { m } from '$lib/paraglide/messages';
	import { currentLocale, themeLabel } from '$lib/i18n.svelte';
	import { room } from '$lib/room.svelte';

	let { open = $bindable(false) }: { open?: boolean } = $props();

	function pickTheme(id: ThemeId) {
		setTheme(id);
	}
	function pickSystemTheme() {
		setSystemTheme();
	}
	function pickLang(loc: Locale) {
		void setLocale(loc);
		room.send?.({ kind: 'SetLocale', locale: loc });
	}
</script>

<Drawer bind:open title={m.common_settings()}>
	<div class="drawer-section">
		<h3 class="section-label">🎨 {m.theme_label()}</h3>
		<div class="chip-row">
			<button class="chip" class:chip-active={themeState.usingSystem} onclick={pickSystemTheme}>
				<span class="chip-emoji">🖥️</span>
				{m.theme_system()}
			</button>
			{#each Object.values(themes) as t (t.id)}
				<button
					class="chip"
					class:chip-active={!themeState.usingSystem && themeState.id === t.id}
					onclick={() => pickTheme(t.id)}
				>
					<span class="chip-emoji">{t.emojis[0]}</span>
					{themeLabel(t.id)}
				</button>
			{/each}
		</div>
	</div>

	<div class="drawer-section">
		<h3 class="section-label">🌐 {m.common_language()}</h3>
		<div class="chip-row">
			{#each locales as loc (loc)}
				<button
					class="chip"
					class:chip-active={currentLocale() === loc}
					onclick={() => pickLang(loc)}
				>
					{loc.toUpperCase()}
				</button>
			{/each}
		</div>
	</div>
</Drawer>

<style>
	.drawer-section {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.section-label {
		font-family: var(--font-heading);
		font-size: calc(0.75rem * var(--font-scale));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		margin: 0;
	}
	.chip-row {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	.chip {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-2) var(--space-3);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-full);
		background: var(--bg-surface);
		color: var(--color-text);
		font-family: var(--font-body);
		font-size: calc(0.875rem * var(--font-scale));
		cursor: pointer;
	}
	.chip-emoji {
		font-size: 1.1em;
	}
	.chip-active {
		border-color: var(--color-primary);
		color: var(--color-primary);
		font-weight: 600;
	}
</style>
