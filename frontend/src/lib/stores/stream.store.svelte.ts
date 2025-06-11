import { browser } from '$app/environment';
import { broadcastService } from '$lib/services/broadcast.service';
import type { BroadcastMessage } from '$lib/services/broadcast.service';
import type { StreamEvent, DisplayConfig } from '$lib/types/stream.types';
import type { PublicGameState } from '$lib/types/game';
import { DEFAULT_DISPLAY_CONFIG } from '$lib/types/stream.types';
import { info, warn } from '$lib/utils/logger';

interface StreamStoreState {
	isVisible: boolean;
	currentGameType: string | null;
	gameState: PublicGameState | null;
	activeEvents: StreamEvent[];
}

const initialState: StreamStoreState = {
	isVisible: true,
	currentGameType: null,
	gameState: null,
	activeEvents: []
};

function createStreamStore() {
	const state = $state<StreamStoreState>({ ...initialState });
	const displayConfig = $state<DisplayConfig>({ ...DEFAULT_DISPLAY_CONFIG });

	let isInitialized = false;
	let eventCleanupInterval: ReturnType<typeof setInterval> | null = null;

	const hasActiveGame = $derived(state.currentGameType !== null && state.gameState !== null);

	function initialize(): void {
		if (isInitialized) {
			info('StreamStore: Already initialized');
			return;
		}

		if (!browser) {
			warn('StreamStore: Cannot initialize in non-browser environment');
			return;
		}

		broadcastService.initialize(true); // true = stream window
		broadcastService.addListener(handleBroadcastMessage);

		// Clean up expired events every 5 seconds
		eventCleanupInterval = setInterval(cleanupExpiredEvents, 5000);

		isInitialized = true;
		info('StreamStore: Initialized');
	}

	function cleanup(): void {
		if (!isInitialized) return;

		broadcastService.removeListener(handleBroadcastMessage);
		broadcastService.cleanup();

		if (eventCleanupInterval) {
			clearInterval(eventCleanupInterval);
			eventCleanupInterval = null;
		}

		// Reset state
		Object.assign(state, initialState);
		Object.assign(displayConfig, DEFAULT_DISPLAY_CONFIG);

		isInitialized = false;
		info('StreamStore: Cleaned up');
	}

	function handleBroadcastMessage(message: BroadcastMessage): void {
		switch (message.type) {
			case 'STATE_UPDATE': {
				info('StreamStore: Received state update', { gameType: message.gameType });
				state.currentGameType = message.gameType;
				state.gameState = message.gameState as PublicGameState;
				break;
			}

			case 'STREAM_EVENT': {
				info('StreamStore: Received stream event', {
					gameType: message.gameType,
					eventType: message.event.type
				});

				if (message.gameType === state.currentGameType) {
					addStreamEvent(message.event);
				}
				break;
			}

			case 'STREAM_CONTROL': {
				info('StreamStore: Received stream control', { action: message.action });
				state.isVisible = message.action === 'show';
				break;
			}

			default:
				warn('StreamStore: Unknown message type', message);
		}
	}

	function addStreamEvent(event: StreamEvent): void {
		// Add event to active events
		state.activeEvents = [...state.activeEvents, event];

		// Schedule removal if event has duration
		if (event.duration && event.duration > 0) {
			setTimeout(() => {
				removeStreamEvent(event.id);
			}, event.duration);
		}
	}

	function removeStreamEvent(eventId: string): void {
		state.activeEvents = state.activeEvents.filter((event) => event.id !== eventId);
	}

	function cleanupExpiredEvents(): void {
		const now = Date.now();
		const beforeCount = state.activeEvents.length;

		state.activeEvents = state.activeEvents.filter((event) => {
			if (!event.duration) return true; // Keep events without duration
			return now - event.timestamp < event.duration;
		});

		const afterCount = state.activeEvents.length;
		if (beforeCount !== afterCount) {
			info(`StreamStore: Cleaned up ${beforeCount - afterCount} expired events`);
		}
	}

	function updateDisplayConfig(updates: Partial<DisplayConfig>): void {
		Object.assign(displayConfig, updates);
		info('StreamStore: Display config updated', updates);
	}

	return {
		get state() {
			return state;
		},
		get displayConfig() {
			return displayConfig;
		},
		get hasActiveGame() {
			return hasActiveGame;
		},
		initialize,
		cleanup,
		updateDisplayConfig,
		removeStreamEvent
	};
}

export const streamStore = createStreamStore();
