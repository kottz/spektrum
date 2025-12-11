import { browser } from '$app/environment';
import { broadcastService } from '$lib/services/broadcast.service';
import type { BroadcastMessage } from '$lib/services/broadcast.service';
import type { StreamEvent, DisplayConfig } from '$lib/types/stream.types';
import type { GameUpdate, PlayerAnswer } from '$lib/types/game';
import { GamePhase } from '$lib/types/game';
import { DEFAULT_DISPLAY_CONFIG } from '$lib/types/stream.types';
import { info, warn } from '$lib/utils/logger';
import { streamTimerStore } from './stream-timer.store.svelte';

interface StreamGameState {
	phase: GamePhase;
	joinCode?: string;
	players: Map<string, { name: string; score: number }>;
	currentAnswers: PlayerAnswer[];
	roundDuration: number;
	currentQuestion?: {
		type: string;
		text?: string;
		alternatives: string[];
	};
	realtimeScoreboard: Array<{ name: string; score: number; roundScore: number }>;
}

interface StreamStoreState {
	isVisible: boolean;
	currentGameType: string | null;
	gameState: StreamGameState | null;
	activeEvents: StreamEvent[];
}

const initialStreamGameState: StreamGameState = {
	phase: GamePhase.Lobby,
	players: new Map(),
	currentAnswers: [],
	roundDuration: 60,
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

		// Signal that the stream window is ready to receive data
		broadcastService.broadcastStreamReady();

		// Listen for window close/refresh to notify admin
		window.addEventListener('beforeunload', () => {
			broadcastService.broadcastStreamDisconnected();
		});
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
				if (message.gameType === state.currentGameType || !state.currentGameType) {
					state.currentGameType = message.gameType;
					processServerMessage(message.message as GameUpdate);
				}
				break;
			}

			case 'INITIAL_STATE': {
				state.currentGameType = message.gameType;
				if (!state.gameState) {
					state.gameState = { ...initialStreamGameState };
				}
				state.gameState.joinCode = message.joinCode;

				if (message.gameState && typeof message.gameState === 'object') {
					processServerMessage(message.gameState as GameUpdate);
				}
				break;
			}

			case 'STREAM_CONTROL': {
				state.isVisible = message.action === 'show';
				break;
			}

			case 'STREAM_CLOSE': {
				window.close();
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

		switch (message.type) {
			case 'Connected': {
				if ('player_id' in message && 'name' in message) {
					state.gameState.roundDuration = message.round_duration ?? state.gameState.roundDuration;
					state.gameState.players.set(message.name, {
						name: message.name,
						score: 0
					});
					const existingIndex = state.gameState.realtimeScoreboard.findIndex(
						(p) => p.name === message.name
					);
					if (existingIndex === -1) {
						state.gameState.realtimeScoreboard.push({
							name: message.name,
							score: 0,
							roundScore: 0
						});
					}
				}
				break;
			}

			case 'StateDelta': {
				if (message.phase !== undefined && message.phase !== null) {
					state.gameState.phase = message.phase;

					if (message.phase === GamePhase.Question) {
						state.gameState.currentAnswers = [];
						state.gameState.realtimeScoreboard = state.gameState.realtimeScoreboard.map(
							(player) => ({
								...player,
								roundScore: 0
							})
						);
						streamTimerStore.startTimer(
							state.gameState.roundDuration,
							message.question_time_remaining_ms
						);
					} else {
						streamTimerStore.stopTimer();
					}
				}

				if (message.question_type !== undefined || message.alternatives !== undefined) {
					state.gameState.currentQuestion = {
						type: message.question_type || 'default',
						text: message.question_text,
						alternatives: message.alternatives || []
					};
				}

				if (message.scoreboard) {
					if (message.scoreboard.length > 0) {
						state.gameState.players.clear();
						message.scoreboard.forEach(([name, score]) => {
							state.gameState!.players.set(name, { name, score });
						});
					}

					if (message.scoreboard.length > 0) {
						state.gameState.realtimeScoreboard = message.scoreboard
							.map(([name, score]) => {
								const roundScore =
									message.round_scores?.find(([playerName]) => playerName === name)?.[1] || 0;
								return { name, score, roundScore };
							})
							.sort((a, b) => b.score - a.score);
					}
				}

				if (message.answered_player_names) {
					const roundScoreMap = new Map(message.round_scores ?? []);
					const now = Date.now();
					state.gameState.currentAnswers = message.answered_player_names.map((name) => ({
						name,
						score: roundScoreMap.get(name) ?? 0,
						timestamp: now
					}));
				}

				if (
					state.gameState.phase === GamePhase.Question &&
					message.question_time_remaining_ms !== undefined
				) {
					streamTimerStore.startTimer(
						state.gameState.roundDuration,
						message.question_time_remaining_ms
					);
				}
				break;
			}

			case 'Answered': {
				state.gameState.currentAnswers = state.gameState.currentAnswers.filter(
					(answer) => answer.name !== message.name
				);

				state.gameState.currentAnswers.push({
					name: message.name,
					score: message.score,
					timestamp: Date.now()
				});

				const existingPlayerIndex = state.gameState.realtimeScoreboard.findIndex(
					(player) => player.name === message.name
				);

				if (existingPlayerIndex >= 0) {
					if (message.score > 0) {
						state.gameState.realtimeScoreboard = state.gameState.realtimeScoreboard
							.map((player) => {
								if (player.name === message.name) {
									return {
										...player,
										score: player.score + message.score,
										roundScore: player.roundScore + message.score
									};
								}
								return player;
							})
							.sort((a, b) => b.score - a.score);
					}
				} else {
					state.gameState.realtimeScoreboard.push({
						name: message.name,
						score: message.score,
						roundScore: message.score
					});
					state.gameState.realtimeScoreboard.sort((a, b) => b.score - a.score);
				}
				break;
			}

			case 'PlayerLeft': {
				if ('name' in message) {
					state.gameState.players.delete(message.name);
					state.gameState.realtimeScoreboard = state.gameState.realtimeScoreboard.filter(
						(player) => player.name !== message.name
					);
					state.gameState.currentAnswers = state.gameState.currentAnswers.filter(
						(answer) => answer.name !== message.name
					);
				}
				break;
			}

			case 'GameOver': {
				state.gameState.phase = GamePhase.GameOver;

				if (message.final_scores) {
					state.gameState.players.clear();
					message.final_scores.forEach(([name, score]) => {
						state.gameState!.players.set(name, { name, score });
					});

					state.gameState.realtimeScoreboard = message.final_scores
						.map(([name, score]) => ({ name, score, roundScore: 0 }))
						.sort((a, b) => b.score - a.score);
				}

				streamTimerStore.stopTimer();
				break;
			}

			default:
				break;
		}
	}

	function removeStreamEvent(eventId: string): void {
		state.activeEvents = state.activeEvents.filter((event) => event.id !== eventId);
	}

	function cleanupExpiredEvents(): void {
		const now = Date.now();

		state.activeEvents = state.activeEvents.filter((event) => {
			if (!event.duration) return true;
			return now - event.timestamp < event.duration;
		});
	}

	function updateDisplayConfig(updates: Partial<DisplayConfig>): void {
		Object.assign(displayConfig, updates);
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
