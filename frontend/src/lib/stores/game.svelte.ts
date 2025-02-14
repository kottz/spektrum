// src/lib/stores/game.ts

import { browser } from '$app/environment';
import { youtubeStore } from '$lib/stores/youtube-store.svelte';
import { timerStore } from '$lib/stores/timer-store.svelte';
import { info, warn } from '$lib/utils/logger';
import { notifications } from '$lib/stores/notification-store';

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
}

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
	roundDuration: 60,
	players: new Map(),
	currentAnswers: []
};

function createGameStore() {
	const state = $state<GameState>({ ...initialState });

	/**
	 * Cleanup resets the store and removes the current session.
	 */
	function cleanup() {
		info('Running cleanup...');
		removeSession();
		state.playerId = undefined;
		state.playerName = undefined;
		state.isAdmin = false;
		state.joinCode = undefined;
		// Reset state to initial values.
		Object.assign(state, initialState);
	}

	/**
	 * Optional helper to validate the current session with the server.
	 */
	async function checkSessions(): Promise<(SessionInfo & { last_update: string }) | null> {
		if (!browser) return null;
		const session = loadSession();
		if (!session) return null;

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
				})
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
				} as SessionInfo & { last_update: string };
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
					joinCode: state.joinCode || ''
				};
				saveSession(session);
				state.playerId = message.player_id;
				state.playerName = message.name;
				state.roundDuration = message.round_duration;
				break;
			}

			case 'StateDelta': {
				// Update phase if provided.
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
					state.players = newPlayers;
				}

				// Update the current question if alternatives are provided.
				if (message.alternatives) {
					state.currentQuestion = {
						type: message.question_type || '',
						alternatives: message.alternatives
					};
				} else {
					state.currentQuestion = undefined;
				}

				// Perform phase-specific actions.
				const newPhase = (message.phase && message.phase.toLowerCase()) || '';
				if (newPhase === GamePhase.Question.toLowerCase()) {
					timerStore.startTimer();
				}
				if (newPhase === GamePhase.Score.toLowerCase()) {
					state.currentAnswers = [];
				}

				// Update the phase.
				state.phase = message.phase || state.phase;
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
						correct: message.correct,
						timestamp: Date.now()
					}
				];
				break;
			}

			case 'PlayerLeft': {
				info(`Player left: ${message.name}`);
				// Optionally remove the player from the players map or notify the UI.
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
		setPlayerId,
		setPlayerName,
		cleanup,
		clearError
	};
}

export const gameStore = createGameStore();
