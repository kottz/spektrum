import { browser } from '$app/environment';
import { broadcastService } from '$lib/services/broadcast.service';
import type { BroadcastMessage } from '$lib/services/broadcast.service';
import type { StreamEvent, DisplayConfig } from '$lib/types/stream.types';
import type { GameUpdate, GamePhase, PlayerAnswer } from '$lib/types/game';
import { DEFAULT_DISPLAY_CONFIG } from '$lib/types/stream.types';
import { info, warn } from '$lib/utils/logger';

interface StreamGameState {
	phase: GamePhase;
	joinCode?: string;
	players: Map<string, { name: string; score: number }>;
	currentAnswers: PlayerAnswer[];
	currentQuestion?: {
		type: string;
		text?: string;
		alternatives: string[];
	};
	realtimeScoreboard: Array<{ name: string; score: number }>;
}

interface StreamStoreState {
	isVisible: boolean;
	currentGameType: string | null;
	gameState: StreamGameState | null;
	activeEvents: StreamEvent[];
}

import { GamePhase } from '$lib/types/game';

const initialStreamGameState: StreamGameState = {
	phase: GamePhase.Lobby,
	players: new Map(),
	currentAnswers: [],
	currentQuestion: undefined,
	realtimeScoreboard: []
};

const initialState: StreamStoreState = {
	isVisible: true,
	currentGameType: null,
	gameState: { ...initialStreamGameState },
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
			case 'SERVER_MESSAGE': {
				info('StreamStore: Received server message', {
					gameType: message.gameType,
					messageType: message.message.type
				});

				if (message.gameType === state.currentGameType || !state.currentGameType) {
					state.currentGameType = message.gameType;
					processServerMessage(message.message as GameUpdate);
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

	function processServerMessage(message: GameUpdate): void {
		if (!state.gameState) {
			state.gameState = { ...initialStreamGameState };
		}

		info('StreamStore: Processing server message', { messageType: message.type, message });

		switch (message.type) {
			case 'Connected': {
				// Initialize or update player
				if ('player_id' in message && 'name' in message) {
					state.gameState.players.set(message.name, {
						name: message.name,
						score: 0
					});
				}
				break;
			}

			case 'StateDelta': {
				info('StreamStore: Processing StateDelta', message);

				if (message.phase !== undefined) {
					const previousPhase = state.gameState.phase;
					state.gameState.phase = message.phase;
					info('StreamStore: Phase changed', { from: previousPhase, to: message.phase });

					// Clear answers when entering a new question phase
					if (message.phase === GamePhase.Question) {
						state.gameState.currentAnswers = [];
						info('StreamStore: Cleared answers for new question');
					}
				}

				// Handle question data
				if (message.question_type !== undefined || message.alternatives !== undefined) {
					state.gameState.currentQuestion = {
						type: message.question_type || 'default',
						text: message.question_text,
						alternatives: message.alternatives || []
					};
					info('StreamStore: Updated current question', state.gameState.currentQuestion);
				}

				if (message.scoreboard) {
					info('StreamStore: Updating players from scoreboard', message.scoreboard);
					state.gameState.players.clear();
					message.scoreboard.forEach(([name, score]) => {
						state.gameState!.players.set(name, { name, score });
					});

					// Update realtime scoreboard from official scores
					state.gameState.realtimeScoreboard = message.scoreboard
						.map(([name, score]) => ({ name, score }))
						.sort((a, b) => b.score - a.score);
				}
				break;
			}

			case 'Answered': {
				info('StreamStore: Processing Answered', { name: message.name, score: message.score });

				// Remove any existing answer from this player
				state.gameState.currentAnswers = state.gameState.currentAnswers.filter(
					(answer) => answer.name !== message.name
				);

				// Add the new answer
				state.gameState.currentAnswers.push({
					name: message.name,
					score: message.score,
					timestamp: Date.now()
				});

				// Update realtime scoreboard if player got points
				if (message.score > 0) {
					state.gameState.realtimeScoreboard = state.gameState.realtimeScoreboard
						.map((player) => {
							if (player.name === message.name) {
								return { ...player, score: player.score + message.score };
							}
							return player;
						})
						.sort((a, b) => b.score - a.score);

					info('StreamStore: Updated realtime scoreboard', {
						player: message.name,
						roundScore: message.score,
						newScoreboard: state.gameState.realtimeScoreboard
					});
				}

				info('StreamStore: Current answers after update', state.gameState.currentAnswers);
				break;
			}

			case 'PlayerLeft': {
				if ('name' in message) {
					state.gameState.players.delete(message.name);
					// Also remove from current answers
					state.gameState.currentAnswers = state.gameState.currentAnswers.filter(
						(answer) => answer.name !== message.name
					);
				}
				break;
			}

			default:
				info('StreamStore: Unhandled server message', { messageType: message.type });
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
