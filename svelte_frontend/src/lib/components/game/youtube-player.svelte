<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { youtubeStore } from '../../stores/youtube-store';
	import { gameStore } from '../../stores/game';
	import { Card, CardContent } from '$lib/components/ui/card';

	let playerId = 'youtube-player';

	onMount(() => {
		console.log('YouTube component mounted');
		youtubeStore.initializeAPI();

		// Define global callback for YouTube API
		(window as any).onYouTubeIframeAPIReady = () => {
			console.log('YouTube API Ready');
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
						console.log('Player ready');
						youtubeStore.setPlayer(event.target);
					},
					onError: (event) => {
						console.error('YouTube player error:', event);
					}
				}
			});
		};
	});

	onDestroy(() => {
		youtubeStore.cleanup();
	});

	// Watch for game phase changes
	$: if ($gameStore.phase) {
		youtubeStore.handlePhaseChange($gameStore.phase);
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
