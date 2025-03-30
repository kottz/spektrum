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
	 * Join an existing lobby with a given player ID.
	 */
	public async joinGame(playerId: string) {
		try {
			await websocketStore.connect(playerId);
		} catch (error) {
			warn('Failed to join game:', error);
			throw error;
		}
	}

	/**
	 * Create a new lobby, then automatically join it as admin.
	 */
	public async createGame(set: number | null = null) {
		try {
			const response = await fetch(`${PUBLIC_SPEKTRUM_SERVER_URL}/api/create-lobby`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ round_duration: 60, set_id: set })
			});
			if (!response.ok) {
				throw new Error('Failed to create lobby');
			}
			const data = await response.json();

			// IMPORTANT: Remove the previously saved session for this lobby/player.
			removeSession();

			await websocketStore.connect(data.player_id);

			gameStore.state.isAdmin = true;
			gameStore.state.joinCode = data.join_code;

			return data.join_code;
		} catch (error) {
			warn('Failed to create game:', error);
			throw error;
		}
	}

	/**
	 * Attempt to reconnect using any credentials stored in localStorage.
	 */
	public reconnectGame(playerId: string) {
		info('Attempting to reconnect...');
		websocketStore.connect(playerId);
	}

	/**
	 * Submit an answer to the current question.
	 */
	public submitAnswer(answer: string) {
		// Ensure we have a valid player ID (stored after joining).
		if (!gameStore.state.playerId) {
			warn('No active player');
			return;
		}

		const message: ClientMessage = {
			type: 'Answer',
			answer
		};
		websocketStore.send(message);
	}

	/**
	 * Helper to send an admin action if the user is authorized as admin.
	 */
	private sendAdminAction(action: AdminAction) {
		if (!gameStore.state.isAdmin) {
			warn('Not authorized to perform admin action');
			return;
		}

		const message: ClientMessage = {
			type: 'AdminAction',
			action
		};
		info('Sending admin action:', message);
		websocketStore.send(message);
	}

	public startGame() {
		this.sendAdminAction({ type: 'StartGame' });
	}

	public startRound() {
		this.sendAdminAction({ type: 'StartRound' });
	}

	public endRound() {
		timerStore.stopTimer();
		this.sendAdminAction({ type: 'EndRound' });
	}

	public skipQuestion() {
		this.sendAdminAction({ type: 'SkipQuestion' });
	}

	public kickPlayer(playerName: string) {
		this.sendAdminAction({ type: 'KickPlayer', player_name: playerName });
	}

	public endGame(reason: string = 'Game ended by admin') {
		this.sendAdminAction({ type: 'EndGame', reason });
	}

	public closeGame(reason: string = 'Game closed by admin') {
		this.sendAdminAction({ type: 'CloseGame', reason });
	}

	/**
	 * Leave the current game (if any), then clean up local state and YouTube player.
	 */
	public leaveGame() {
		// Ensure we have a valid player ID.
		if (!gameStore.state.playerId) return;

		const message: ClientMessage = {
			type: 'Leave'
		};
		websocketStore.send(message);
		gameStore.cleanup();
		youtubeStore.cleanup();
	}
}

export const gameActions = new GameActions();
