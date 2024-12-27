import { writable, derived, get } from 'svelte/store';
import { websocketStore } from './websocket';
import { browser } from '$app/environment';
import type { GameState, ServerMessage } from '../types/game';
import { GamePhase } from '../types/game';

const initialState: GameState = {
    phase: GamePhase.Lobby,
    isAdmin: false,
    adminId: undefined,
    lobbyId: undefined,
    roundDuration: 60,
    players: new Map(),
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
                case 'StateChanged':
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
                    return {
                        ...state,
                        phase: message.phase.toLowerCase() as GamePhase, // Convert to proper phase enum
                        players: newPlayers,
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
