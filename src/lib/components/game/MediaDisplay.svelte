<script lang="ts">
	import type { MediaView } from '$lib/bindings/Protocol';
	import { has } from '$lib/room.svelte';

	let { media, playNonce = 0 }: { media: MediaView; playNonce?: number } = $props();

	let el: HTMLMediaElement | undefined = $state();

	// Mod-triggered playback: the nonce bumps on each PlayMedia; only Present
	// screens react. Initialized from the first snapshot so a reconnect
	// mid-question doesn't fire a stale play (initial value on purpose).
	// svelte-ignore state_referenced_locally
	let lastNonce = playNonce;
	$effect(() => {
		if (playNonce === lastNonce) return;
		lastNonce = playNonce;
		if (!el || !has('Present')) return;
		el.currentTime = 0;
		// rejection = browser blocked autoplay (no gesture yet); native
		// controls are still there to start it manually
		el.play().catch(() => {
			/* noop */
		});
	});
</script>

{#if media.kind === 'Image'}
	<img
		class="visual"
		src={media.src}
		alt={media.alt ?? ''}
		width={media.width}
		height={media.height}
	/>
{:else if media.kind === 'Video'}
	<!-- quiz media has no caption tracks -->
	<!-- svelte-ignore a11y_media_has_caption -->
	<video
		class="visual"
		bind:this={el}
		src={media.src}
		controls
		width={media.width}
		height={media.height}
	></video>
{:else}
	<audio class="audio" bind:this={el} src={media.src} controls></audio>
{/if}

<style>
	.visual {
		max-width: min(40rem, 100%);
		max-height: 40vh;
		width: auto;
		height: auto;
		border-radius: var(--radius-md);
	}
	.audio {
		width: min(28rem, 100%);
	}
</style>
