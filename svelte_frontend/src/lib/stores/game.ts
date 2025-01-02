// src/lib/stores/game.ts

import { writable, get } from 'svelte/store';
import { websocketStore } from './websocket';
import { browser } from '$app/environment';
import type { GameState, ServerMessage } from '../types/game';
import { GamePhase } from '../types/game';
import { youtubeStore } from './youtube-store';

// Utility functions for localStorage
function saveCredentials(lobbyId: string, playerId: string, joinCode: string, playerName: string) {
    if (!browser) return;
    localStorage.setItem('lobbyId', lobbyId);
    localStorage.setItem('playerId', playerId);
    localStorage.setItem('joinCode', joinCode);
    localStorage.setItem('playerName', playerName);
}

function clearCredentials() {
    if (!browser) return;
    localStorage.removeItem('lobbyId');
    localStorage.removeItem('playerId');
    localStorage.removeItem('joinCode');
    localStorage.removeItem('playerName');
}

// Initial client-side game state
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

    // Listen for new messages in the websocket store
    websocketStore.subscribe(ws => {
        if (ws.messages.length > 0) {
            const lastMessage = ws.messages[ws.messages.length - 1];
            handleServerMessage(lastMessage);
        }
    });

    function handleServerMessage(message: ServerMessage) {
        console.log('Handling server message:', message);

        // If the server announces the game is closed, clean up entirely
        if (message.type === 'GameClosed') {
            console.log('Game closed, cleaning up...');
            cleanup();
            return;
        }

        update(state => {
            switch (message.type) {
                case 'JoinedLobby': {
                    // Store credentials for reconnect
                    saveCredentials(
                        message.lobby_id,
                        message.player_id,
                        message.join_code ?? '',
                        message.name
                    );

                    return {
                        ...state,
                        lobbyId: message.lobby_id,
                        playerId: message.player_id,
                        playerName: message.name,
                        roundDuration: message.round_duration,
                        joinCode: message.join_code,
                        isAdmin: false, // Not admin unless set otherwise
                        players: new Map(
                            message.players.map(([name, score]) => [
                                name,
                                {
                                    name,
                                    score,
                                    hasAnswered: false,
                                    answer: null
                                }
                            ])
                        )
                    };
                }

                // If the backend supports 'ReconnectSuccess', handle that here
                case 'ReconnectSuccess': {
                    // Attempt to restore the same lobbyId/playerId from localStorage
                    const storedLobbyId = browser ? localStorage.getItem('lobbyId') : null;
                    const storedPlayerId = browser ? localStorage.getItem('playerId') : null;
                    const storedPlayerName = browser ? localStorage.getItem('playerName') : null;

                    return {
                        ...state,
                        // If either is missing in localStorage, at least keep the old state
                        lobbyId: storedLobbyId ?? state.lobbyId,
                        playerId: storedPlayerId ?? state.playerId,
                        playerName: storedPlayerName ?? state.playerName,

                        // Now apply the rest of the game_state details from the message
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
                                {
                                    name,
                                    score,
                                    hasAnswered: false,
                                    answer: null
                                }
                            ])
                        )
                    };
                }

                case 'StateChanged': {
                    console.log('State changed:', message);
                    // Update players from the scoreboard
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

                    // Handle video playback based on phase
                    youtubeStore.handlePhaseChange(newPhase);

                    return {
                        ...state,
                        phase: newPhase,
                        players: newPlayers,
                        // Clear answers when entering score phase
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
                    return {
                        ...state,
                        phase: 'gameover'
                    };

                case 'AdminInfo': {
                    // Optionally load video from message.question
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
                    // Load the next video for preview
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
                    // Avoid adding duplicate answers
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
                    // If a reconnect attempt fails, clear credentials so we don't loop
                    clearCredentials();
                    return state;

                default:
                    return state;
            }
        });
    }

    function cleanup() {
        console.log('Running cleanup...');
        clearCredentials(); // Clear localStorage
        websocketStore.disconnect();
        set(initialState);
    }

    return {
        subscribe,

        setAdmin: (adminId: string) => {
            update(state => ({
                ...state,
                isAdmin: true,
                adminId
            }));
        },

        setJoinCode: (joinCode: string) => {
            update(state => ({
                ...state,
                joinCode
            }));
        },

        setLobbyId: (lobbyId: string) => {
            update(state => ({
                ...state,
                lobbyId
            }));
        },

        getState: () => get(store),
        cleanup,

        clearError: () => {
            update(state => ({
                ...state,
                error: undefined
            }));
        }
    };
}

export const gameStore = createGameStore();
