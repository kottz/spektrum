// src/lib/stores/websocket.svelte.ts
import { browser } from '$app/environment';
import type { GameUpdate, ClientMessage } from '../types/game';
import { ErrorCode } from '../types/game';
import { PUBLIC_SPEKTRUM_WS_SERVER_URL } from '$env/static/public';
import { gameStore } from '$lib/stores/game.svelte';
import { info, warn } from '$lib/utils/logger';

export enum ConnectionState {
	INITIAL = 'INITIAL',
	CONNECTING = 'CONNECTING',
	CONNECTED = 'CONNECTED',
	DISCONNECTED = 'DISCONNECTED',
	ERROR = 'ERROR',
	RECONNECTING = 'RECONNECTING',
	SUSPENDED = 'SUSPENDED',
	OFFLINE = 'OFFLINE'
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

type SessionStatus = 'valid' | 'invalid' | 'unknown';

type SupervisorEvent =
	| { type: 'CONNECT_REQUEST'; token: string }
	| { type: 'MANUAL_RECONNECT' }
	| { type: 'USER_LEAVE' }
	| { type: 'GAME_CLOSED'; reason?: string }
	| { type: 'SESSION_INVALID'; reason?: string }
	| { type: 'GAME_ENDED' }
	| { type: 'OPEN'; gen: number }
	| { type: 'CLOSE'; gen: number; code: number; reason: string }
	| { type: 'ERROR'; gen: number; reason: string }
	| { type: 'HEARTBEAT_TIMEOUT'; gen: number }
	| { type: 'LIVENESS_TIMEOUT'; gen: number }
	| { type: 'CONNECTION_TIMEOUT'; gen: number }
	| { type: 'RETRY_TIMER'; gen: number }
	| { type: 'VISIBILITY_HIDDEN' }
	| { type: 'VISIBILITY_VISIBLE' }
	| { type: 'OFFLINE' }
	| { type: 'ONLINE' }
	| { type: 'NETWORK_CHANGE' };

const SESSION_INVALID_MESSAGES = new Set<string>([
	ErrorCode.GameClosed,
	ErrorCode.LobbyNotFound,
	ErrorCode.PlayerNotFound,
	ErrorCode.NotAuthorized
]);

export function createWebSocketStore() {
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
		HEARTBEAT_TIMEOUT: 6000,
		LIVENESS_CHECK_INTERVAL: 5000,
		LIVENESS_TIMEOUT: 20000,
		OFFLINE_PROBE_INTERVAL: 10000
	} as const;

	// Private variables
	let socket: WebSocket | null = null;
	let reconnectTimeout: number | undefined = undefined;
	let heartbeatInterval: number | undefined = undefined;
	let heartbeatTimeout: number | undefined = undefined;
	let connectionTimeout: number | undefined = undefined;
	let livenessInterval: number | undefined = undefined;
	let offlineProbeTimeout: number | undefined = undefined;
	let generationId = 0;
	let desiredConnection = false;
	let sessionToken: string | null = null;
	let sessionStatus: SessionStatus = 'unknown';
	let wasIntentionalClose = false;
	let isVisible = browser ? document.visibilityState === 'visible' : true;
	let isOnline = browser ? navigator.onLine : true;
	let lastActivityAt: number | null = null;
	let pendingConnect: {
		resolve: () => void;
		reject: (error: Error) => void;
		gen: number | null;
	} | null = null;

	// Computed properties using runes
	const timeUntilReconnect = $derived(() =>
		state.nextAttemptTime ? Math.max(0, state.nextAttemptTime - Date.now()) : null
	);

	const isReconnecting = $derived(() => state.connectionState === ConnectionState.RECONNECTING);

	const canReconnect = $derived(
		() =>
			desiredConnection &&
			state.connectionState !== ConnectionState.ERROR &&
			state.reconnectAttempts < CONFIG.MAX_RECONNECT_ATTEMPTS
	);

	function calculateBackoffDelay(attempts: number): number {
		const baseDelay = Math.min(
			CONFIG.INITIAL_BACKOFF_DELAY * Math.pow(2, attempts),
			CONFIG.MAX_BACKOFF_DELAY
		);
		const jitter = baseDelay * (0.8 + Math.random() * 0.4);
		return Math.round(jitter);
	}

	function clearRetryTimer() {
		if (reconnectTimeout !== undefined) {
			window.clearTimeout(reconnectTimeout);
			reconnectTimeout = undefined;
		}
		state.nextAttemptTime = null;
	}

	function clearConnectionTimeout() {
		if (connectionTimeout !== undefined) {
			window.clearTimeout(connectionTimeout);
			connectionTimeout = undefined;
		}
	}

	function clearOfflineProbe() {
		if (offlineProbeTimeout !== undefined) {
			window.clearTimeout(offlineProbeTimeout);
			offlineProbeTimeout = undefined;
		}
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

	function startHeartbeat() {
		if (!browser) return;
		if (!socket || socket.readyState !== WebSocket.OPEN) return;
		if (!isVisible) return;
		stopHeartbeat();
		heartbeatInterval = window.setInterval(() => {
			if (!socket || socket.readyState !== WebSocket.OPEN) return;
			if (!isVisible) return;
			try {
				socket.send(new Uint8Array([0x42]));
			} catch (error) {
				warn('Heartbeat send failed', error);
				dispatch({ type: 'ERROR', gen: generationId, reason: 'Heartbeat send failed' });
				return;
			}
			if (heartbeatTimeout !== undefined) {
				window.clearTimeout(heartbeatTimeout);
			}
			heartbeatTimeout = window.setTimeout(() => {
				dispatch({ type: 'HEARTBEAT_TIMEOUT', gen: generationId });
			}, CONFIG.HEARTBEAT_TIMEOUT);
		}, CONFIG.HEARTBEAT_INTERVAL);
	}

	function startLivenessMonitor() {
		if (!browser) return;
		stopLivenessMonitor();
		livenessInterval = window.setInterval(() => {
			if (state.connectionState !== ConnectionState.CONNECTED) return;
			if (!desiredConnection || !isVisible) return;
			if (!lastActivityAt) return;
			if (Date.now() - lastActivityAt > CONFIG.LIVENESS_TIMEOUT) {
				dispatch({ type: 'LIVENESS_TIMEOUT', gen: generationId });
			}
		}, CONFIG.LIVENESS_CHECK_INTERVAL);
	}

	function stopLivenessMonitor() {
		if (livenessInterval !== undefined) {
			window.clearInterval(livenessInterval);
			livenessInterval = undefined;
		}
	}

	function resetAttempts() {
		state.reconnectAttempts = 0;
		state.nextAttemptTime = null;
	}

	function markConnected() {
		state.connectionState = ConnectionState.CONNECTED;
		state.lastConnectedAt = Date.now();
		state.error = null;
		isOnline = true;
		lastActivityAt = Date.now();
		resetAttempts();
		sessionStatus = 'valid';
		clearOfflineProbe();
	}

	function markDisconnected() {
		state.connectionState = ConnectionState.DISCONNECTED;
		state.lastDisconnectedAt = Date.now();
		state.error = null;
		clearRetryTimer();
		clearConnectionTimeout();
		clearOfflineProbe();
		stopHeartbeat();
		stopLivenessMonitor();
	}

	function markSuspended() {
		state.connectionState = ConnectionState.SUSPENDED;
		state.lastDisconnectedAt = Date.now();
		clearRetryTimer();
		stopHeartbeat();
		stopLivenessMonitor();
	}

	function markOffline() {
		state.connectionState = ConnectionState.OFFLINE;
		state.lastDisconnectedAt = Date.now();
		clearRetryTimer();
		stopHeartbeat();
		stopLivenessMonitor();
		scheduleOfflineProbe();
	}

	function markFailed(message: string) {
		state.connectionState = ConnectionState.ERROR;
		state.error = message;
		desiredConnection = false;
		sessionStatus = 'invalid';
		clearRetryTimer();
		clearConnectionTimeout();
		clearOfflineProbe();
		stopHeartbeat();
		stopLivenessMonitor();
		state.lastDisconnectedAt = Date.now();
	}

	function scheduleRetry() {
		if (reconnectTimeout !== undefined) return;
		if (state.reconnectAttempts >= CONFIG.MAX_RECONNECT_ATTEMPTS) {
			markFailed('Failed to reconnect after multiple attempts');
			return;
		}

		state.connectionState = ConnectionState.RECONNECTING;
		state.reconnectAttempts += 1;
		const backoffDelay = calculateBackoffDelay(state.reconnectAttempts - 1);
		state.nextAttemptTime = Date.now() + backoffDelay;
		info(
			`Scheduling reconnect attempt ${state.reconnectAttempts}/${CONFIG.MAX_RECONNECT_ATTEMPTS} in ${backoffDelay}ms`
		);
		reconnectTimeout = window.setTimeout(() => {
			reconnectTimeout = undefined;
			dispatch({ type: 'RETRY_TIMER', gen: generationId });
		}, backoffDelay);
	}

	function shouldReconnect(): boolean {
		return desiredConnection && sessionStatus !== 'invalid' && isVisible && isOnline;
	}

	function scheduleOfflineProbe() {
		if (!browser) return;
		if (!desiredConnection || !isVisible) return;
		if (offlineProbeTimeout !== undefined) return;
		offlineProbeTimeout = window.setTimeout(() => {
			offlineProbeTimeout = undefined;
			if (!desiredConnection || !isVisible) return;
			if (state.connectionState !== ConnectionState.OFFLINE) return;
			openSocket();
		}, CONFIG.OFFLINE_PROBE_INTERVAL);
	}

	function handleConnectionLoss(message?: string) {
		stopHeartbeat();
		stopLivenessMonitor();
		clearConnectionTimeout();
		state.lastDisconnectedAt = Date.now();
		if (message) {
			state.error = message;
		}

		if (!desiredConnection) {
			markDisconnected();
			return;
		}
		if (!isVisible) {
			markSuspended();
			return;
		}
		if (!isOnline) {
			markOffline();
			return;
		}
		scheduleRetry();
	}

	function openSocket() {
		if (!browser) return;
		if (!sessionToken) {
			warn('Session token is required to connect');
			return;
		}
		clearRetryTimer();
		clearConnectionTimeout();
		clearOfflineProbe();
		stopHeartbeat();
		stopLivenessMonitor();
		generationId += 1;
		const currentGen = generationId;
		if (socket) {
			const staleSocket = socket;
			socket = null;
			staleSocket.close(1000, 'Reconnecting');
		}
		wasIntentionalClose = false;
		state.connectionState = ConnectionState.CONNECTING;
		socket = new WebSocket(PUBLIC_SPEKTRUM_WS_SERVER_URL);
		connectionTimeout = window.setTimeout(() => {
			dispatch({ type: 'CONNECTION_TIMEOUT', gen: currentGen });
		}, CONFIG.CONNECTION_TIMEOUT);
		socket.onopen = () => dispatch({ type: 'OPEN', gen: currentGen });
		socket.onmessage = (event) => handleMessage(event, currentGen);
		socket.onclose = (event) =>
			dispatch({ type: 'CLOSE', gen: currentGen, code: event.code, reason: event.reason });
		socket.onerror = () =>
			dispatch({ type: 'ERROR', gen: currentGen, reason: 'Connection error occurred' });
		if (pendingConnect) {
			pendingConnect.gen = currentGen;
		}
	}

	function closeSocket(intentional: boolean) {
		clearConnectionTimeout();
		clearRetryTimer();
		stopHeartbeat();
		if (socket) {
			wasIntentionalClose = intentional;
			socket.close(1000, intentional ? 'Client disconnecting' : 'Connection closing');
			socket = null;
		}
	}

	function resolvePendingConnect(gen: number) {
		if (!pendingConnect) return;
		if (pendingConnect.gen !== gen) return;
		pendingConnect.resolve();
		pendingConnect = null;
	}

	function rejectPendingConnect(gen: number, message: string) {
		if (!pendingConnect) return;
		if (pendingConnect.gen !== gen) return;
		pendingConnect.reject(new Error(message));
		pendingConnect = null;
	}

	function isSessionInvalidMessage(message: string): boolean {
		if (!message) return false;
		for (const code of SESSION_INVALID_MESSAGES) {
			if (message.includes(code)) return true;
		}
		return false;
	}

	function handleMessage(event: MessageEvent, gen: number) {
		if (gen !== generationId) return;
		lastActivityAt = Date.now();
		if (typeof event.data !== 'string') {
			if (heartbeatTimeout !== undefined) {
				window.clearTimeout(heartbeatTimeout);
				heartbeatTimeout = undefined;
			}
			return;
		}
		if (heartbeatTimeout !== undefined) {
			window.clearTimeout(heartbeatTimeout);
			heartbeatTimeout = undefined;
		}
		try {
			const message = JSON.parse(event.data) as GameUpdate;

			if (message.type === 'GameClosed') {
				dispatch({ type: 'GAME_CLOSED', reason: message.reason });
			} else if (message.type === 'PlayerKicked') {
				dispatch({ type: 'SESSION_INVALID', reason: message.reason });
			} else if (message.type === 'Error' && isSessionInvalidMessage(message.message)) {
				dispatch({ type: 'SESSION_INVALID', reason: message.message });
			} else if (message.type === 'GameOver') {
				dispatch({ type: 'GAME_ENDED' });
			}

			gameStore.processServerMessage(message);
			state.error = null;
		} catch (error) {
			warn('Failed to parse message:', error);
			state.error = 'Invalid message received';
			state.connectionState = ConnectionState.ERROR;
		}
	}

	function dispatch(event: SupervisorEvent) {
		if ('gen' in event && event.gen !== generationId) {
			return;
		}

		switch (event.type) {
			case 'CONNECT_REQUEST': {
				const previousToken = sessionToken;
				sessionToken = event.token;
				desiredConnection = true;
				sessionStatus = 'unknown';
				state.error = null;
				if (!isVisible) {
					markSuspended();
					if (pendingConnect) {
						pendingConnect.reject(new Error('Page not visible'));
						pendingConnect = null;
					}
					return;
				}
				if (socket?.readyState === WebSocket.OPEN) {
					if (previousToken && previousToken !== sessionToken) {
						openSocket();
						return;
					}
					markConnected();
					startHeartbeat();
					if (pendingConnect) {
						pendingConnect.resolve();
						pendingConnect = null;
					}
					return;
				}
				openSocket();
				return;
			}
			case 'MANUAL_RECONNECT': {
				desiredConnection = true;
				resetAttempts();
				state.error = null;
				if (!sessionToken) {
					warn('No session token available for manual reconnect');
					markFailed('No session available for reconnect');
					return;
				}
				if (!isVisible) {
					markSuspended();
					return;
				}
				openSocket();
				return;
			}
			case 'USER_LEAVE': {
				desiredConnection = false;
				sessionToken = null;
				sessionStatus = 'unknown';
				closeSocket(true);
				resetAttempts();
				markDisconnected();
				return;
			}
			case 'GAME_CLOSED': {
				desiredConnection = false;
				sessionToken = null;
				sessionStatus = 'invalid';
				closeSocket(true);
				resetAttempts();
				markDisconnected();
				return;
			}
			case 'SESSION_INVALID': {
				sessionStatus = 'invalid';
				desiredConnection = false;
				sessionToken = null;
				state.error = null;
				resetAttempts();
				closeSocket(true);
				markDisconnected();
				return;
			}
			case 'GAME_ENDED': {
				// Keep connection alive for possible restarts.
				return;
			}
			case 'OPEN': {
				clearConnectionTimeout();
				markConnected();
				if (sessionToken) {
					gameStore.setSessionToken(sessionToken);
					send({ type: 'Connect', session_token: sessionToken });
				}
				if (isVisible) {
					startHeartbeat();
					startLivenessMonitor();
				}
				resolvePendingConnect(event.gen);
				return;
			}
			case 'CLOSE': {
				state.lastDisconnectedAt = Date.now();
				if (state.connectionState === ConnectionState.ERROR) {
					return;
				}
				if (wasIntentionalClose) {
					markDisconnected();
					return;
				}
				handleConnectionLoss('Connection closed');
				rejectPendingConnect(event.gen, 'Connection closed');
				return;
			}
			case 'ERROR': {
				state.error = event.reason;
				handleConnectionLoss(event.reason);
				rejectPendingConnect(event.gen, event.reason);
				return;
			}
			case 'HEARTBEAT_TIMEOUT': {
				warn('Heartbeat timeout');
				handleConnectionLoss('Connection lost - heartbeat timeout');
				return;
			}
			case 'LIVENESS_TIMEOUT': {
				warn('Connection stalled');
				handleConnectionLoss('Connection stalled');
				return;
			}
			case 'CONNECTION_TIMEOUT': {
				state.error = 'Connection timeout';
				handleConnectionLoss('Connection timeout');
				rejectPendingConnect(event.gen, 'Connection timeout');
				return;
			}
			case 'RETRY_TIMER': {
				if (!shouldReconnect()) {
					if (!isVisible) {
						markSuspended();
						return;
					}
					if (!isOnline) {
						markOffline();
						return;
					}
					markDisconnected();
					return;
				}
				openSocket();
				return;
			}
			case 'VISIBILITY_HIDDEN': {
				isVisible = false;
				if (state.connectionState === ConnectionState.CONNECTED) {
					stopHeartbeat();
					clearRetryTimer();
					stopLivenessMonitor();
					return;
				}
				if (state.connectionState === ConnectionState.RECONNECTING) {
					markSuspended();
					return;
				}
				return;
			}
			case 'VISIBILITY_VISIBLE': {
				isVisible = true;
				if (socket?.readyState === WebSocket.OPEN) {
					markConnected();
					startHeartbeat();
					startLivenessMonitor();
					return;
				}
				if (!desiredConnection) {
					markDisconnected();
					return;
				}
				if (!isOnline) {
					markOffline();
					return;
				}
				resetAttempts();
				openSocket();
				return;
			}
			case 'OFFLINE': {
				isOnline = false;
				state.error = 'You appear to be offline';
				markOffline();
				closeSocket(false);
				return;
			}
			case 'ONLINE': {
				isOnline = true;
				state.error = null;
				if (!desiredConnection) {
					markDisconnected();
					return;
				}
				if (!isVisible) {
					markSuspended();
					return;
				}
				resetAttempts();
				openSocket();
				return;
			}
			case 'NETWORK_CHANGE': {
				const currentOnline = navigator.onLine;
				if (!currentOnline) {
					dispatch({ type: 'OFFLINE' });
					return;
				}
				isOnline = true;
				state.error = null;
				if (!desiredConnection) {
					return;
				}
				if (!isVisible) {
					markSuspended();
					return;
				}
				resetAttempts();
				openSocket();
				return;
			}
		}
	}

	function connect(token: string): Promise<void> {
		if (!token) {
			return Promise.reject(new Error('Session token is required'));
		}
		if (pendingConnect) {
			pendingConnect.reject(new Error('Connection superseded by a new request'));
			pendingConnect = null;
		}
		return new Promise((resolve, reject) => {
			pendingConnect = { resolve, reject, gen: null };
			dispatch({ type: 'CONNECT_REQUEST', token });
		});
	}

	function disconnect() {
		dispatch({ type: 'USER_LEAVE' });
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

	function clearError() {
		state.error = null;
	}

	function setupEnvironmentListeners() {
		if (!browser) return;
		document.addEventListener('visibilitychange', () => {
			dispatch({
				type: document.visibilityState === 'visible' ? 'VISIBILITY_VISIBLE' : 'VISIBILITY_HIDDEN'
			});
		});
		window.addEventListener('online', () => dispatch({ type: 'ONLINE' }));
		window.addEventListener('offline', () => dispatch({ type: 'OFFLINE' }));
		const connection =
			(navigator as Navigator & { connection?: EventTarget }).connection ||
			(navigator as Navigator & { mozConnection?: EventTarget }).mozConnection ||
			(navigator as Navigator & { webkitConnection?: EventTarget }).webkitConnection;
		if (connection && 'addEventListener' in connection) {
			connection.addEventListener('change', () => dispatch({ type: 'NETWORK_CHANGE' }));
		}
	}

	setupEnvironmentListeners();

	return {
		state,
		connect,
		disconnect,
		send,
		clearError,
		get timeUntilReconnect() {
			return timeUntilReconnect;
		},
		get isReconnecting() {
			return isReconnecting;
		},
		get canReconnect() {
			return canReconnect;
		},
		get maxReconnectAttempts() {
			return CONFIG.MAX_RECONNECT_ATTEMPTS;
		},
		manualReconnect: () => dispatch({ type: 'MANUAL_RECONNECT' })
	};
}

export const websocketStore = createWebSocketStore();
