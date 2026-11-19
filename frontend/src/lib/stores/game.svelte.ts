// src/lib/stores/game.ts

import { browser } from '$app/env';
import { youtubeStore } from '#lib/stores/youtube-store.svelte.ts';
import { timerStore } from '#lib/stores/timer.svelte.ts';
import { info, warn } from '#lib/utils/logger.ts';
import { notifications } from '#lib/stores/notification-store.svelte.ts';
import { broadcastService } from '#lib/services/broadcast.service.ts';

import type { GameState } from '#lib/types/game.ts';
import type { GameUpdate } from '#lib/generated/GameUpdate.ts';
import type { GameQuestion } from '#lib/generated/GameQuestion.ts';
import { api } from '#lib/api.ts';
import { applyPlayerDelta } from '#lib/stores/player-delta.ts';

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
	phase: 'lobby',
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
		timerStore.resetTimer();
		Object.assign(state, {
			...initialState,
			players: new Map(),
			currentAnswers: [],
			playerId: undefined,
			playerName: undefined,
			currentQuestion: undefined,
			upcomingQuestions: undefined,
			questionTimeRemainingMs: undefined
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
			const data = await api.checkSessions({ sessions: [{ player_id: session.playerId }] });

			info('Session check response:', data.valid_sessions);

			const validSession = data.valid_sessions.find((v) => v.player_id === session.playerId);

			if (validSession) {
				return {
					...session,
					last_update: validSession.last_update
				} satisfies ValidatedSession;
			}

			removeSession();
			return null;
		} catch (err) {
			warn('Error checking session:', err);
			return null;
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

				// Deltas are partial: an absent field means "unchanged", never "zero".
				// A scoreboard, when present, is the complete player list.
				state.players = applyPlayerDelta(state.players, message);

				// A question arrives with its type; without one, the delta leaves the
				// question unchanged. Clearing happens below when the phase leaves Question.
				if (message.question_type !== undefined) {
					state.currentQuestion = {
						type: message.question_type,
						text: message.question_text,
						alternatives: message.alternatives ?? []
					};
				}

				if (message.question_time_remaining_ms !== undefined) {
					state.questionTimeRemainingMs = message.question_time_remaining_ms;
				}

				if (message.admin_extra) {
					setUpcomingQuestions(message.admin_extra.upcoming_questions);
				}

				// Sync answered players and currentAnswers from snapshot.
				if (message.answered_player_names) {
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
					if (currentPhase === 'question') {
						timerStore.startTimer(state.roundDuration, message.question_time_remaining_ms);
					}
					if (currentPhase === 'score') {
						timerStore.stopTimer(true);
						state.currentAnswers = [];
						state.currentQuestion = undefined;
					}
				} else if (state.phase === 'question' && message.question_time_remaining_ms !== undefined) {
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
				state.players = new Map([...state.players].filter(([name]) => name !== message.name));
				break;
			}

			case 'PlayerKicked': {
				info(`Player kicked: ${message.reason}`);
				cleanup();
				notifications.add(message.reason, 'destructive');
				break;
			}

			case 'GameOver': {
				// Final scores are authoritative: a game ended mid-round has points
				// that no scoreboard delta reported yet
				const finalScores = new Map(message.final_scores);
				state.players = new Map(
					[...state.players].map(([name, player]) => [
						name,
						{ ...player, score: finalScores.get(name) ?? player.score }
					])
				);
				state.phase = 'gameover';
				state.upcomingQuestions = undefined;
				state.currentQuestion = undefined;
				state.currentAnswers = [];
				timerStore.stopTimer(true);
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
				break;
			}

			case 'AdminNextQuestions': {
				setUpcomingQuestions(message.upcoming_questions);
				break;
			}

			case 'Error': {
				notifications.add(message.message, 'destructive');
				if (message.code === 'session_invalid') {
					// The game is gone for us: leave the dead game screen
					cleanup();
				}
				break;
			}

			default:
				warn('Unhandled message type:', message);
		}
	}

	function setUpcomingQuestions(upcoming: GameQuestion[]) {
		const nextQuestion = upcoming[0];
		if (state.phase !== 'question' && nextQuestion?.youtube_id) {
			info('Loading next video:', nextQuestion.youtube_id);
			youtubeStore.loadVideo(nextQuestion.youtube_id);
		}
		state.upcomingQuestions = upcoming;
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

	return {
		state,
		checkSessions,
		processServerMessage,
		setAdminTo,
		setJoinCode,
		setSessionToken,
		setPlayerId,
		setPlayerName,
		cleanup
	};
}

export const gameStore = createGameStore();
