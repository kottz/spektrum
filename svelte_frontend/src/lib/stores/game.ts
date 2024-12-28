import { writable, derived, get } from 'svelte/store';
import { websocketStore } from './websocket';
import { browser } from '$app/environment';
import type { GameState, ServerMessage } from '../types/game';
import { GamePhase } from '../types/game';
import { youtubeStore } from './youtube-store';

const initialState: GameState = {
    phase: GamePhase.Lobby,
    isAdmin: false,
    adminId: undefined,
    lobbyId: undefined,
    roundDuration: 60,
    players: new Map(),
    currentAnswers: [],
};

function createGameStore() {
    const store = writable<GameState>(initialState);
    const { subscribe, set } = store;

    // Subscribe to WebSocket messages
    websocketStore.subscribe(ws => {
        if (ws.messages.length > 0) {
            const lastMessage = ws.messages[ws.messages.length - 1];
            handleServerMessage(lastMessage);
        }
    });

    function handleServerMessage(message: ServerMessage) {
        console.log('Handling server message:', message);

        if (message.type === 'GameClosed') {
            console.log('Game closed, cleaning up...');
            cleanup();
            return;
        }

        store.update(state => {
            switch (message.type) {
                case 'JoinedLobby':
                    return {
                        ...state,
                        lobbyId: message.lobby_id,
                        playerId: message.player_id,
                        playerName: message.name,
                        roundDuration: message.round_duration,
                        joinCode: message.join_code,
                        isAdmin: false, // Ensure we're not admin when joining
                        players: new Map(message.players.map(([name, score]) => [
                            name,
                            {
                                name,
                                score,
                                hasAnswered: false,
                                answer: null
                            }
                        ]))
                    };
                    case 'StateChanged': {
                        console.log('State changed:', message);
                        // Update players from scoreboard
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
                            currentQuestion: message.alternatives ? {
                                type: message.question_type,
                                alternatives: message.alternatives
                            } : undefined
                        };
                    }
                    return {
                        ...state,
                        phase: message.phase.toLowerCase() as GamePhase,
                        players: newPlayers,
                        // Clear answers when entering score phase (round ended)
                        currentAnswers: message.phase.toLowerCase() === 'score' ? [] : state.currentAnswers,
                        currentQuestion: message.alternatives ? {
                            type: message.question_type,
                            alternatives: message.alternatives
                        } : undefined
                    };
                case 'GameOver':
                    return {
                        ...state,
                        phase: 'gameover'
                    };
                case 'AdminInfo': {
                    // Update game state
                    if (message.question) {
                        const { youtube_id, song_name, artist } = message.question;
                        console.log('Received song info:', { youtube_id, song_name, artist });
                        
                        // Load the video in YouTube player
                        if (youtube_id) {
                            youtubeStore.loadVideo(youtube_id);
                        }
                    }
                    return {
                        ...state,
                        currentSong: message.question ? {
                            songName: message.question.song_name,
                            artist: message.question.artist,
                            youtubeId: message.question.youtube_id
                        } : undefined
                    };
                }
                case 'AdminNextQuestions': {
                    // Load the next video regardless of current phase
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
                case 'PlayerAnswered':
                    // Don't add duplicate answers
                    if (state.currentAnswers?.some(a => a.name === message.name)) {
                        return state;
                    }
                    return {
                        ...state,
                        currentAnswers: [
                            ...(state.currentAnswers || []),
                            {
                                name: message.name,
                                correct: message.correct,
                                timestamp: Date.now()
                            }
                        ]
                    };
                default:
                    return state;
            }
        });
    }

    function cleanup() {
        console.log('Running cleanup...');
        if (browser) {
            localStorage.removeItem('gameState');
        }
        websocketStore.disconnect();
        set(initialState);
    }

    return {
        subscribe,
        setAdmin: (adminId: string) => {
            store.update(state => ({
                ...state,
                isAdmin: true,
                adminId
            }));
        },
        setJoinCode: (joinCode: string) => {
            store.update(state => ({
                ...state,
                joinCode
            }));
        },
        setLobbyId: (lobbyId: string) => {
            store.update(state => ({
                ...state,
                lobbyId
            }));
        },
        getState: () => get(store),
        cleanup,
        clearError: () => {
            store.update(state => ({
                ...state,
                error: undefined
            }));
        }
    };
}

export const gameStore = createGameStore();
