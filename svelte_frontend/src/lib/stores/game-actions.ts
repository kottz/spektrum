// src/lib/stores/game-actions.ts

import { websocketStore } from './websocket';
import { gameStore } from './game';
import { youtubeStore } from './youtube-store';
import type { ClientMessage, AdminAction } from '../types/game';
import { PUBLIC_SPEKTRUM_SERVER_URL } from '$env/static/public';

class GameActions {
    /**
     * Join an existing lobby with a given joinCode and playerName.
     * The store will handle connecting and storing credentials if successful.
     */
    public async joinGame(joinCode: string, playerName: string) {
        try {
            await websocketStore.connect(joinCode, playerName);
        } catch (error) {
            console.error('Failed to join game:', error);
            throw error;
        }
    }

    /**
     * Create a new lobby, then automatically join it as admin.
     */
    public async createGame(playerName: string = 'Admin') {
        try {
            const response = await fetch(`${PUBLIC_SPEKTRUM_SERVER_URL}/api/lobbies`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    round_duration: 60,
                    lobby_name: 'New Lobby'
                })
            });

            if (!response.ok) {
                throw new Error('Failed to create lobby');
            }

            const data = await response.json();
            // Store admin info and connect as admin
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

    /**
     * Attempt to reconnect using any credentials stored in localStorage.
     * If the server recognizes the lobby/player, it will restore the session.
     */
    public reconnectGame() {
        console.log('Attempting to reconnect...');
        websocketStore.connect();
    }


    /**
     * Submit an answer to the current question.
     */
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

    /**
     * Helper to send an admin action if the user is authorized as admin.
     */
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

    /**
     * Leave the current game (if any), then clean up local state and YouTube player.
     */
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

export const gameActions = new GameActions();
