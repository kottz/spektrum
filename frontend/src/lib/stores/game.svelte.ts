// src/lib/stores/game.ts

import { browser } from '$app/environment';
import { youtubeStore } from '$lib/stores/youtube-store.svelte';
import { timerStore } from '$lib/stores/timer-store.svelte';
import { info, warn } from '$lib/utils/logger';

import type { GameState, GameUpdate } from '../types/game';
import { GamePhase } from '../types/game';
import { PUBLIC_SPEKTRUM_SERVER_URL } from '$env/static/public';

/* ------------------------------------------------------------------
   Multi-session storage for localStorage
------------------------------------------------------------------ */
export interface SessionInfo {
	playerId: string;
	playerName: string;
	joinCode: string;
	createdAt: string;
}

export function loadSessions(): SessionInfo[] {
	if (!browser) return [];
	try {
		const data = localStorage.getItem('spektrumSessions');
		return data ? JSON.parse(data) : [];
	} catch {
		return [];
	}
}

export function saveSession(session: SessionInfo) {
	if (!browser) return;
	const sessions = loadSessions().filter((s) => !(s.playerId === session.playerId));
	sessions.push(session);
	localStorage.setItem('spektrumSessions', JSON.stringify(sessions));
}

function saveSessions(sessions: SessionInfo[]) {
	localStorage.setItem('spektrumSessions', JSON.stringify(sessions));
}

export function removeSession(playerId: string) {
	if (!browser) return;
	const sessions = loadSessions().filter((s) => !(s.playerId === playerId));
	localStorage.setItem('spektrumSessions', JSON.stringify(sessions));
}

/**
 * Optional helper to remove any invalid sessions by asking the server.
 */
async function checkSessionsFromServer(): Promise<SessionInfo[]> {
	if (!browser) return [];

	const sessions = loadSessions();
	if (sessions.length === 0) return [];

	try {
		const res = await fetch(`${PUBLIC_SPEKTRUM_SERVER_URL}/api/check-sessions`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				sessions: sessions.map((sess) => ({
					player_id: sess.playerId
				}))
			})
		});
		if (!res.ok) {
			warn('Failed to check sessions:', res.status, res.statusText);
			return sessions; // Return unfiltered on error
		}

		const data = (await res.json()) as {
			valid_sessions: Array<{ lobby_id: string; player_id: string }>;
		};
		info('Valid sessions:', data.valid_sessions);

		// Build a set of valid combos
		const validSet = new Set(data.valid_sessions.map((v) => v.player_id));

		// Filter out invalid sessions
		const validSessions = sessions.filter((sess) => validSet.has(sess.playerId));

		saveSessions(validSessions);
		return validSessions;
	} catch (err) {
		warn('Error checking sessions:', err);
		return sessions; // Return unfiltered on fetch error
	}
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
		if (state.playerId) {
			removeSession(state.playerId);
		}
		state.playerId = undefined;
		state.playerName = undefined;
		state.isAdmin = false;
		state.joinCode = undefined;
		// Reset state to initial values.
		Object.assign(state, initialState);
	}

	async function checkSessions() {
		return await checkSessionsFromServer();
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
				// The Connected message is sent after a successful join.
				const time = new Date();
				const timeString = time.toLocaleTimeString('en-US', {
					hour12: false,
					hour: '2-digit',
					minute: '2-digit'
				});
				// Create a session using stored joinCode and lobbyId.
				const session: SessionInfo = {
					playerId: message.player_id,
					playerName: message.name,
					joinCode: state.joinCode || '',
					createdAt: timeString
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
				break;
			}

			default:
				warn('Unhandled message type:', message);
		}
	}

	function setAdmin() {
		state.isAdmin = true;
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
		processServerMessage,
		setAdmin,
		setJoinCode,
		setPlayerId,
		setPlayerName,
		cleanup,
		clearError
	};
}

export const gameStore = createGameStore();
