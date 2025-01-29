// src/lib/stores/websocket.ts

import type { ServerMessage, ClientMessage } from '../types/game';
import { PUBLIC_SPEKTRUM_WS_SERVER_URL } from '$env/static/public';
import { gameStore } from '$lib/stores/game.svelte';
import { info, warn } from '$lib/utils/logger';

interface WebSocketState {
	connected: boolean;
	messages: ServerMessage[];
	error: string | null;
	isConnecting: boolean;
}

function createWebSocketStore() {
	const state = $state<WebSocketState>({
		connected: false,
		messages: [],
		error: null,
		isConnecting: false
	});

	let socket: WebSocket | null = null;

	// For exponential backoff in auto-reconnect
	let reconnectAttempts = 0;
	let reconnectTimeout: number | null = null;
	const MAX_RECONNECT_ATTEMPTS = 3;

	function clearReconnectTimeout() {
		if (reconnectTimeout) {
			window.clearTimeout(reconnectTimeout);
			reconnectTimeout = null;
		}
	}

	function connect(joinCode?: string, playerName?: string, adminId?: string): Promise<void> {
		return new Promise((resolve, reject) => {
			const connectionTimeout = setTimeout(() => {
				if (socket) {
					socket.close();
				}
				reject(new Error("Connection timeout - couldn't reach the server"));
			}, 5000);

			disconnect();
			state.isConnecting = true;

			const wsUrl = PUBLIC_SPEKTRUM_WS_SERVER_URL;
			socket = new WebSocket(wsUrl);

			socket.onopen = () => {
				clearTimeout(connectionTimeout);
				info('WebSocket connected');
				reconnectAttempts = 0;

				state.connected = true;
				state.error = null;
				state.isConnecting = false;

				if (joinCode && playerName) {
					const joinMsg: ClientMessage = {
						type: 'JoinLobby',
						join_code: joinCode,
						name: playerName,
						admin_id: adminId
					};
					send(joinMsg);
				} else {
					const { lobbyId, playerId } = $derived(gameStore);
					if (lobbyId && playerId) {
						info('Attempting an automatic Reconnect...');
						const reconnectMsg: ClientMessage = {
							type: 'Reconnect',
							lobby_id: lobbyId,
							player_id: playerId
						};
						send(reconnectMsg);
					}
				}
				resolve();
			};

			socket.onmessage = (event) => {
				try {
					const message = JSON.parse(event.data) as ServerMessage;
					info('Received message:', message);

					gameStore.processServerMessage(message);

					if (message.type === 'Error') {
						reject(new Error(message.code || 'Server error'));
						return;
					}

					state.error = null;
				} catch (e) {
					warn('Failed to parse message:', e);
					const error = new Error('Failed to parse server message');
					state.error = error.message;
					reject(error);
				}
			};

			socket.onclose = (event) => {
				clearTimeout(connectionTimeout);
				info('WebSocket closed:', event);
				const wasConnected = state.connected;

				state.connected = false;
				state.isConnecting = false;

				if (state.isConnecting) {
					reject(new Error('Connection closed before it could be established'));
					return;
				}

				if (wasConnected && event.code !== 1000) {
					attemptReconnect();
				}
			};

			socket.onerror = (error) => {
				clearTimeout(connectionTimeout);
				warn('WebSocket error:', error);
				const errorMessage = 'Failed to connect to game server';

				state.error = errorMessage;
				state.isConnecting = false;

				reject(new Error(errorMessage));
			};
		});
	}

	function attemptReconnect() {
		if (reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
			state.error = 'Failed to reconnect after multiple attempts';
			return;
		}

		const { lobbyId, playerId } = $derived(gameStore);
		if (!lobbyId || !playerId) {
			info('No valid lobby/player in gameStore, skipping auto-reconnect');
			return;
		}

		reconnectAttempts++;
		const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), 10000);

		clearReconnectTimeout();
		reconnectTimeout = window.setTimeout(() => {
			info(`Attempting reconnect #${reconnectAttempts}...`);
			connect();
		}, delay);
	}

	function send(message: ClientMessage) {
		if (socket?.readyState === WebSocket.OPEN) {
			info('Sending message:', message);
			socket.send(JSON.stringify(message));
		} else {
			warn('Cannot send message: connection not open');
			state.error = 'Connection not available';
		}
	}

	function disconnect() {
		clearReconnectTimeout();
		reconnectAttempts = 0;

		if (socket) {
			socket.close(1000, 'Client disconnecting');
			socket = null;
		}

		state.connected = false;
		state.messages = [];
		state.error = null;
		state.isConnecting = false;
	}

	function clearError() {
		state.error = null;
	}

	return {
		state,
		connect,
		send,
		disconnect,
		clearError
	};
}

export const websocketStore = createWebSocketStore();
