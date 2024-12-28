import { writable, get } from 'svelte/store';
import { gameStore } from './game';

interface YouTubeState {
    player: YT.Player | null;
    isLoading: boolean;
    currentVideoId: string | null;
    isPlayerReady: boolean;
}

function createYouTubeStore() {
    const { subscribe, set, update } = writable<YouTubeState>({
        player: null,
        isLoading: false,
        currentVideoId: null,
        isPlayerReady: false
    });

    let autoplayPending = false;

    function initializeAPI() {
        console.log('Initializing YouTube API');
        if (document.querySelector('script[src*="youtube.com/iframe_api"]')) return;

        const tag = document.createElement('script');
        tag.src = "https://www.youtube.com/iframe_api";
        const firstScriptTag = document.getElementsByTagName('script')[0];
        firstScriptTag.parentNode?.insertBefore(tag, firstScriptTag);
    }

    function loadVideo(videoId: string) {
        const state = get({ subscribe });
        if (state.player && state.isPlayerReady) {
            console.log('Loading video:', videoId);
            // Only load if it's different from current
            if (state.currentVideoId !== videoId) {
                state.player.cueVideoById(videoId);
            }
            update(state => ({ ...state, currentVideoId: videoId }));
        }
    }

    async function verifyAndPlayVideo(expectedVideoId: string): Promise<boolean> {
        const state = get({ subscribe });
        if (!state.player || !state.isPlayerReady) return false;

        try {
            const currentVideoId = state.player.getVideoData()?.video_id;
            console.log('Verifying video before play:', { current: currentVideoId, expected: expectedVideoId });

            if (currentVideoId !== expectedVideoId) {
                // We have the wrong video loaded, need to load and play the correct one
                console.log('Video mismatch, loading correct video');
                state.player.loadVideoById(expectedVideoId);
                return true;
            } else {
                // Correct video is already loaded, just play it
                console.log('Correct video already loaded, playing');
                state.player.playVideo();
                return true;
            }
        } catch (error) {
            console.error('Error verifying video:', error);
            return false;
        }
    }

    function handlePhaseChange(phase: string) {
        const state = get({ subscribe });
        if (!state.player || !state.isPlayerReady) {
            if (phase === 'question') {
                autoplayPending = true;
            }
            return;
        }

        console.log('Handling phase change:', phase);

        switch (phase.toLowerCase()) {
            case 'question':
                // Verify and play current video
                if (state.currentVideoId) {
                    verifyAndPlayVideo(state.currentVideoId);
                }
                break;
            case 'score':
                console.log('Score phase, stopping video');
                state.player.stopVideo();
                break;
            case 'lobby':
            case 'gameover':
                console.log('Stopping video');
                state.player.stopVideo();
                break;
        }
    }

    return {
        subscribe,
        initializeAPI,
        setPlayer: (player: YT.Player) => {
            console.log('Setting player');
            update(state => ({
                ...state,
                player,
                isPlayerReady: true
            }));
            if (autoplayPending) {
                player.playVideo();
                autoplayPending = false;
            }
        },
        loadVideo,
        handlePhaseChange,
        cleanup: () => {
            update(state => {
                state.player?.destroy();
                return {
                    player: null,
                    isLoading: false,
                    currentVideoId: null,
                    isPlayerReady: false
                };
            });
        }
    };
}

export const youtubeStore = createYouTubeStore();
