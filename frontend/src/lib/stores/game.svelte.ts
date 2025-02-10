// src/lib/stores/game.ts

import { browser } from '$app/environment';
import { youtubeStore } from '$lib/stores/youtube-store.svelte';
import { timerStore } from '$lib/stores/timer-store.svelte';
import { info, warn } from '$lib/utils/logger';

import type { GameState, ServerMessage } from '../types/game';
import { GamePhase } from '../types/game';
import { PUBLIC_SPEKTRUM_SERVER_URL } from '$env/static/public';

/* ------------------------------------------------------------------
   Multi-session storage for localStorage
------------------------------------------------------------------ */
export interface SessionInfo {
	lobbyId: string;
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
	const sessions = loadSessions().filter(
		(s) => !(s.lobbyId === session.lobbyId && s.playerId === session.playerId)
	);
	sessions.push(session);
	localStorage.setItem('spektrumSessions', JSON.stringify(sessions));
}

function saveSessions(sessions: SessionInfo[]) {
	localStorage.setItem('spektrumSessions', JSON.stringify(sessions));
}

export function removeSession(lobbyId: string, playerId: string) {
	if (!browser) return;
	const sessions = loadSessions().filter(
		(s) => !(s.lobbyId === lobbyId && s.playerId === playerId)
	);
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
					lobby_id: sess.lobbyId,
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
		const validSet = new Set(data.valid_sessions.map((v) => v.lobby_id + '|' + v.player_id));

		// Filter out invalid sessions
		const validSessions = sessions.filter((sess) =>
			validSet.has(sess.lobbyId + '|' + sess.playerId)
		);

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
	adminId: undefined,
	lobbyId: undefined,
	roundDuration: 60,
	players: new Map(),
	currentAnswers: []
};

function createGameStore() {
	const state = $state<GameState>({ ...initialState });

	function cleanup() {
		info('Running cleanup...');
		if (state.lobbyId && state.playerId) {
			removeSession(state.lobbyId, state.playerId);
		}
		state.lobbyId = undefined;
		state.playerId = undefined;
		state.playerName = undefined;
		state.isAdmin = false;
		state.adminId = undefined;
		state.joinCode = undefined;
		// Reset state to initial state
		Object.assign(state, initialState);
	}

	async function checkSessions() {
		return await checkSessionsFromServer();
	}

	function processServerMessage(message: ServerMessage) {
		info('Handling server message:', message);

		if (message.type === 'GameClosed') {
			info('Game closed, cleaning up...');
			cleanup();
			return;
		}

		switch (message.type) {
			case 'JoinedLobby': {
				const time = new Date();
				const timeString = time.toLocaleTimeString('en-US', {
					hour12: false,
					hour: '2-digit',
					minute: '2-digit'
				});

				const session: SessionInfo = {
					lobbyId: message.lobby_id,
					playerId: message.player_id,
					playerName: message.name,
					joinCode: state.joinCode || '',
					createdAt: timeString
				};
				saveSession(session);

				state.lobbyId = message.lobby_id;
				state.playerId = message.player_id;
				state.playerName = message.name;
				state.roundDuration = message.round_duration;
				// Instead of always setting isAdmin to false,
				// check if the message includes an admin_id.
				state.players = new Map(
					message.players.map(([name, score]) => [
						name,
						{ name, score, roundScore: 0, hasAnswered: false, answer: null }
					])
				);
				break;
			}

			case 'ReconnectSuccess': {
				const storedSessions = loadSessions();
				state.phase = message.game_state.phase as GamePhase;
				state.currentQuestion = message.game_state.alternatives
					? {
						type: message.game_state.question_type,
						alternatives: message.game_state.alternatives
					}
					: undefined;
				state.players = new Map(
					message.game_state.scoreboard.map(([name, score]) => [
						name,
						{ name, score, roundScore: 0, hasAnswered: false, answer: null }
					])
				);
				break;
			}

			case 'StateChanged': {
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

				// Update round scores
				message.round_scores.forEach(([name, roundScore]) => {
					const player = newPlayers.get(name);
					if (player) {
						player.roundScore = roundScore;
					}
				});

				const newPhase = message.phase.toLowerCase() as GamePhase;
				youtubeStore.handlePhaseChange(newPhase);

				if (newPhase === 'question') {
					timerStore.startTimer();
				}

				state.phase = newPhase;
				state.players = newPlayers;

				if (newPhase === 'score') {
					state.currentAnswers = [];
				}

				state.currentQuestion = message.alternatives
					? {
						type: message.question_type,
						alternatives: message.alternatives
					}
					: undefined;
				break;
			}

			case 'GameOver': {
				state.phase = GamePhase.GameOver;
				state.upcomingQuestions = undefined;
				break;
			}

			case 'AdminInfo': {
				if (message.question) {
					const { youtube_id, title, artist } = message.question;
					info('Received song info:', { youtube_id, title, artist });
					if (youtube_id) {
						youtubeStore.loadVideo(youtube_id);
					}
				}
				state.currentSong = message.question
					? {
						songName: message.question.title,
						artist: message.question.artist,
						youtubeId: message.question.youtube_id
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

			case 'PlayerAnswered': {
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

			case 'Error':
				// Handle error case
				break;
		}
	}

	function setAdmin(adminId: string) {
		state.isAdmin = true;
		state.adminId = adminId;
	}

	function setJoinCode(joinCode: string) {
		state.joinCode = joinCode;
	}

	function setLobbyId(lobbyId: string) {
		state.lobbyId = lobbyId;
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
		setLobbyId,
		setPlayerId,
		setPlayerName,
		cleanup,
		clearError
	};
}

export const gameStore = createGameStore();
