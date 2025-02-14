// src/lib/stores/websocket.ts
import type { GameUpdate, ClientMessage } from '../types/game';
import { PUBLIC_SPEKTRUM_WS_SERVER_URL } from '$env/static/public';
import { gameStore } from '$lib/stores/game.svelte';
import { info, warn } from '$lib/utils/logger';

export enum ConnectionState {
	INITIAL = 'INITIAL',
	CONNECTING = 'CONNECTING',
	CONNECTED = 'CONNECTED',
	DISCONNECTED = 'DISCONNECTED',
	ERROR = 'ERROR',
	RECONNECTING = 'RECONNECTING'
}

interface WebSocketState {
	connectionState: ConnectionState;
	messages: GameUpdate[];
	error: string | null;
	lastConnectedAt: number | null;
	lastDisconnectedAt: number | null;
	reconnectAttempts: number;
	nextAttemptTime: number | null;
}

function createWebSocketStore() {
	// Core state with runes
	const state = $state<WebSocketState>({
		connectionState: ConnectionState.INITIAL,
		messages: [],
		error: null,
		lastConnectedAt: null,
		lastDisconnectedAt: null,
		reconnectAttempts: 0,
		nextAttemptTime: null
	});

	// Configuration constants
	const CONFIG = {
		MAX_RECONNECT_ATTEMPTS: 5,
		INITIAL_BACKOFF_DELAY: 1000,
		MAX_BACKOFF_DELAY: 10000,
		CONNECTION_TIMEOUT: 5000,
		HEARTBEAT_INTERVAL: 5000,
		HEARTBEAT_TIMEOUT: 6000
	} as const;

	// Private variables
	let socket: WebSocket | null = null;
	let reconnectTimeout: number | undefined = undefined;
	let heartbeatInterval: number | undefined = undefined;
	let heartbeatTimeout: number | undefined = undefined;
	let connectionTimeout: number | undefined = undefined;

	// Computed properties using runes
	const timeUntilReconnect = $derived(() =>
		state.nextAttemptTime ? Math.max(0, state.nextAttemptTime - Date.now()) : null
	);

	const isReconnecting = $derived(() => state.connectionState === ConnectionState.RECONNECTING);

	const canReconnect = $derived(
		() =>
			state.connectionState === ConnectionState.ERROR &&
			state.reconnectAttempts < CONFIG.MAX_RECONNECT_ATTEMPTS
	);

	function calculateBackoffDelay(attempts: number): number {
		return Math.min(CONFIG.INITIAL_BACKOFF_DELAY * Math.pow(2, attempts), CONFIG.MAX_BACKOFF_DELAY);
	}

	function clearTimeouts() {
		// Clear all possible timeouts and intervals
		[reconnectTimeout, heartbeatInterval, heartbeatTimeout, connectionTimeout].forEach(
			(timeout) => {
				if (timeout !== undefined) {
					if (typeof timeout === 'number') {
						// setTimeout returns a number in browser
						window.clearTimeout(timeout);
					}
				}
			}
		);
		reconnectTimeout = undefined;
		heartbeatInterval = undefined;
		heartbeatTimeout = undefined;
		connectionTimeout = undefined;
		state.nextAttemptTime = null;
	}

	function attemptReconnect() {
		// Early return if we're already in a reconnection cycle
		if (state.connectionState === ConnectionState.RECONNECTING || reconnectTimeout !== undefined) {
			info('Already in reconnection cycle, skipping new attempt');
			return;
		}

		const playerId = $derived(gameStore.state.playerId);
		if (!playerId) {
			warn('No player ID available for reconnection');
			return;
		}

		state.connectionState = ConnectionState.RECONNECTING;
		state.reconnectAttempts++;

		if (state.reconnectAttempts > CONFIG.MAX_RECONNECT_ATTEMPTS) {
			state.error = 'Failed to reconnect after multiple attempts';
			state.connectionState = ConnectionState.ERROR;
			return;
		}

		const backoffDelay = calculateBackoffDelay(state.reconnectAttempts - 1);
		state.nextAttemptTime = Date.now() + backoffDelay;

		info(
			`Scheduling reconnect attempt ${state.reconnectAttempts}/${CONFIG.MAX_RECONNECT_ATTEMPTS} in ${backoffDelay}ms`
		);

		reconnectTimeout = window.setTimeout(() => {
			reconnectTimeout = undefined; // Clear the timeout reference once it fires
			info(
				`Executing reconnect attempt ${state.reconnectAttempts}/${CONFIG.MAX_RECONNECT_ATTEMPTS}`
			);
			connect(playerId).catch(() => {
				if (state.reconnectAttempts >= CONFIG.MAX_RECONNECT_ATTEMPTS) {
					state.error = 'Failed to reconnect after multiple attempts';
					state.connectionState = ConnectionState.ERROR;
				}
			});
		}, backoffDelay);
	}

	function startHeartbeat() {
		stopHeartbeat();
		heartbeatInterval = window.setInterval(() => {
			if (socket?.readyState === WebSocket.OPEN) {
				socket.send(new Uint8Array([0x42]));
				heartbeatTimeout = window.setTimeout(() => {
					// If already reconnecting, don't update error message but still trigger reconnect.
					if (state.connectionState === ConnectionState.RECONNECTING) {
						attemptReconnect();
						return;
					}
					warn('Heartbeat timeout');
					state.error = 'Connection lost - heartbeat timeout';
					state.connectionState = ConnectionState.ERROR;
					attemptReconnect();
				}, CONFIG.HEARTBEAT_TIMEOUT);
			}
		}, CONFIG.HEARTBEAT_INTERVAL);
	}

	function stopHeartbeat() {
		if (heartbeatInterval !== undefined) {
			window.clearInterval(heartbeatInterval);
			heartbeatInterval = undefined;
		}
		if (heartbeatTimeout !== undefined) {
			window.clearTimeout(heartbeatTimeout);
			heartbeatTimeout = undefined;
		}
	}

	function disconnect() {
		clearTimeouts();
		if (socket) {
			socket.close(1000, 'Client disconnecting');
			socket = null;
		}
		state.connectionState = ConnectionState.DISCONNECTED;
		state.lastDisconnectedAt = Date.now();
		state.reconnectAttempts = 0;
		state.nextAttemptTime = null;
	}

	async function connect(playerId: string): Promise<void> {
		if (!playerId) {
			throw new Error('Player ID is required');
		}

		return new Promise((resolve, reject) => {
			state.connectionState = ConnectionState.CONNECTING;
			socket = new WebSocket(PUBLIC_SPEKTRUM_WS_SERVER_URL);

			// Only set connection timeout if we're not in a reconnection cycle
			if (state.reconnectAttempts === 0) {
				connectionTimeout = window.setTimeout(() => {
					state.error = 'Connection timeout';
					state.connectionState = ConnectionState.ERROR;
					if (canReconnect()) {
						attemptReconnect();
					}
					reject(new Error(state.error));
				}, CONFIG.CONNECTION_TIMEOUT);
			}

			socket.onopen = () => {
				if (connectionTimeout) {
					clearTimeout(connectionTimeout);
					connectionTimeout = undefined;
				}
				state.connectionState = ConnectionState.CONNECTED;
				state.lastConnectedAt = Date.now();
				state.error = null;
				state.reconnectAttempts = 0;

				gameStore.setPlayerId(playerId);
				send({ type: 'Connect', player_id: playerId });
				startHeartbeat();
				resolve();
			};

			socket.onmessage = handleMessage;
			socket.onclose = handleClose;
			socket.onerror = handleError;
		});
	}

	function handleMessage(event: MessageEvent) {
		if (typeof event.data !== 'string') {
			if (heartbeatTimeout !== undefined) {
				clearTimeout(heartbeatTimeout);
				heartbeatTimeout = undefined;
			}
			return;
		}
		try {
			const message = JSON.parse(event.data) as GameUpdate;
			gameStore.processServerMessage(message);
			state.error = null;
		} catch (e) {
			warn('Failed to parse message:', e);
			state.error = 'Invalid message received';
			state.connectionState = ConnectionState.ERROR;
		}
	}

	function handleClose(event: CloseEvent) {
		state.lastDisconnectedAt = Date.now();

		if (state.connectionState === ConnectionState.CONNECTED && event.code !== 1000) {
			state.connectionState = ConnectionState.ERROR;
			state.error = 'Connection lost unexpectedly';
			attemptReconnect();
		} else if (event.code === 1000) {
			state.connectionState = ConnectionState.DISCONNECTED;
		}
	}

	function handleError(event: Event) {
		warn('WebSocket error:', event);
		state.connectionState = ConnectionState.ERROR;
		state.error = 'Connection error occurred';
		state.lastDisconnectedAt = Date.now();

		if (canReconnect()) {
			attemptReconnect();
		}
	}

	function send(message: ClientMessage) {
		if (socket?.readyState !== WebSocket.OPEN) {
			warn('Cannot send message: connection not open');
			state.error = 'Connection not available';
			return;
		}

		try {
			socket.send(JSON.stringify(message));
			info('Message sent:', message);
		} catch (error) {
			warn('Failed to send message:', error);
			state.error = 'Failed to send message';
		}
	}

	return {
		state,
		connect,
		disconnect,
		send,
		clearError: () => (state.error = null),
		timeUntilReconnect,
		isReconnecting,
		canReconnect
	};
}

export const websocketStore = createWebSocketStore();
