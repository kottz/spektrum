// src/lib/stores/game-actions.ts
import { websocketStore } from './websocket';
import { gameStore } from './game';
import { youtubeStore } from './youtube-store';
import type { ClientMessage, AdminAction } from '../types/game';
import { GamePhase } from '../types/game';
import { PUBLIC_SPEKTRUM_SERVER_URL } from '$env/static/public';

class GameActions {
    public async joinGame(joinCode: string, playerName: string) {
        try {
            websocketStore.connect(joinCode, playerName);
        } catch (error) {
            console.error('Failed to join game:', error);
            throw error;
        }
    }

    public async createGame(playerName: string = 'Admin') {
        try {
            const response = await fetch(`${PUBLIC_SPEKTRUM_SERVER_URL}/api/lobbies`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    round_duration: 60,
                    lobby_name: "New Lobby"
                }),
            });

            if (!response.ok) {
                throw new Error('Failed to create lobby');
            }

            const data = await response.json();
            // Store both admin status and admin ID
            gameStore.setAdmin(data.admin_id);
            gameStore.setJoinCode(data.join_code);
            gameStore.setLobbyId(data.lobby_id);
            websocketStore.connect(data.join_code, playerName, data.admin_id);

            return data.join_code;
        } catch (error) {
            console.error('Failed to create game:', error);
            throw error;
        }
    }

    public submitAnswer(answer: string) {
        const state = gameStore.getState();
        if (!state.lobbyId) {
            console.error('No active lobby');
            return;
        }

        const message: ClientMessage = {
            type: 'Answer',
            lobby_id: state.lobbyId,
            answer
        };

        websocketStore.send(message);
    }

    private sendAdminAction(action: AdminAction) {
        const state = gameStore.getState();
        if (!state.lobbyId || !state.isAdmin) {
            console.error('Not authorized to perform admin action');
            return;
        }

        const message: ClientMessage = {
            type: 'AdminAction',
            lobby_id: state.lobbyId,
            action
        };

        console.log('Sending admin action:', message);
        websocketStore.send(message);
    }

    public startGame() {
        this.sendAdminAction({ type: 'StartGame' });
    }

    public startRound(specifiedAlternatives: string[] | null = null) {
        this.sendAdminAction({
            type: 'StartRound',
            specified_alternatives: specifiedAlternatives
        });
    }

    public endRound() {
        this.sendAdminAction({ type: 'EndRound' });
    }

    public skipQuestion() {
        this.sendAdminAction({ type: 'SkipQuestion' });
    }

    public endGame(reason: string = 'Game ended by admin') {
        this.sendAdminAction({
            type: 'EndGame',
            reason
        });
    }

    public closeGame(reason: string = 'Game closed by admin') {
        this.sendAdminAction({
            type: 'CloseGame',
            reason
        });
    }

    public leaveGame() {
        const state = gameStore.getState();
        if (!state.lobbyId) return;

        const message: ClientMessage = {
            type: 'Leave',
            lobby_id: state.lobbyId
        };

        websocketStore.send(message);
        gameStore.cleanup();
        youtubeStore.cleanup();
    }
}

// Export a single instance to be used throughout the app
export const gameActions = new GameActions();
