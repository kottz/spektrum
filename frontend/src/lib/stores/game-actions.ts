// src/lib/stores/game-actions.ts
import { websocketStore } from './websocket.svelte';
import { gameStore } from '$lib/stores/game.svelte';
import { youtubeStore } from '$lib/stores/youtube-store.svelte';
import { timerStore } from '$lib/stores/timer-store.svelte';
import { info, warn } from '$lib/utils/logger';
import type { ClientMessage, AdminAction } from '../types/game';
import { PUBLIC_SPEKTRUM_SERVER_URL } from '$env/static/public';
import { removeSession } from '$lib/stores/game.svelte';

class GameActions {
	/**
	 * Join an existing lobby with a given joinCode and playerName.
	 * The store will handle connecting and storing credentials if successful.
	 */
	public async joinGame(joinCode: string, playerName: string) {
		try {
			await websocketStore.connect(joinCode, playerName);
		} catch (error) {
			warn('Failed to join game:', error);
			throw error;
		}
		gameStore.state.joinCode = joinCode;
	}

	/**
	 * Create a new lobby, then automatically join it as admin.
	 */
	public async createGame(playerName: string = 'Admin', set: number | null = null) {
		try {
			const response = await fetch(`${PUBLIC_SPEKTRUM_SERVER_URL}/api/lobbies`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ round_duration: 60, set_id: set })
			});
			if (!response.ok) {
				throw new Error('Failed to create lobby');
			}
			const data = await response.json();

			// Set admin flag and update game store state
			gameStore.state.isAdmin = true;
			gameStore.state.adminId = data.admin_id;
			gameStore.state.joinCode = data.join_code;
			gameStore.state.lobbyId = data.lobby_id;

			// IMPORTANT: Remove any previously saved session for this lobby/player.
			removeSession(data.lobby_id, data.admin_id);

			// Now, pass the admin_id to the websocket connect call
			await websocketStore.connect(data.join_code, playerName, data.admin_id);
			return data.join_code;
		} catch (error) {
			warn('Failed to create game:', error);
			throw error;
		}
	}

	/**
	 * Attempt to reconnect using any credentials stored in localStorage.
	 */
	public reconnectGame() {
		info('Attempting to reconnect...');
		websocketStore.connect();
	}

	/**
	 * Submit an answer to the current question.
	 */
	public submitAnswer(answer: string) {
		const { lobbyId } = gameStore.state;
		if (!lobbyId) {
			warn('No active lobby');
			return;
		}

		const message: ClientMessage = {
			type: 'Answer',
			lobby_id: lobbyId,
			answer
		};
		websocketStore.send(message);
	}

	/**
	 * Helper to send an admin action if the user is authorized as admin.
	 */
	private sendAdminAction(action: AdminAction) {
		const { lobbyId, isAdmin } = gameStore.state;
		if (!lobbyId || !isAdmin) {
			warn('Not authorized to perform admin action');
			return;
		}

		const message: ClientMessage = {
			type: 'AdminAction',
			lobby_id: lobbyId,
			action
		};
		info('Sending admin action:', message);
		websocketStore.send(message);
	}

	public startGame() {
		this.sendAdminAction({ type: 'StartGame' });
	}

	public startRound(specifiedAlternatives: string[] | undefined = undefined) {
		this.sendAdminAction({
			type: 'StartRound',
			specified_alternatives: specifiedAlternatives
		});
	}

	public endRound() {
		timerStore.stopTimer();
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
		const { lobbyId } = gameStore.state;
		if (!lobbyId) return;

		const message: ClientMessage = {
			type: 'Leave',
			lobby_id: lobbyId
		};
		websocketStore.send(message);
		gameStore.cleanup();
		youtubeStore.cleanup();
	}
}

export const gameActions = new GameActions();
