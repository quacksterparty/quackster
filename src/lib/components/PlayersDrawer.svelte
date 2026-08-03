<script lang="ts">
	import Drawer from '$lib/components/Drawer.svelte';
	import PlayerAvatar from '$lib/components/PlayerAvatar.svelte';
	import { DropdownMenu } from 'bits-ui';
	import { m } from '$lib/paraglide/messages';
	import { room, has } from '$lib/room.svelte';
	import { sortedByScore } from '$lib/playerUi';
	import type { Grant } from '$lib/bindings/Grants';

	let { open = $bindable(false) }: { open?: boolean } = $props();

	let tab = $state<'players' | 'scoreboard'>('players');

	const phase = $derived.by(() => {
		const stage = room.view?.stage;
		return stage?.kind === 'GridQuiz' ? stage.phase : undefined;
	});
	const sortedPlayers = $derived(sortedByScore(room.view?.players ?? {}));

	const allGrants: Grant[] = ['Play', 'Present', 'Moderate'];
	const grantLabels: Record<Grant, () => string> = {
		Play: m.room_grant_play,
		Present: m.room_grant_present,
		Moderate: m.room_grant_moderate
	};

	function kick(player: string) {
		room.send?.({ kind: 'Kick', player });
	}
	function toggleGrant(player: string, current: Grant[], g: Grant) {
		const grants = current.includes(g) ? current.filter((x) => x !== g) : [...current, g];
		room.send?.({ kind: 'Grant', player, grants });
	}
</script>

<Drawer bind:open title={m.common_players_and_scoreboard()}>
	{#snippet header()}
		<div class="tabs" role="tablist">
			<button
				class="tab"
				role="tab"
				aria-selected={tab === 'players'}
				class:tab-active={tab === 'players'}
				onclick={() => (tab = 'players')}>{m.common_players()}</button
			>
			<button
				class="tab"
				role="tab"
				aria-selected={tab === 'scoreboard'}
				class:tab-active={tab === 'scoreboard'}
				onclick={() => (tab = 'scoreboard')}>{m.common_scoreboard()}</button
			>
		</div>
	{/snippet}

	{#if tab === 'players'}
		<div class="drawer-section">
			<ul class="player-list">
				{#each Object.entries(room.view?.players ?? {}) as [player, view] (player)}
					<li class="player-row">
						<PlayerAvatar name={player} />
						<span class="player-name">
							{player}
							{#if room.player === player}
								<span class="player-you">({m.common_you()})</span>
							{/if}
						</span>
						{#if view.grants.includes('Moderate')}
							<span class="mod-badge" title={m.room_mod_actions()}>🛡️ {m.common_mod()}</span>
						{/if}
						{#if has('Moderate')}
							<DropdownMenu.Root>
								<DropdownMenu.Trigger
									class="bits-menu-trigger"
									aria-label={m.room_player_actions({ name: player })}
									title={m.room_player_actions({ name: player })}
								>
									<svg class="dots-icon" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
										<circle cx="5" cy="12" r="2" />
										<circle cx="12" cy="12" r="2" />
										<circle cx="19" cy="12" r="2" />
									</svg>
								</DropdownMenu.Trigger>
								<DropdownMenu.Portal>
									<DropdownMenu.Content class="bits-menu-list" align="end" sideOffset={4}>
										{#each allGrants as g (g)}
											<DropdownMenu.CheckboxItem
												class="bits-menu-item"
												checked={view.grants.includes(g)}
												closeOnSelect={false}
												onCheckedChange={() => toggleGrant(player, view.grants, g)}
											>
												<span class="bits-menu-check">{view.grants.includes(g) ? '✓' : ''}</span>
												{grantLabels[g]()}
											</DropdownMenu.CheckboxItem>
										{/each}
										{#if player !== room.player}
											<DropdownMenu.Item
												class="bits-menu-item bits-menu-danger"
												onSelect={() => kick(player)}
											>
												<span class="bits-menu-check">✕</span>
												{m.room_kick_player({ name: player })}
											</DropdownMenu.Item>
										{/if}
									</DropdownMenu.Content>
								</DropdownMenu.Portal>
							</DropdownMenu.Root>
						{/if}
					</li>
				{/each}
			</ul>
		</div>
	{:else}
		<div class="drawer-section">
			{#if phase === 'lobby'}
				<p class="empty-state">{m.room_game_not_started()}</p>
			{:else}
				<ul class="player-list">
					{#each sortedPlayers as [player, view], i (player)}
						<li class="player-row">
							<span class="rank">{i + 1}</span>
							<PlayerAvatar name={player} />
							<span class="player-name">
								{player}
								{#if room.player === player}
									<span class="player-you">({m.common_you()})</span>
								{/if}
							</span>
							<span class="score">{view.score}</span>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}
</Drawer>

<style>
	.drawer-section {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	/* ── bits-ui global (Content portaled to body — scoped selectors can't reach them) ── */
	:global(.bits-menu-trigger) {
		margin-left: auto;
		width: 1.5rem;
		height: 1.5rem;
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--color-text-muted);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}
	:global(.bits-menu-trigger:hover),
	:global(.bits-menu-trigger[data-state='open']) {
		background: var(--bg-muted);
		color: var(--color-text);
	}
	:global(.bits-menu-trigger .dots-icon) {
		width: 1.1rem;
		height: 1.1rem;
	}
	:global(.bits-menu-list) {
		z-index: 60;
		min-width: 10rem;
		padding: var(--space-1);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		background: var(--bg-surface);
		box-shadow: 0 4px 12px rgb(0 0 0 / 0.15);
		display: flex;
		flex-direction: column;
		outline: none;
	}
	:global(.bits-menu-item) {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-sm);
		color: var(--color-text);
		font-family: var(--font-body);
		font-size: calc(0.875rem * var(--font-scale));
		cursor: pointer;
		white-space: nowrap;
		outline: none;
	}
	:global(.bits-menu-item[data-highlighted]) {
		background: var(--bg-muted);
	}
	:global(.bits-menu-check) {
		width: 1em;
		flex-shrink: 0;
	}
	:global(.bits-menu-danger) {
		color: var(--color-danger);
	}
	:global(.bits-menu-danger[data-highlighted]) {
		background: var(--color-danger);
		color: var(--color-text-inverse);
	}
	.player-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.player-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-3);
		border: var(--border-width) var(--border-style) var(--border-color);
		border-radius: var(--radius-md);
		background: var(--bg-surface);
	}
	.player-name {
		font-family: var(--font-body);
		font-size: calc(0.95rem * var(--font-scale));
	}
	.player-you {
		margin-left: 0.3em;
		color: var(--color-text-muted);
		opacity: 0.8;
		font-weight: 400;
	}
	.mod-badge {
		margin-left: auto;
		padding: 0 0.4rem;
		border-radius: var(--radius-full);
		border: var(--border-width) var(--border-style) var(--color-accent);
		color: var(--color-accent);
		background: var(--bg-surface);
		font-family: var(--font-body);
		font-size: calc(0.7rem * var(--font-scale));
		font-weight: 600;
		white-space: nowrap;
	}
	.tabs {
		display: flex;
		gap: var(--space-1);
		flex: 1;
	}
	.tab {
		flex: 1;
		padding: var(--space-2) var(--space-3);
		border: none;
		border-bottom: 2px solid transparent;
		background: transparent;
		color: var(--color-text-muted);
		font-family: var(--font-body);
		font-size: calc(0.875rem * var(--font-scale));
		font-weight: 600;
		cursor: pointer;
	}
	.tab-active {
		color: var(--color-primary);
		border-bottom-color: var(--color-primary);
	}
	.rank {
		font-family: var(--font-heading);
		font-weight: 700;
		color: var(--color-text-muted);
		min-width: 1.5rem;
		font-size: calc(0.9rem * var(--font-scale));
	}
	.score {
		margin-left: auto;
		font-family: var(--font-heading);
		font-weight: 700;
		font-size: calc(1rem * var(--font-scale));
	}
	.empty-state {
		margin: 0;
		padding: var(--space-6) var(--space-3);
		text-align: center;
		color: var(--color-text-muted);
		font-family: var(--font-body);
		font-size: calc(0.9rem * var(--font-scale));
	}
</style>
