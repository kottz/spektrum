<script lang="ts">
	import { youtubeStore } from '$lib/stores/youtube-store.svelte';
	import { gameStore } from '$lib/stores/game.svelte';
	import { Card, CardContent } from '$lib/components/ui/card';
	import { info, warn } from '$lib/utils/logger';

	let playerId = 'youtube-player';

	$effect.root(() => {
		info('YouTube component mounted');
		youtubeStore.initializeAPI();

		const initializePlayer = () => {
			info('YouTube API Ready');
			return new YT.Player(playerId, {
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

		(window as any).onYouTubeIframeAPIReady = initializePlayer;

		// Cleanup function
		return () => {
			youtubeStore.cleanup();
			// Clean up the global callback
			delete (window as any).onYouTubeIframeAPIReady;
		};
	});

	$effect(() => {
		const phase = gameStore.state.phase;
		if (phase) {
			youtubeStore.handlePhaseChange(phase);
		}
	});
</script>

<Card>
	<CardContent class="p-1">
		<div class="aspect-video w-full bg-muted">
			<div id={playerId}></div>
		</div>
	</CardContent>
</Card>

<style>
	#youtube-player {
		width: 100%;
		height: 100%;
	}
</style>
