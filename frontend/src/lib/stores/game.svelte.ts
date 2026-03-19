// src/lib/stores/game.ts

import { browser } from '$app/environment';
import { youtubeStore } from '$lib/stores/youtube-store.svelte';
import { timerStore } from '$lib/stores/timer.svelte';
import { info, warn } from '$lib/utils/logger';
import { notifications } from '$lib/stores/notification-store.svelte';
import { broadcastService } from '$lib/services/broadcast.service';

import type { GameState, GameUpdate } from '../types/game';
import { GamePhase } from '../types/game';
import { PUBLIC_SPEKTRUM_SERVER_URL } from '$env/static/public';

/* ------------------------------------------------------------------
   Multi-session storage for localStorage
------------------------------------------------------------------ */
export interface SessionInfo {
	playerId: string;
	playerName: string;
	isAdmin: boolean;
	joinCode: string;
	sessionToken: string;
}

/** SessionInfo enriched with server-validated last_update timestamp. */
export type ValidatedSession = SessionInfo & { last_update: string };

export function loadSession(): SessionInfo | null {
	if (!browser) return null;
	try {
		const data = localStorage.getItem('spektrumSession');
		return data ? JSON.parse(data) : null;
	} catch {
		return null;
	}
}

export function saveSession(session: SessionInfo) {
	if (!browser) return;
	localStorage.setItem('spektrumSession', JSON.stringify(session));
}

export function removeSession() {
	if (!browser) return;
	localStorage.removeItem('spektrumSession');
}

/* ------------------------------------------------------------------
   Game store for the currently active session
------------------------------------------------------------------ */
const initialState: GameState = {
	phase: GamePhase.Lobby,
	isAdmin: false,
	joinCode: undefined,
	sessionToken: undefined,
	roundDuration: 60,
	players: new Map(),
	currentAnswers: [],
	lobbyLocked: false
};

function createGameStore() {
	const state = $state<GameState>({ ...initialState });

	/**
	 * Cleanup resets the store and removes the current session.
	 */
	function cleanup() {
		info('Running cleanup...');

		// Close stream window if admin
		if (state.isAdmin && broadcastService.getIsInitialized()) {
			broadcastService.broadcastStreamClose();
		}

		removeSession();
		Object.assign(state, {
			...initialState,
			players: new Map(),
			currentAnswers: [],
			playerId: undefined,
			playerName: undefined,
			currentQuestion: undefined,
			currentSong: undefined,
			upcomingQuestions: undefined,
			error: undefined,
			questionTimeRemainingMs: undefined,
			answeredPlayerNames: undefined
		});
	}

	/**
	 * Optional helper to validate the current session with the server.
	 */
	async function checkSessions(): Promise<ValidatedSession | null> {
		if (!browser) return null;
		const session = loadSession();
		if (!session || !session.sessionToken) return null;

		try {
			const res = await fetch(`${PUBLIC_SPEKTRUM_SERVER_URL}/api/check-sessions`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					sessions: [
						{
							player_id: session.playerId
						}
					]
				}),
				cache: 'no-store'
			});

			if (!res.ok) {
				warn('Failed to check session:', res.status, res.statusText);
				return null; // Return null instead of session on error
			}

			const data = (await res.json()) as {
				valid_sessions: Array<{ player_id: string; last_update: string }>;
			};

			info('Session check response:', data.valid_sessions);

			const validSession = data.valid_sessions.find((v) => v.player_id === session.playerId);

			if (validSession) {
				// Explicitly return the combined type
				return {
					...session,
					last_update: validSession.last_update
				} satisfies ValidatedSession;
			}

			removeSession();
			return null;
		} catch (err) {
			warn('Error checking session:', err);
			return null; // Return null instead of session on error
		}
	}

	/**
	 * Processes an incoming GameUpdate message from the server.
	 */
	function processServerMessage(message: GameUpdate) {
		info('Handling server message:', message);

		// Initialize broadcast service if not already done and this is admin
		if (
			state.isAdmin &&
			!broadcastService.getIsInitialized() &&
			!broadcastService.getIsStreamWindow()
		) {
			broadcastService.initialize(false); // false = admin window
		}

		// Broadcast the raw server message to stream immediately (only if admin)
		if (state.isAdmin) {
			broadcastService.broadcastServerMessage('SpektrumGame', message as Record<string, unknown>);
		}

		// If the game is closed, run cleanup.
		if (message.type === 'GameClosed') {
			info('Game closed, cleaning up...');
			cleanup();
			return;
		}

		switch (message.type) {
			case 'Connected': {
				// Save the session to localStorage for reconnection.
				const session: SessionInfo = {
					playerId: message.player_id,
					playerName: message.name,
					isAdmin: state.isAdmin,
					joinCode: state.joinCode || '',
					sessionToken: state.sessionToken || ''
				};
				saveSession(session);
				state.playerId = message.player_id;
				state.playerName = message.name;
				state.roundDuration = message.round_duration;
				timerStore.setRoundDuration(message.round_duration);
				break;
			}

			case 'StateDelta': {
				// Update phase if provided.
				const previousPhase = state.phase;
				if (message.phase !== undefined && message.phase !== null) {
					state.phase = message.phase;
				}

				// Update players using provided scoreboard and round scores.
				if (message.scoreboard) {
					const newPlayers = new Map(state.players);
					message.scoreboard.forEach(([name, score]) => {
						newPlayers.set(name, {
							name,
							score,
							roundScore: 0,
							hasAnswered: false,
							consecutiveMisses: 0,
							answer: null
						});
					});
					if (message.round_scores) {
						message.round_scores.forEach(([name, roundScore]) => {
							const player = newPlayers.get(name);
							if (player) {
								player.roundScore = roundScore;
							}
						});
					}
					if (message.consecutive_misses) {
						message.consecutive_misses.forEach(([name, misses]) => {
							const player = newPlayers.get(name);
							if (player) {
								player.consecutiveMisses = misses;
							}
						});
					}
					if (message.answered_player_names) {
						message.answered_player_names.forEach((name) => {
							const player = newPlayers.get(name);
							if (player) {
								player.hasAnswered = true;
							}
						});
					}
					state.players = newPlayers;
				}

				// Update answered flags on existing players even if no scoreboard was provided.
				if (message.answered_player_names && !message.scoreboard) {
					const updated = new Map(state.players);
					updated.forEach((player, name) => {
						player.hasAnswered = message.answered_player_names!.includes(name);
					});
					state.players = updated;
				}

				// Update the current question when alternatives are explicitly provided.
				// When absent, leave currentQuestion alone — absence in a delta means "no change".
				// Clearing happens below when the phase transitions away from Question.
				if (message.alternatives !== undefined) {
					state.currentQuestion = {
						type: message.question_type ?? '',
						text: message.question_text ?? undefined,
						alternatives: message.alternatives
					};
				}

				if (message.question_time_remaining_ms !== undefined) {
					state.questionTimeRemainingMs = message.question_time_remaining_ms;
				}

				// Sync answered players and currentAnswers from snapshot.
				if (message.answered_player_names) {
					state.answeredPlayerNames = message.answered_player_names;
					const roundScoreMap = new Map(message.round_scores ?? []);
					const existingAnswers = new Map(
						state.currentAnswers.map((ans) => [ans.name, ans] as const)
					);
					const now = Date.now();
					state.currentAnswers = message.answered_player_names.map((name) => {
						const existing = existingAnswers.get(name);
						if (existing) return existing;
						return {
							name,
							score: roundScoreMap.get(name) ?? 0,
							timestamp: now
						};
					});
				}

				// Perform phase-specific actions.
				const currentPhase = state.phase;
				if (previousPhase !== currentPhase) {
					if (currentPhase === GamePhase.Question) {
						timerStore.startTimer(state.roundDuration, message.question_time_remaining_ms);
					}
					if (currentPhase === GamePhase.Score) {
						timerStore.stopTimer(true);
						state.currentAnswers = [];
						state.currentQuestion = undefined;
					}
				} else if (
					state.phase === GamePhase.Question &&
					message.question_time_remaining_ms !== undefined
				) {
					// Same phase but got a time snapshot — resync the timer.
					timerStore.startTimer(state.roundDuration, message.question_time_remaining_ms);
				}

				if (message.lobby_locked !== undefined) {
					state.lobbyLocked = message.lobby_locked;
				}

				break;
			}

			case 'Answered': {
				// Update the current answers if not already present.
				if (state.currentAnswers.some((a) => a.name === message.name)) {
					break;
				}
				state.currentAnswers = [
					...state.currentAnswers,
					{
						name: message.name,
						score: message.score,
						timestamp: Date.now()
					}
				];

				break;
			}

			case 'PlayerLeft': {
				info(`Player left: ${message.name}`);
				const updated = new Map(state.players);
				updated.delete(message.name);
				state.players = updated;
				break;
			}

			case 'PlayerKicked': {
				info(`Player kicked: ${message.reason}`);
				cleanup();
				notifications.add(`${message.reason}`, 'destructive');
				break;
			}

			case 'GameOver': {
				state.phase = GamePhase.GameOver;
				state.upcomingQuestions = undefined;
				break;
			}

			case 'AdminInfo': {
				if (message.current_question) {
					const { youtube_id, title, artist } = message.current_question;
					info('Received song info:', { youtube_id, title, artist });
					if (youtube_id) {
						youtubeStore.loadVideo(youtube_id);
					}
				}
				state.currentSong = message.current_question
					? {
							songName: message.current_question.title,
							artist: message.current_question.artist,
							youtubeId: message.current_question.youtube_id
						}
					: undefined;
				break;
			}

			case 'AdminNextQuestions': {
				const nextQuestion = message.upcoming_questions[0];
				if (nextQuestion?.youtube_id) {
					info('Loading next video:', nextQuestion.youtube_id);
					youtubeStore.loadVideo(nextQuestion.youtube_id);
				}
				state.upcomingQuestions = message.upcoming_questions;
				break;
			}

			case 'Error': {
				state.error = message.message;
				notifications.add(`${message.message}`, 'destructive');
				break;
			}

			default:
				warn('Unhandled message type:', message);
		}
	}

	function setAdmin() {
		state.isAdmin = true;
	}

	function setAdminTo(value: boolean) {
		state.isAdmin = value;
	}

	function setJoinCode(joinCode: string) {
		state.joinCode = joinCode;
	}

	function setSessionToken(sessionToken: string) {
		state.sessionToken = sessionToken;
	}

	function setPlayerId(playerId: string) {
		state.playerId = playerId;
	}

	function setPlayerName(playerName: string) {
		state.playerName = playerName;
	}

	function clearError() {
		state.error = undefined;
	}

	return {
		state,
		checkSessions,
		loadSession,
		processServerMessage,
		setAdmin,
		setAdminTo,
		setJoinCode,
		setSessionToken,
		setPlayerId,
		setPlayerName,
		cleanup,
		clearError
	};
}

export const gameStore = createGameStore();
