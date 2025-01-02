// src/lib/stores/websocket.ts

import { writable, get } from 'svelte/store';
import type { ServerMessage, ClientMessage } from '../types/game';
import { PUBLIC_SPEKTRUM_WS_SERVER_URL } from '$env/static/public';

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
	let reconnectTimeout: number | null = null;
	let reconnectAttempts = 0;
	const MAX_RECONNECT_ATTEMPTS = 3;

	function clearReconnectTimeout() {
		if (reconnectTimeout) {
			window.clearTimeout(reconnectTimeout);
			reconnectTimeout = null;
		}
	}

	/**
	 * Tries reconnecting via a 'Reconnect' message
	 * if we have credentials stored in localStorage.
	 * Returns true if a reconnect message was sent,
	 * or false if no credentials were found.
	 */
	function tryReconnect(): boolean {
		const lobbyId = localStorage.getItem('lobbyId');
		const playerId = localStorage.getItem('playerId');
		if (lobbyId && playerId) {
			console.log('Attempting to reconnect with stored credentials...');
			const reconnectMsg: ClientMessage = {
				type: 'Reconnect',
				lobby_id: lobbyId,
				player_id: playerId
			};
			send(reconnectMsg);
			return true;
		}
		return false;
	}

	/**
	 * Connect to the backend WebSocket server.
	 * If joinCode/playerName are provided, attempt a normal 'JoinLobby'.
	 * Otherwise, try to 'Reconnect' if we have stored credentials.
	 */
	function connect(joinCode?: string, playerName?: string, adminId?: string) {
		disconnect(); // Clean up any existing connection

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

			// Attempt reconnect first if credentials exist
			const didReconnect = tryReconnect();
			if (!didReconnect && joinCode && playerName) {
				// No stored credentials or forced new join
				const joinMsg: ClientMessage = {
					type: 'JoinLobby',
					join_code: joinCode,
					name: playerName,
					admin_id: adminId
				};
				send(joinMsg);
			}
		};

		socket.onmessage = event => {
			try {
				const message = JSON.parse(event.data) as ServerMessage;
				console.log('Received message:', message);

				update(state => ({
					...state,
					messages: [...state.messages, message]
				}));

				// Clear any previous error if we got a non-error message
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

		socket.onclose = event => {
			console.log('WebSocket closed:', event);

			const wasConnected = get({ subscribe }).connected;
			update(state => ({
				...state,
				connected: false,
				isConnecting: false
			}));

			// Attempt reconnection if the socket was previously open
			// and the closure wasn’t clean (code != 1000)
			if (wasConnected && event.code !== 1000) {
				attemptReconnect(joinCode, playerName, adminId);
			}
		};

		socket.onerror = error => {
			console.error('WebSocket error:', error);
			update(state => ({
				...state,
				error: 'WebSocket connection error',
				isConnecting: false
			}));
		};
	}

	/**
	 * If reconnectAttempts < MAX, increment and retry connecting.
	 */
	function attemptReconnect(joinCode?: string, playerName?: string, adminId?: string) {
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
			console.log(`Attempting reconnect ${reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS}...`);
			connect(joinCode, playerName, adminId);
		}, delay);
	}

	/**
	 * Send a message if the socket is open.
	 */
	function send(message: ClientMessage) {
		if (socket?.readyState === WebSocket.OPEN) {
			console.log('Sending message:', message);
			socket.send(JSON.stringify(message));
		} else {
			console.error('Cannot send message: connection not open');
			update(state => ({
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
			socket.close(1000, 'Client disconnecting'); // normal closure
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
		clearError: () => update(state => ({ ...state, error: null }))
	};
}

export const websocketStore = createWebSocketStore();
