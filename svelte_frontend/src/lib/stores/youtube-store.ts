import { writable, get } from 'svelte/store';
import { gameStore } from './game';

interface YouTubeState {
    player: YT.Player | null;
    isLoading: boolean;
    currentVideoId: string | null;
    isPlayerReady: boolean;
    pendingVideoId: string | null;
}

function createYouTubeStore() {
    const { subscribe, set, update } = writable<YouTubeState>({
        player: null,
        isLoading: false,
        currentVideoId: null,
        isPlayerReady: false,
        pendingVideoId: null
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
        console.log('Load video requested:', videoId, 'Player ready:', state.isPlayerReady);

        if (!state.isPlayerReady || !state.player) {
            // Store the video ID to load when player is ready
            console.log('Player not ready, storing video ID for later:', videoId);
            update(state => ({ ...state, pendingVideoId: videoId }));
            return;
        }

        if (state.currentVideoId !== videoId) {
            console.log('Loading new video:', videoId);
            state.player.cueVideoById(videoId);
            update(state => ({
                ...state,
                currentVideoId: videoId,
                pendingVideoId: null
            }));
        } else {
            console.log('Video already loaded:', videoId);
        }
    }

    async function verifyAndPlayVideo(expectedVideoId: string): Promise<boolean> {
        const state = get({ subscribe });
        if (!state.player || !state.isPlayerReady) {
            console.log('Player not ready for verification');
            return false;
        }

        try {
            const currentVideoId = state.player.getVideoData()?.video_id;
            console.log('Verifying video before play:', { current: currentVideoId, expected: expectedVideoId });

            if (currentVideoId !== expectedVideoId) {
                // Wrong video loaded, need to load the correct one
                console.log('Video mismatch, loading correct video');
                state.player.loadVideoById(expectedVideoId);
                update(state => ({ ...state, currentVideoId: expectedVideoId }));
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
                console.log('Setting autoplay pending due to player not ready');
                autoplayPending = true;
            }
            return;
        }

        console.log('Handling phase change:', phase);

        switch (phase.toLowerCase()) {
            case 'question':
                if (state.currentVideoId || state.pendingVideoId) {
                    const videoId = state.currentVideoId || state.pendingVideoId;
                    console.log('Question phase: verifying and playing video:', videoId);
                    verifyAndPlayVideo(videoId!);
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
            const state = get({ subscribe });

            update(state => ({
                ...state,
                player,
                isPlayerReady: true
            }));

            // If we have a pending video, load it now
            if (state.pendingVideoId) {
                console.log('Loading pending video:', state.pendingVideoId);
                loadVideo(state.pendingVideoId);
            }

            if (autoplayPending && state.currentVideoId) {
                console.log('Executing pending autoplay');
                player.playVideo();
                autoplayPending = false;
            }
        },
        loadVideo,
        handlePhaseChange,
        cleanup: () => {
            console.log('Cleaning up YouTube player');
            update(state => {
                state.player?.destroy();
                return {
                    player: null,
                    isLoading: false,
                    currentVideoId: null,
                    isPlayerReady: false,
                    pendingVideoId: null
                };
            });
        }
    };
}

export const youtubeStore = createYouTubeStore();
