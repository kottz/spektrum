// src/lib/stores/websocket.ts

import { writable, get } from 'svelte/store';
import type { ServerMessage, ClientMessage } from '../types/game';
import { PUBLIC_SPEKTRUM_WS_SERVER_URL } from '$env/static/public';
import { gameStore } from './game'; // We'll read current lobby/player from here
import { info, warn } from '$lib/utils/logger';

// Define our store shape
interface WebSocketState {
	connected: boolean;
	messages: ServerMessage[];
	error: string | null;
	isConnecting: boolean;
}

function createWebSocketStore() {
	const { subscribe, set, update } = writable<WebSocketState>({
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

	/**
	 * Connect to the backend WebSocket server.
	 * Optionally pass a joinCode/playerName/adminId if you
	 * want to do a fresh JoinLobby. Otherwise, if the gameStore
	 * already has (lobbyId, playerId), we can attempt a Reconnect.
	 */
	function connect(joinCode?: string, playerName?: string, adminId?: string): Promise<void> {
		return new Promise((resolve, reject) => {
			// Set a connection timeout
			const connectionTimeout = setTimeout(() => {
				if (socket) {
					socket.close();
				}
				reject(new Error("Connection timeout - couldn't reach the server"));
			}, 5000); // 5 second timeout

			// Clean up any existing connection
			disconnect();
			update((state) => ({ ...state, isConnecting: true }));

			const wsUrl = PUBLIC_SPEKTRUM_WS_SERVER_URL;
			socket = new WebSocket(wsUrl);

			socket.onopen = () => {
				clearTimeout(connectionTimeout);
				info('WebSocket connected');
				reconnectAttempts = 0;
				update((state) => ({
					...state,
					connected: true,
					error: null,
					isConnecting: false
				}));

				// If we were passed explicit join data, do a JoinLobby
				if (joinCode && playerName) {
					const joinMsg: ClientMessage = {
						type: 'JoinLobby',
						join_code: joinCode,
						name: playerName,
						admin_id: adminId
					};
					send(joinMsg);
				} else {
					// Otherwise, see if gameStore already has a (lobbyId, playerId)
					const { lobbyId, playerId } = get(gameStore);
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
					update((state) => ({
						...state,
						messages: [...state.messages, message]
					}));

					// If we receive an error message from the server
					if (message.type === 'Error') {
						reject(new Error(message.error || 'Server error'));
						return;
					}

					update((state) => ({ ...state, error: null }));
				} catch (e) {
					warn('Failed to parse message:', e);
					const error = new Error('Failed to parse server message');
					update((state) => ({ ...state, error: error.message }));
					reject(error);
				}
			};

			socket.onclose = (event) => {
				clearTimeout(connectionTimeout);
				info('WebSocket closed:', event);
				const wasConnected = get({ subscribe }).connected;
				update((state) => ({
					...state,
					connected: false,
					isConnecting: false
				}));

				// If this happens during initial connection, reject the promise
				if (get({ subscribe }).isConnecting) {
					reject(new Error('Connection closed before it could be established'));
					return;
				}

				// For existing connections, attempt reconnect
				if (wasConnected && event.code !== 1000) {
					attemptReconnect();
				}
			};

			socket.onerror = (error) => {
				clearTimeout(connectionTimeout);
				warn('WebSocket error:', error);
				const errorMessage = 'Failed to connect to game server';
				update((state) => ({
					...state,
					error: errorMessage,
					isConnecting: false
				}));
				reject(new Error(errorMessage));
			};
		});
	}

	/**
	 * If reconnectAttempts < MAX, increment and retry connecting
	 * with the same gameStore data (if present).
	 */
	function attemptReconnect() {
		if (reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
			update((state) => ({
				...state,
				error: 'Failed to reconnect after multiple attempts'
			}));
			return;
		}

		// Check if there's an actual game in progress
		const { lobbyId, playerId } = get(gameStore);
		if (!lobbyId || !playerId) {
			info('No valid lobby/player in gameStore, skipping auto-reconnect');
			return;
		}

		reconnectAttempts++;
		const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), 10000);

		clearReconnectTimeout();
		reconnectTimeout = window.setTimeout(() => {
			info(`Attempting reconnect #${reconnectAttempts}...`);
			// Call connect() with no new join data so we do a Reconnect automatically in onopen
			connect();
		}, delay);
	}

	/**
	 * Send a message if the socket is open.
	 */
	function send(message: ClientMessage) {
		if (socket?.readyState === WebSocket.OPEN) {
			info('Sending message:', message);
			socket.send(JSON.stringify(message));
		} else {
			warn('Cannot send message: connection not open');
			update((state) => ({
				...state,
				error: 'Connection not available'
			}));
		}
	}

	/**
	 * Clean up the connection.
	 */
	function disconnect() {
		clearReconnectTimeout();
		reconnectAttempts = 0;

		if (socket) {
			socket.close(1000, 'Client disconnecting');
			socket = null;
		}

		set({
			connected: false,
			messages: [],
			error: null,
			isConnecting: false
		});
	}

	return {
		subscribe,
		connect,
		send,
		disconnect,
		clearError: () => update((state) => ({ ...state, error: null }))
	};
}

export const websocketStore = createWebSocketStore();
