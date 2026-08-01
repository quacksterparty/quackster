<script lang="ts">
	import { GAMES, formatPlays, formatRelative, type GameSeed } from '$lib/data/seed';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { ScrollArea } from 'bits-ui';
	import { api } from '$lib/api';
	import Button from '$lib/components/Button.svelte';
	import Dialog from '$lib/components/Dialog.svelte';
	import TextInput from '$lib/components/TextInput.svelte';
	import { m } from '$lib/paraglide/messages';
	import { toast } from '$lib/toast.svelte';

	let query = $state('');
	let sort: 'popular' | 'recent' | 'rating' | 'title' = $state('popular');
	let density: 'comfy' | 'compact' = $state('comfy');
	let modeFilter = $state<string | null>(null);
	let categoryFilter = $state<string | null>(null);
	let audienceFilter = $state<string | null>(null);
	let regionFilter = $state<string | null>(null);
	let licenseFilter = $state<string | null>(null);
	let selectedTags = $state<string[]>([]);
	let selectedGame = $state<string | null>(null);

	let hostOpen = $state(false);
	let secret = $state('');

	const ALL_MODES = Array.from(new Set(GAMES.flatMap((g) => g.modes))).sort();
	const ALL_CATEGORIES = Array.from(new Set(GAMES.map((g) => g.category))).sort();
	const ALL_AUDIENCES = ['kids', 'family', 'adult', 'expert'] as const;
	const ALL_REGIONS = ['de', 'us', 'uk', 'global'] as const;
	const ALL_LICENSES = Array.from(new Set(GAMES.map((g) => g.license))).sort();
	const ALL_TAGS = Array.from(new Set(GAMES.flatMap((g) => g.tags))).sort();

	const counts = {
		mode: (m: string) => GAMES.filter((g) => g.modes.includes(m)).length,
		cat: (c: string) => GAMES.filter((g) => g.category === c).length,
		aud: (a: string) => GAMES.filter((g) => g.audience === a).length,
		reg: (r: string) => GAMES.filter((g) => g.region === r).length,
		lic: (l: string) => GAMES.filter((g) => g.license === l).length
	};

	const filtered = $derived.by(() => {
		const q = query.trim().toLowerCase();
		let out: GameSeed[] = GAMES.filter((g) => {
			if (q && !`${g.title} ${g.description} ${g.category}`.toLowerCase().includes(q)) return false;
			if (modeFilter && !g.modes.includes(modeFilter)) return false;
			if (categoryFilter && g.category !== categoryFilter) return false;
			if (audienceFilter && g.audience !== audienceFilter) return false;
			if (regionFilter && g.region !== regionFilter) return false;
			if (licenseFilter && g.license !== licenseFilter) return false;
			if (selectedTags.length && !selectedTags.every((t) => g.tags.includes(t))) return false;
			return true;
		});
		out = [...out];
		if (sort === 'popular') out.sort((a, b) => b.plays - a.plays);
		else if (sort === 'recent') out.sort((a, b) => +new Date(b.updated) - +new Date(a.updated));
		else if (sort === 'rating') out.sort((a, b) => b.rating - a.rating);
		else out.sort((a, b) => a.title.localeCompare(b.title));
		return out;
	});

	const activeFilterCount = $derived(
		[modeFilter, categoryFilter, audienceFilter, regionFilter, licenseFilter].filter(Boolean).length +
			selectedTags.length
	);

	function toggleTag(t: string) {
		selectedTags = selectedTags.includes(t)
			? selectedTags.filter((x) => x !== t)
			: [...selectedTags, t];
	}
	function resetFilters() {
		query = '';
		modeFilter = null;
		categoryFilter = null;
		audienceFilter = null;
		regionFilter = null;
		licenseFilter = null;
		selectedTags = [];
	}

	async function create() {
		if (!selectedGame) {
			toast.error(m.common_no_game_selected());
			return;
		}
		const result = await api.room.create({ secret, game_id: selectedGame });
		if (!result.ok) {
			toast.error(m.common_error_generic());
			return;
		}
		await goto(`/room/${result.value.join_code}`);
	}
</script>

<section class="room">
	<header class="head">
		<h2 class="room-title">{m.common_games()}</h2>
		<div class="head-right">
			<label class="search">
				<span class="sr">Search games</span>
				<input bind:value={query} placeholder="Search 250 games…" />
			</label>
			<label class="sort">
				Sort
				<select bind:value={sort}>
					<option value="popular">Most played</option>
					<option value="recent">Recently updated</option>
					<option value="rating">Highest rated</option>
					<option value="title">Title A–Z</option>
				</select>
			</label>
			<div class="density" role="group" aria-label="Density">
				<button class:active={density === 'comfy'} onclick={() => (density = 'comfy')}>Comfy</button>
				<button class:active={density === 'compact'} onclick={() => (density = 'compact')}>Compact</button>
			</div>
			<Button variant="primary" size="sm" onclick={() => goto(resolve('/curate', {}))}>
				+ Create game
			</Button>
		</div>
	</header>

	<div class="layout">
		<aside class="rail" aria-label="Filters">
			<div class="rail-head">
				<h2>Filters</h2>
				{#if activeFilterCount}
					<button class="clear" onclick={resetFilters}>Clear ({activeFilterCount})</button>
				{/if}
			</div>
			<section class="facet">
				<h3>Modes</h3>
				<ul>
					<li>
						<button class:active={!modeFilter} onclick={() => (modeFilter = null)}>
							<span>All</span><span class="ct">{GAMES.length}</span>
						</button>
					</li>
					{#each ALL_MODES as m (m)}
						<li>
							<button class:active={modeFilter === m} onclick={() => (modeFilter = m)}>
								<span>{m.replace('_', ' ')}</span><span class="ct">{counts.mode(m)}</span>
							</button>
						</li>
					{/each}
				</ul>
			</section>
			<section class="facet">
				<h3>Category</h3>
				<select bind:value={categoryFilter}>
					<option value={null}>All categories</option>
					{#each ALL_CATEGORIES as c (c)}
						<option value={c}>{c} ({counts.cat(c)})</option>
					{/each}
				</select>
			</section>
			<section class="facet">
				<h3>Audience</h3>
				<div class="chips">
					{#each ALL_AUDIENCES as a (a)}
						<button
							class="chip"
							class:active={audienceFilter === a}
							onclick={() => (audienceFilter = audienceFilter === a ? null : a)}
						>
							{a} <span class="ct-sm">{counts.aud(a)}</span>
						</button>
					{/each}
				</div>
			</section>
			<section class="facet">
				<h3>Region</h3>
				<div class="chips">
					{#each ALL_REGIONS as r (r)}
						<button
							class="chip"
							class:active={regionFilter === r}
							onclick={() => (regionFilter = regionFilter === r ? null : r)}
						>
							{r.toUpperCase()} <span class="ct-sm">{counts.reg(r)}</span>
						</button>
					{/each}
				</div>
			</section>
			<section class="facet">
				<h3>License</h3>
				<select bind:value={licenseFilter}>
					<option value={null}>Any license</option>
					{#each ALL_LICENSES as l (l)}
						<option value={l}>{l} ({counts.lic(l)})</option>
					{/each}
				</select>
			</section>
			<section class="facet">
				<h3>Tags</h3>
				<div class="tag-cloud">
					{#each ALL_TAGS as t (t)}
						<button class="tag" class:active={selectedTags.includes(t)} onclick={() => toggleTag(t)}>
							{t}
						</button>
					{/each}
				</div>
			</section>
		</aside>

		<ScrollArea.Root class="games-scroll" type="hover">
			<ScrollArea.Viewport class="games-viewport">
				<div class="meta-row">
					<span><strong>{filtered.length}</strong> games</span>
					{#if activeFilterCount}<span>· {activeFilterCount} active filter{activeFilterCount === 1 ? '' : 's'}</span>{/if}
				</div>
				<div class="grid" class:compact={density === 'compact'}>
					{#each filtered as g (g.id)}
						<button
							class="tile"
							class:selected={selectedGame === g.id}
							onclick={() => (selectedGame = g.id)}
							aria-pressed={selectedGame === g.id}
						>
							<div class="t-head">
								<div class="t-title">{g.title}</div>
								<div class="t-cat">{g.category}</div>
							</div>
							<p class="t-desc">{g.description}</p>
							<div class="t-meta">
								<span class="m">{(g.modes[0] ?? '').replace('_', ' ')}{#if g.modes.length > 1} +{g.modes.length - 1}{/if}</span>
								<span class="dot">·</span>
								<span class="m">{g.questions}q</span>
								<span class="dot">·</span>
								<span class="m">{formatPlays(g.plays)} plays</span>
							</div>
							<div class="t-tags">
								{#each g.tags.slice(0, 3) as t (t)}
									<span class="t-tag">{t}</span>
								{/each}
							</div>
						</button>
					{/each}
				</div>
			</ScrollArea.Viewport>
			<ScrollArea.Scrollbar orientation="vertical" class="games-bar">
				<ScrollArea.Thumb class="games-thumb" />
			</ScrollArea.Scrollbar>
		</ScrollArea.Root>
	</div>

	{#if selectedGame}
		{@const g = GAMES.find((x) => x.id === selectedGame)}
		{#if g}
			<footer class="dock">
				<div>
					<strong>{g.title}</strong> · {g.modes.map((m) => m.replace('_', ' ')).join(', ')} · {g.questions} questions · updated {formatRelative(g.updated)}
				</div>
				<Button size="lg" onclick={() => (hostOpen = true)}>{m.common_host()}</Button>
			</footer>
		{/if}
	{/if}
</section>

<Dialog bind:open={hostOpen} title={m.common_host()}>
	<form
		onsubmit={(e) => {
			e.preventDefault();
			void create();
		}}
	>
		<TextInput bind:value={secret} placeholder="Secret" />
	</form>
	<Button size="lg" onclick={create}>{m.common_create_room()}</Button>
</Dialog>

<style>
	.room {
		height: 100%;
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		width: 100%;
		max-width: 80rem;
		margin-inline: auto;
		padding: var(--space-4);
		box-sizing: border-box;
	}
	.head {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		gap: var(--space-3);
		flex-wrap: wrap;
	}
	.room-title {
		flex: none;
		font-family: var(--font-heading);
		margin: 0;
	}
	.head-right {
		display: flex;
		gap: var(--space-2);
		align-items: center;
		flex-wrap: wrap;
	}
	.search {
		min-width: 16rem;
		display: block;
	}
	.search input {
		width: 100%;
		padding: var(--space-1) var(--space-3);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		background: var(--bg-surface);
		color: var(--color-text);
		font-family: var(--font-body);
		font-size: calc(0.9rem * var(--font-scale));
	}
	.sort {
		display: flex;
		gap: var(--space-1);
		align-items: center;
		font-size: calc(0.8rem * var(--font-scale));
		color: var(--color-text-muted);
	}
	.sort select {
		padding: var(--space-1) var(--space-2);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		background: var(--bg-surface);
		color: var(--color-text);
	}
	.density {
		display: flex;
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		overflow: hidden;
	}
	.density button {
		padding: var(--space-1) var(--space-2);
		background: var(--bg-surface);
		color: var(--color-text);
		border: none;
		font-size: calc(0.8rem * var(--font-scale));
		cursor: pointer;
	}
	.density button.active {
		background: var(--color-primary);
		color: var(--color-text-inverse);
	}
	.sr {
		position: absolute;
		left: -9999px;
	}
	.layout {
		flex: 1;
		min-height: 0;
		display: grid;
		grid-template-columns: 16rem 1fr;
		gap: var(--space-3);
	}
	.rail {
		overflow-y: auto;
		padding: var(--space-3);
		background: var(--bg-surface);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.rail-head {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
	}
	.rail-head h2 {
		margin: 0;
		font-family: var(--font-heading);
		font-size: calc(0.95rem * var(--font-scale));
	}
	.clear {
		font-size: calc(0.75rem * var(--font-scale));
		background: transparent;
		border: none;
		color: var(--color-primary);
		cursor: pointer;
	}
	.facet h3 {
		font-family: var(--font-body);
		font-size: calc(0.7rem * var(--font-scale));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		margin: 0 0 var(--space-1) 0;
	}
	.facet ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.facet button:not(.chip):not(.tag) {
		display: flex;
		justify-content: space-between;
		width: 100%;
		text-align: left;
		padding: var(--space-1) var(--space-2);
		background: transparent;
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text);
		font-family: var(--font-body);
		font-size: calc(0.8rem * var(--font-scale));
		cursor: pointer;
		text-transform: capitalize;
	}
	.facet button:not(.chip):not(.tag):hover {
		background: var(--bg-primary);
	}
	.facet button:not(.chip):not(.tag).active {
		background: var(--color-primary);
		color: var(--color-text-inverse);
	}
	.ct {
		font-family: var(--font-mono);
		font-size: calc(0.7rem * var(--font-scale));
		opacity: 0.7;
	}
	.ct-sm {
		font-family: var(--font-mono);
		font-size: calc(0.6rem * var(--font-scale));
		opacity: 0.7;
	}
	.facet select {
		width: 100%;
		padding: var(--space-1) var(--space-2);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-sm);
		background: var(--bg-surface);
		color: var(--color-text);
		font-family: var(--font-body);
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
	}
	.chip {
		padding: 2px var(--space-2);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-full);
		background: var(--bg-surface);
		color: var(--color-text);
		font-size: calc(0.7rem * var(--font-scale));
		cursor: pointer;
	}
	.chip.active {
		border-color: var(--color-primary);
		color: var(--color-primary);
	}
	.tag-cloud {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
	}
	.tag {
		padding: 2px var(--space-2);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-full);
		background: var(--bg-surface);
		color: var(--color-text-muted);
		font-size: calc(0.65rem * var(--font-scale));
		cursor: pointer;
	}
	.tag.active {
		background: var(--color-accent);
		color: var(--color-text-inverse);
		border-color: var(--color-accent);
	}
	:global(.games-scroll) {
		flex: 1;
		min-height: 0;
	}
	:global(.games-viewport) {
		height: 100%;
		padding-top: 6px;
		padding-left: 6px;
		padding-right: var(--space-2);
		padding-bottom: 6rem;
	}
	.meta-row {
		color: var(--color-text-muted);
		font-size: calc(0.85rem * var(--font-scale));
		margin-bottom: var(--space-2);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr));
		gap: var(--space-3);
		align-content: start;
	}
	.grid.compact {
		grid-template-columns: repeat(auto-fill, minmax(14rem, 1fr));
	}
	.tile {
		text-align: left;
		background: var(--bg-surface);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		padding: var(--space-3);
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		cursor: pointer;
		color: var(--color-text);
		font-family: inherit;
	}
	.tile:hover {
		border-color: var(--color-primary);
	}
	.tile.selected {
		border: 2px solid var(--color-primary);
		padding: calc(var(--space-3) - 1px);
	}
	.t-head {
		display: flex;
		justify-content: space-between;
		gap: var(--space-2);
		align-items: baseline;
	}
	.t-title {
		font-family: var(--font-heading);
		font-size: calc(0.95rem * var(--font-scale));
	}
	.t-cat {
		font-size: calc(0.65rem * var(--font-scale));
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		flex-shrink: 0;
	}
	.t-desc {
		margin: 0;
		font-size: calc(0.8rem * var(--font-scale));
		color: var(--color-text-muted);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.t-meta {
		display: flex;
		gap: var(--space-1);
		align-items: center;
		font-size: calc(0.7rem * var(--font-scale));
		color: var(--color-text-muted);
		flex-wrap: wrap;
	}
	.t-meta .m {
		text-transform: capitalize;
	}
	.dot {
		opacity: 0.5;
	}
	.t-tags {
		display: flex;
		gap: 4px;
		flex-wrap: wrap;
	}
	.t-tag {
		font-size: calc(0.6rem * var(--font-scale));
		padding: 1px var(--space-2);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-full);
		color: var(--color-text-muted);
	}
	.dock {
		position: fixed;
		bottom: 1.5rem;
		left: 50%;
		transform: translateX(-50%);
		width: min(56rem, calc(100vw - 2rem));
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-3) var(--space-4);
		background: var(--bg-surface-elevated);
		border: 2px solid var(--color-primary);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-lg);
		z-index: 20;
		gap: var(--space-3);
	}
	:global(.games-bar) {
		position: absolute;
		width: 0.5rem;
		--bits-scroll-area-thumb-width: 100%;
	}
	:global(.games-thumb) {
		background: var(--color-text-muted);
		border-radius: var(--radius-full);
	}
	@media (max-width: 800px) {
		.layout {
			grid-template-columns: 1fr;
		}
		.rail {
			max-height: 10rem;
		}
	}
</style>
