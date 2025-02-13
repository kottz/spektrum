// src/lib/stores/websocket.ts

import type { GameUpdate, ClientMessage } from '../types/game';
import { PUBLIC_SPEKTRUM_WS_SERVER_URL } from '$env/static/public';
import { gameStore } from '$lib/stores/game.svelte';
import { info, warn } from '$lib/utils/logger';

export enum ConnectionState {
	INITIAL = 'INITIAL', // Never connected
	CONNECTING = 'CONNECTING', // First connection attempt
	CONNECTED = 'CONNECTED', // Successfully connected
	DISCONNECTED = 'DISCONNECTED', // Gracefully disconnected by client
	ERROR = 'ERROR', // Disconnected due to error
	RECONNECTING = 'RECONNECTING' // Attempting to reconnect
}

interface ReconnectMetadata {
	attempts: number;
	maxAttempts: number;
	nextAttemptTime: number | null;
	backoffDelay: number;
}

interface WebSocketState {
	connectionState: ConnectionState;
	messages: GameUpdate[];
	error: string | null;
	lastConnectedAt: number | null;
	lastDisconnectedAt: number | null;
	reconnectInfo: ReconnectMetadata;
}

function createWebSocketStore() {
	const state = $state<WebSocketState>({
		connectionState: ConnectionState.INITIAL,
		messages: [],
		error: null,
		lastConnectedAt: null,
		lastDisconnectedAt: null,
		reconnectInfo: {
			attempts: 0,
			maxAttempts: 3,
			nextAttemptTime: null,
			backoffDelay: 0
		}
	});

	let socket: WebSocket | null = null;
	let reconnectTimeout: number | null = null;
	const MAX_RECONNECT_ATTEMPTS = 3;
	const INITIAL_BACKOFF_DELAY = 1000; // 1 second

	function clearReconnectTimeout() {
		if (reconnectTimeout) {
			window.clearTimeout(reconnectTimeout);
			reconnectTimeout = null;
			state.reconnectInfo.nextAttemptTime = null;
		}
	}

	function updateReconnectMetadata(attemptsMade: number) {
		const backoffDelay = Math.min(INITIAL_BACKOFF_DELAY * Math.pow(2, attemptsMade), 10000);

		state.reconnectInfo = {
			attempts: attemptsMade,
			maxAttempts: MAX_RECONNECT_ATTEMPTS,
			nextAttemptTime: Date.now() + backoffDelay,
			backoffDelay
		};
	}

	function connect(playerId: string): Promise<void> {
		return new Promise((resolve, reject) => {
			const connectionTimeout = setTimeout(() => {
				if (socket) {
					socket.close();
				}
				state.connectionState = ConnectionState.ERROR;
				state.error = "Connection timeout - couldn't reach the server";
				reject(new Error(state.error));
			}, 5000);

			disconnect();
			state.connectionState = ConnectionState.CONNECTING;

			const wsUrl = PUBLIC_SPEKTRUM_WS_SERVER_URL;
			socket = new WebSocket(wsUrl);

			socket.onopen = () => {
				clearTimeout(connectionTimeout);
				info('WebSocket connected');
				state.connectionState = ConnectionState.CONNECTED;
				state.lastConnectedAt = Date.now();
				state.error = null;
				state.reconnectInfo.attempts = 0;
				state.reconnectInfo.nextAttemptTime = null;
				gameStore.setPlayerId(playerId);

				const connectMsg: ClientMessage = {
					type: 'Connect',
					player_id: playerId
				};

				send(connectMsg);
				resolve();
			};

			socket.onmessage = (event) => {
				try {
					const message = JSON.parse(event.data) as GameUpdate;
					info('Received message:', message);

					gameStore.processServerMessage(message);

					// All messages that arrived through websocket are valid websocket messages
					// Game error messages are handled by gameStore.processServerMessage
					state.error = null;
				} catch (e) {
					warn('Failed to parse message:', e);
					state.connectionState = ConnectionState.ERROR;
					state.error = 'Failed to parse server message';
					reject(new Error(state.error));
				}
			};

			socket.onclose = (event) => {
				clearTimeout(connectionTimeout);
				info('WebSocket closed:', event);
				state.lastDisconnectedAt = Date.now();

				const wasConnected = state.connectionState === ConnectionState.CONNECTED;

				// If it wasn't a normal closure and we were previously connected
				if (wasConnected && event.code !== 1000) {
					state.connectionState = ConnectionState.ERROR;
					state.error = 'Connection lost unexpectedly';
					attemptReconnect();
				} else if (event.code === 1000) {
					state.connectionState = ConnectionState.DISCONNECTED;
				}

				if (state.connectionState === ConnectionState.CONNECTING) {
					reject(new Error('Connection closed before it could be established'));
				}
			};

			socket.onerror = (error) => {
				clearTimeout(connectionTimeout);
				warn('WebSocket error:', error);
				state.connectionState = ConnectionState.ERROR;
				state.error = 'Failed to connect to game server';
				state.lastDisconnectedAt = Date.now();
				reject(new Error(state.error));
			};
		});
	}

	function attemptReconnect() {
		if (state.reconnectInfo.attempts >= MAX_RECONNECT_ATTEMPTS) {
			state.error = 'Failed to reconnect after multiple attempts';
			return;
		}

		const playerId = $derived(gameStore.state.playerId);
		if (!playerId) {
			info('No valid player in gameStore, skipping auto-reconnect');
			return;
		}

		state.connectionState = ConnectionState.RECONNECTING;
		updateReconnectMetadata(state.reconnectInfo.attempts + 1);

		clearReconnectTimeout();
		reconnectTimeout = window.setTimeout(() => {
			info(`Attempting reconnect #${state.reconnectInfo.attempts}...`);
			connect(playerId);
		}, state.reconnectInfo.backoffDelay);
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
		state.reconnectInfo.attempts = 0;
		state.reconnectInfo.nextAttemptTime = null;

		if (socket) {
			socket.close(1000, 'Client disconnecting');
			socket = null;
		}

		state.connectionState = ConnectionState.DISCONNECTED;
		state.messages = [];
		state.error = null;
		state.lastDisconnectedAt = Date.now();
	}

	function clearError() {
		state.error = null;
	}

	// Computed properties for the UI
	const timeUntilReconnect = $derived(() => {
		if (state.reconnectInfo.nextAttemptTime) {
			return Math.max(0, state.reconnectInfo.nextAttemptTime - Date.now());
		}
		return null;
	});

	const isReconnecting = $derived(state.connectionState === ConnectionState.RECONNECTING);

	const canReconnect = $derived(
		state.connectionState === ConnectionState.ERROR &&
			state.reconnectInfo.attempts < state.reconnectInfo.maxAttempts
	);

	return {
		state,
		connect,
		send,
		disconnect,
		clearError,
		// Computed properties for easy access
		timeUntilReconnect,
		isReconnecting,
		canReconnect
	};
}

export const websocketStore = createWebSocketStore();
