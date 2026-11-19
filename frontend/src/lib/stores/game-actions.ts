import { websocketStore } from './websocket.svelte';
import { gameStore } from '#lib/stores/game.svelte.ts';
import { youtubeStore } from '#lib/stores/youtube-store.svelte.ts';
import { info, warn } from '#lib/utils/logger.ts';
import { notifications } from '#lib/stores/notification-store.svelte.ts';
import type { ClientMessage } from '#lib/generated/ClientMessage.ts';
import type { AdminAction } from '#lib/generated/AdminAction.ts';
import { api } from '#lib/api.ts';
import { removeSession } from '#lib/stores/game.svelte.ts';

class GameActions {
	/**
	 * Join an existing lobby with a given session token.
	 */
	public async joinGame(sessionToken: string) {
		try {
			await websocketStore.connect(sessionToken);
		} catch (error) {
			warn('Failed to join game:', error);
			throw error;
		}
	}

	/**
	 * Join a lobby by its code as a new player, then connect.
	 */
	public async joinLobby(joinCode: string, name: string) {
		const data = await api.joinLobby({ join_code: joinCode, name });
		gameStore.setPlayerName(name);
		gameStore.setJoinCode(data.join_code);
		gameStore.setSessionToken(data.session_token);
		gameStore.setPlayerId(data.player_id);
		await this.joinGame(data.session_token);
	}

	/**
	 * Create a new lobby, then automatically join it as admin.
	 */
	public async createGame(set: number | null = null) {
		try {
			const data = await api.createLobby({ round_duration: 60, set_id: set });

			// IMPORTANT: Remove the previously saved session for this lobby/player.
			removeSession();

			gameStore.setAdminTo(true);
			gameStore.setJoinCode(data.join_code);
			gameStore.setSessionToken(data.session_token);
			gameStore.setPlayerId(data.player_id);

			await websocketStore.connect(data.session_token);

			return data.join_code;
		} catch (error) {
			warn('Failed to create game:', error);
			throw error;
		}
	}

	/**
	 * Sends a message, telling the user when it could not go out (e.g. while
	 * reconnecting) instead of letting the UI assume it did.
	 */
	private send(message: ClientMessage): boolean {
		const sent = websocketStore.send(message);
		if (!sent) {
			notifications.add('Not connected — please try again in a moment', 'destructive');
		}
		return sent;
	}

	/**
	 * Submit an answer to the current question. Returns whether it was sent.
	 */
	public submitAnswer(answer: string): boolean {
		if (!gameStore.state.playerId) {
			warn('No active player');
			return false;
		}
		return this.send({ type: 'Answer', answer });
	}

	private sendAdminAction(action: AdminAction): boolean {
		if (!gameStore.state.isAdmin) {
			warn('Not authorized to perform admin action');
			return false;
		}
		info('Sending admin action:', action);
		return this.send({ type: 'AdminAction', action });
	}

	public startGame() {
		this.sendAdminAction({ type: 'StartGame' });
	}

	public startRound() {
		this.sendAdminAction({ type: 'StartRound' });
	}

	public endRound() {
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

	public lockLobby(locked: boolean) {
		this.sendAdminAction({ type: 'LockLobby', locked });
	}

	/**
	 * Leave the current game (if any), then clean up local state and YouTube player.
	 */
	public leaveGame() {
		// Ensure we have a valid player ID.
		if (!gameStore.state.playerId) return;

		websocketStore.send({ type: 'Leave' });
		websocketStore.disconnect();
		gameStore.cleanup();
		youtubeStore.cleanup();
	}
}

export const gameActions = new GameActions();
