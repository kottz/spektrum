// src/lib/stores/websocket.ts
import { writable, get } from 'svelte/store';
import type { ServerMessage, ClientMessage } from '../types/game';
import { PUBLIC_SPEKTRUM_WS_SERVER_URL } from '$env/static/public';

// Store state interface
interface WebSocketState {
	connected: boolean;
	messages: ServerMessage[];
	error: string | null;
	isConnecting: boolean;
}

function createWebSocketStore() {
	// Initialize the store with default state
	const { subscribe, set, update } = writable<WebSocketState>({
		connected: false,
		messages: [],
		error: null,
		isConnecting: false
	});

	// WebSocket instance
	let socket: WebSocket | null = null;
	// Reconnection timeout handle
	let reconnectTimeout: number | null = null;
	// Number of reconnection attempts
	let reconnectAttempts = 0;
	// Maximum number of reconnection attempts
	const MAX_RECONNECT_ATTEMPTS = 3;

	function clearReconnectTimeout() {
		if (reconnectTimeout) {
			window.clearTimeout(reconnectTimeout);
			reconnectTimeout = null;
		}
	}

	const connect = (joinCode: string, playerName: string, adminId?: string) => {
		// Clean up any existing connection first
		disconnect();

		update(state => ({ ...state, isConnecting: true }));

		const wsUrl = PUBLIC_SPEKTRUM_WS_SERVER_URL;
		socket = new WebSocket(wsUrl);

		socket.onopen = () => {
			console.log('WebSocket connected');
			reconnectAttempts = 0;
			update(state => ({
				...state,
				connected: true,
				error: null,
				isConnecting: false
			}));

			// Send join message once connected
			const joinMsg: ClientMessage = {
				type: 'JoinLobby',
				join_code: joinCode,
				name: playerName,
				admin_id: adminId
			};
			send(joinMsg);
		};

		socket.onmessage = (event) => {
			try {
				const message = JSON.parse(event.data) as ServerMessage;
				console.log('Received message:', message);

				update(state => ({
					...state,
					messages: [...state.messages, message]
				}));

				// Clear error state on successful message
				if (message.type !== 'Error') {
					update(state => ({ ...state, error: null }));
				}
			} catch (e) {
				console.error('Failed to parse message:', e);
				update(state => ({
					...state,
					error: 'Failed to parse server message'
				}));
			}
		};

		socket.onclose = (event) => {
			console.log('WebSocket closed:', event);

			const wasConnected = get({ subscribe }).connected;
			update(state => ({
				...state,
				connected: false,
				isConnecting: false
			}));

			// Only attempt reconnect if we were previously connected
			// and it wasn't a normal closure
			if (wasConnected && event.code !== 1000) {
				attemptReconnect(joinCode, playerName, adminId);
			}
		};

		socket.onerror = (error) => {
			console.error('WebSocket error:', error);
			update(state => ({
				...state,
				error: 'WebSocket connection error',
				isConnecting: false
			}));
		};
	};

	const attemptReconnect = (joinCode: string, playerName: string, adminId?: string) => {
		if (reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
			update(state => ({
				...state,
				error: 'Failed to reconnect after multiple attempts'
			}));
			return;
		}

		reconnectAttempts++;
		const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), 10000);

		clearReconnectTimeout();
		reconnectTimeout = window.setTimeout(() => {
			console.log(`Attempting reconnect ${reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS}`);
			connect(joinCode, playerName, adminId);
		}, delay);
	};

	const send = (message: ClientMessage) => {
		if (socket?.readyState === WebSocket.OPEN) {
			console.log('Sending message:', message);
			socket.send(JSON.stringify(message));
		} else {
			console.error('Cannot send message: connection not available');
			update(state => ({
				...state,
				error: 'Connection not available'
			}));
		}
	};

	const disconnect = () => {
		clearReconnectTimeout();
		reconnectAttempts = 0;

		if (socket) {
			// Close the connection cleanly
			socket.close(1000, 'Client disconnecting');
			socket = null;
		}

		// Reset the store state
		set({
			connected: false,
			messages: [],
			error: null,
			isConnecting: false
		});
	};

	return {
		subscribe,
		connect,
		send,
		disconnect,
		clearError: () => update(state => ({ ...state, error: null }))
	};
}

export const websocketStore = createWebSocketStore();
