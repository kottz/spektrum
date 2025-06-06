// src/lib/stores/youtube-store.ts
import { info, warn } from '$lib/utils/logger';

interface YouTubeState {
	player: YT.Player | null;
	isLoading: boolean;
	currentVideoId: string | null;
	isPlayerReady: boolean;
	pendingVideoId: string | null;
}

function createYouTubeStore() {
	const state = $state<YouTubeState>({
		player: null,
		isLoading: false,
		currentVideoId: null,
		isPlayerReady: false,
		pendingVideoId: null
	});

	let autoplayPending = false;
	let apiInitialized = false;

	function initializeAPI() {
		info('Initializing YouTube API');

		// Remove existing script if it exists
		const existingScript = document.querySelector('script[src*="youtube.com/iframe_api"]');
		if (existingScript) {
			existingScript.remove();
		}

		// Reset global YouTube API state
		(window as any).YT = undefined;
		(window as any).onYouTubeIframeAPIReady = undefined;

		// Add new script
		const tag = document.createElement('script');
		tag.src = 'https://www.youtube.com/iframe_api';
		const firstScriptTag = document.getElementsByTagName('script')[0];
		firstScriptTag.parentNode?.insertBefore(tag, firstScriptTag);

		apiInitialized = true;
	}

	function loadVideo(videoId: string) {
		info('Load video requested:', videoId, 'Player ready:', state.isPlayerReady);

		if (!apiInitialized) {
			info('API not initialized, initializing first');
			initializeAPI();
		}

		if (!state.isPlayerReady || !state.player) {
			info('Player not ready, storing video ID for later:', videoId);
			state.pendingVideoId = videoId;
			return;
		}

		if (state.currentVideoId !== videoId) {
			info('Loading new video:', videoId);
			state.player.cueVideoById(videoId);
			state.currentVideoId = videoId;
			state.pendingVideoId = null;
		} else {
			info('Video already loaded:', videoId);
		}
	}

	async function verifyAndPlayVideo(expectedVideoId: string): Promise<boolean> {
		if (!state.player || !state.isPlayerReady) {
			info('Player not ready for verification');
			return false;
		}

		try {
			const currentVideoId = state.player.getVideoData()?.video_id;
			info('Verifying video before play:', { current: currentVideoId, expected: expectedVideoId });

			if (currentVideoId !== expectedVideoId) {
				info('Video mismatch, loading correct video');
				state.player.loadVideoById(expectedVideoId);
				state.currentVideoId = expectedVideoId;
				return true;
			} else {
				info('Correct video already loaded, playing');
				state.player.playVideo();
				return true;
			}
		} catch (error) {
			warn('Error verifying video:', error);
			return false;
		}
	}

	function handlePhaseChange(phase: string) {
		if (!state.player || !state.isPlayerReady) {
			if (phase === 'question') {
				info('Setting autoplay pending due to player not ready');
				autoplayPending = true;
			}
			return;
		}

		info('Handling phase change:', phase);

		switch (phase.toLowerCase()) {
			case 'question':
				if (state.currentVideoId || state.pendingVideoId) {
					const videoId = state.currentVideoId || state.pendingVideoId;
					info('Question phase: verifying and playing video:', videoId);
					verifyAndPlayVideo(videoId!);
				}
				break;
			case 'score':
			case 'lobby':
			case 'gameover':
				info('Stopping video');
				state.player.stopVideo();
				break;
		}
	}

	function setPlayer(player: YT.Player) {
		info('Setting player');

		state.player = player;
		state.isPlayerReady = true;

		if (state.pendingVideoId) {
			info('Loading pending video:', state.pendingVideoId);
			loadVideo(state.pendingVideoId);
		}

		if (autoplayPending && state.currentVideoId) {
			info('Executing pending autoplay');
			player.playVideo();
			autoplayPending = false;
		}
	}

	function cleanup() {
		info('Cleaning up YouTube player');

		if (state.player) {
			state.player.destroy();
		}

		// Reset API initialization flag
		apiInitialized = false;

		// Reset the store state
		state.player = null;
		state.isLoading = false;
		state.currentVideoId = null;
		state.isPlayerReady = false;
		state.pendingVideoId = null;

		// Remove the YouTube iframe if it exists
		const iframe = document.querySelector('iframe[src*="youtube.com"]');
		if (iframe) {
			iframe.remove();
		}
	}

	return {
		state,
		initializeAPI,
		setPlayer,
		loadVideo,
		handlePhaseChange,
		cleanup
	};
}

export const youtubeStore = createYouTubeStore();
