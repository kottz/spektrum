<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { youtubeStore } from '$lib/stores/youtube-store.svelte';
	import { gameStore } from '$lib/stores/game.svelte';
	import { Card, CardContent } from '$lib/components/ui/card';
	import { info, warn } from '$lib/utils/logger';

	let playerId = 'youtube-player';

	onMount(() => {
		info('YouTube component mounted');
		youtubeStore.initializeAPI();

		// Define global callback for YouTube API
		(window as any).onYouTubeIframeAPIReady = () => {
			info('YouTube API Ready');
			const player = new YT.Player(playerId, {
				height: '360',
				width: '640',
				playerVars: {
					controls: 0,
					playsinline: 1,
					enablejsapi: 1
				},
				events: {
					onReady: (event) => {
						info('Player ready');
						youtubeStore.setPlayer(event.target);
					},
					onError: (event) => {
						warn('YouTube player error:', event);
					}
				}
			});
		};
	});

	onDestroy(() => {
		youtubeStore.cleanup();
	});

	// Watch for game phase changes
	$: if (gameStore.state.phase) {
		youtubeStore.handlePhaseChange(gameStore.state.phase);
	}
</script>

<Card>
	<CardContent class="p-4">
		<div class="aspect-video w-full bg-muted">
			<!-- Create a div with the ID that YouTube API will use -->
			<div id={playerId}></div>
		</div>
	</CardContent>
</Card>

<style>
	/* Ensure the player container has dimensions */
	#youtube-player {
		width: 100%;
		height: 100%;
	}
</style>
