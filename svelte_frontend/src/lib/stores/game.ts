// src/lib/stores/game.ts

import { writable, get } from 'svelte/store';
import { browser } from '$app/environment';
import { youtubeStore } from './youtube-store';

import type { GameState, ServerMessage } from '../types/game';
import { GamePhase } from '../types/game';
import { PUBLIC_SPEKTRUM_SERVER_URL } from '$env/static/public';

/* ------------------------------------------------------------------
   Multi-session storage for localStorage
------------------------------------------------------------------ */
interface SessionInfo {
    lobbyId: string;
    playerId: string;
    playerName: string;
    joinCode: string;
    createdAt: string;
}

function loadSessions(): SessionInfo[] {
    if (!browser) return [];
    try {
        const data = localStorage.getItem('spektrumSessions');
        return data ? JSON.parse(data) : [];
    } catch {
        return [];
    }
}

function saveSession(session: SessionInfo) {
    if (!browser) return;
    const sessions = loadSessions().filter(
        s => !(s.lobbyId === session.lobbyId && s.playerId === session.playerId)
    );
    sessions.push(session);
    localStorage.setItem('spektrumSessions', JSON.stringify(sessions));
}

function saveSessions(sessions: SessionInfo[]) {
    localStorage.setItem('spektrumSessions', JSON.stringify(sessions));
}

function removeSession(lobbyId: string, playerId: string) {
    if (!browser) return;
    const sessions = loadSessions().filter(
        s => !(s.lobbyId === lobbyId && s.playerId === playerId)
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
                sessions: sessions.map(sess => ({
                    lobby_id: sess.lobbyId,
                    player_id: sess.playerId
                }))
            })
        });
        if (!res.ok) {
            console.error('Failed to check sessions:', res.status, res.statusText);
            return sessions; // Return unfiltered on error
        }

        const data = await res.json() as {
            valid_sessions: Array<{ lobby_id: string; player_id: string }>;
        };
        console.log('Valid sessions:', data.valid_sessions);

        // Build a set of valid combos
        const validSet = new Set(
            data.valid_sessions.map(v => v.lobby_id + '|' + v.player_id)
        );

        // Filter out invalid sessions
        const validSessions = sessions.filter(sess =>
            validSet.has(sess.lobbyId + '|' + sess.playerId)
        );

        saveSessions(validSessions);
        return validSessions;
    } catch (err) {
        console.error('Error checking sessions:', err);
        return sessions; // Return unfiltered on fetch error
    }
}
/* ------------------------------------------------------------------
   Game store for the currently active session
------------------------------------------------------------------ */
const initialState: GameState = {
    phase: GamePhase.Lobby,
    isAdmin: false,
    adminId: undefined,
    lobbyId: undefined,
    roundDuration: 60,
    players: new Map(),
    currentAnswers: []
};

function createGameStore() {
    const store = writable<GameState>(initialState);
    const { subscribe, set, update } = store;

    /**
     * Public method to reset the store and remove the current active session
     * from localStorage. If you prefer to keep old sessions, remove `removeSession`.
     */
    function cleanup() {
        update(state => {
            console.log('Running cleanup...');
            // If we have an active session in memory, remove from localStorage
            if (state.lobbyId && state.playerId) {
                removeSession(state.lobbyId, state.playerId);
            }
            // You might also want to call your websocketStore.disconnect() from outside
            return initialState;
        });
    }

    return {
        subscribe,

        /**
                 * Optionally expose the helper that checks local sessions
                 * against the server, returning only valid ones.
                 */
        async checkSessions() {
            return await checkSessionsFromServer();
        },

        /**
         * Called by external code to handle an incoming message.
         * We do NOT import websocketStore here to avoid circular imports.
         */
        processServerMessage(message: ServerMessage) {
            console.log('Handling server message:', message);

            if (message.type === 'GameClosed') {
                console.log('Game closed, cleaning up...');
                cleanup();
                return;
            }

            update(state => {
                switch (message.type) {
                    case 'JoinedLobby': {
                        // We now know the actual playerId, so we can store a session.
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
                            joinCode: message.join_code ?? '',
                            createdAt: timeString
                        };
                        saveSession(session);

                        return {
                            ...state,
                            lobbyId: message.lobby_id,
                            playerId: message.player_id,
                            playerName: message.name,
                            roundDuration: message.round_duration,
                            joinCode: message.join_code,
                            isAdmin: false,
                            players: new Map(
                                message.players.map(([name, score]) => [
                                    name,
                                    { name, score, hasAnswered: false, answer: null }
                                ])
                            )
                        };
                    }

                    case 'ReconnectSuccess': {
                        // Possibly load from localStorage if needed
                        const storedSessions = loadSessions();
                        // If you want to re-check which session might apply, do so here.
                        // For now, we just read the single-lobby approach as before.

                        return {
                            ...state,
                            phase: message.game_state.phase as GamePhase,
                            currentQuestion: message.game_state.alternatives
                                ? {
                                    type: message.game_state.question_type,
                                    alternatives: message.game_state.alternatives
                                }
                                : undefined,
                            players: new Map(
                                message.game_state.scoreboard.map(([name, score]) => [
                                    name,
                                    { name, score, hasAnswered: false, answer: null }
                                ])
                            )
                        };
                    }

                    case 'StateChanged': {
                        const newPlayers = new Map(state.players);
                        message.scoreboard.forEach(([name, score]) => {
                            newPlayers.set(name, {
                                name,
                                score,
                                hasAnswered: false,
                                answer: null
                            });
                        });
                        const newPhase = message.phase.toLowerCase() as GamePhase;
                        youtubeStore.handlePhaseChange(newPhase);
                        return {
                            ...state,
                            phase: newPhase,
                            players: newPlayers,
                            currentAnswers: newPhase === 'score' ? [] : state.currentAnswers,
                            currentQuestion: message.alternatives
                                ? {
                                    type: message.question_type,
                                    alternatives: message.alternatives
                                }
                                : undefined
                        };
                    }

                    case 'GameOver':
                        return { ...state, phase: 'gameover' };

                    case 'AdminInfo': {
                        if (message.question) {
                            const { youtube_id, song_name, artist } = message.question;
                            console.log('Received song info:', { youtube_id, song_name, artist });
                            if (youtube_id) {
                                youtubeStore.loadVideo(youtube_id);
                            }
                        }
                        return {
                            ...state,
                            currentSong: message.question
                                ? {
                                    songName: message.question.song_name,
                                    artist: message.question.artist,
                                    youtubeId: message.question.youtube_id
                                }
                                : undefined
                        };
                    }

                    case 'AdminNextQuestions': {
                        const nextQuestion = message.upcoming_questions[0];
                        if (nextQuestion?.youtube_id) {
                            console.log('Loading next video:', nextQuestion.youtube_id);
                            youtubeStore.loadVideo(nextQuestion.youtube_id);
                        }
                        return {
                            ...state,
                            upcomingQuestions: message.upcoming_questions
                        };
                    }

                    case 'PlayerAnswered': {
                        if (state.currentAnswers.some(a => a.name === message.name)) {
                            return state;
                        }
                        return {
                            ...state,
                            currentAnswers: [
                                ...state.currentAnswers,
                                {
                                    name: message.name,
                                    correct: message.correct,
                                    timestamp: Date.now()
                                }
                            ]
                        };
                    }

                    case 'Error':
                        // Possibly remove the active session if reconnect fails
                        // removeSession(state.lobbyId, state.playerId);
                        return state;

                    default:
                        return state;
                }
            });
        },

        setAdmin: (adminId: string) => {
            update(state => ({ ...state, isAdmin: true, adminId }));
        },

        setJoinCode: (joinCode: string) => {
            update(state => ({ ...state, joinCode }));
        },

        setLobbyId: (lobbyId: string) => {
            update(state => ({ ...state, lobbyId }));
        },

        setPlayerId: (playerId: string) => {
            update(state => ({ ...state, playerId }));
        },

        setPlayerName: (playerName: string) => {
            update(state => ({ ...state, playerName }));
        },

        getState: () => get(store),
        cleanup,

        clearError: () => {
            update(state => ({ ...state, error: undefined }));
        }
    };
}

export const gameStore = createGameStore();
