<script lang="ts">
	import { browser } from '$app/environment';
	import { onMount, onDestroy } from 'svelte';
	import { streamStore } from '$lib/stores/stream.store.svelte';
	import { info } from '$lib/utils/logger';
	import GameStreamView from '$lib/components/game/stream/game-stream-view.svelte';

	const state = $derived(streamStore.state);

	onMount(() => {
		if (browser) {
			info('StreamWindow: Mounted, initializing stream store for stream window.');
			streamStore.initialize();
			document.body.classList.add('stream-view-active');
		}
	});

	onDestroy(() => {
		if (browser) {
			info('StreamWindow: Destroying, cleaning up stream store.');
			streamStore.cleanup();
			document.body.classList.remove('stream-view-active');
		}
	});
</script>

<div class="h-screen w-screen overflow-hidden">
	{#if !state.isVisible}
		<div class="flex h-full items-center justify-center bg-black text-white">
			<p>Stream Hidden by Admin</p>
		</div>
	{:else if !streamStore.hasActiveGame || !state.gameState}
		<div class="flex h-full items-center justify-center bg-gray-800 text-white">
			<p>Waiting for game data...</p>
			<!-- Loading spinner can be added here using Project B's UI components -->
		</div>
	{:else}
		<!-- Render game-specific stream view -->
		<GameStreamView gameState={state.gameState} />
	{/if}
</div>
