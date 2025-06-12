import { info, warn } from '$lib/utils/logger';
import type { StreamEvent, BasePublicGameState } from '$lib/types/stream.types';

const BROADCAST_CHANNEL_NAME = 'spektrum-stream';

interface StateUpdateMessage {
	type: 'STATE_UPDATE';
	gameType: string;
	gameState: BasePublicGameState | Record<string, unknown>; // Allow extension
}

interface StreamEventMessage {
	type: 'STREAM_EVENT';
	gameType: string;
	event: StreamEvent;
}

interface StreamControlMessage {
	type: 'STREAM_CONTROL';
	action: 'show' | 'hide';
}

interface ServerMessageMessage {
	type: 'SERVER_MESSAGE';
	gameType: string;
	message: Record<string, unknown>;
}

interface InitialStateMessage {
	type: 'INITIAL_STATE';
	gameType: string;
	joinCode: string;
	gameState: BasePublicGameState | Record<string, unknown>;
}

interface StreamReadyMessage {
	type: 'STREAM_READY';
}

interface StreamCloseMessage {
	type: 'STREAM_CLOSE';
}

interface StreamDisconnectedMessage {
	type: 'STREAM_DISCONNECTED';
}

type BroadcastMessage =
	| StateUpdateMessage
	| StreamEventMessage
	| StreamControlMessage
	| ServerMessageMessage
	| InitialStateMessage
	| StreamReadyMessage
	| StreamCloseMessage
	| StreamDisconnectedMessage;

class BroadcastService {
	private channel: BroadcastChannel | null = null;
	private listeners: Set<(message: BroadcastMessage) => void> = new Set();
	private isInitialized = false;
	private isStreamWindow = false;
	private hasActiveStreams = false;

	initialize(isStreamWindow: boolean = false): void {
		if (this.isInitialized) {
			info('BroadcastService: Already initialized');
			return;
		}

		// Check if we're in a browser environment
		if (typeof BroadcastChannel === 'undefined') {
			warn('BroadcastService: BroadcastChannel not available (likely SSR environment)');
			return;
		}

		try {
			this.channel = new BroadcastChannel(BROADCAST_CHANNEL_NAME);
			this.isStreamWindow = isStreamWindow;
			this.isInitialized = true;

			this.channel.addEventListener('message', (event) => {
				try {
					const message = event.data as BroadcastMessage;
					this.notifyListeners(message);
				} catch (error) {
					warn('BroadcastService: Error processing message', error);
				}
			});

			info(`BroadcastService: Initialized as ${isStreamWindow ? 'stream window' : 'admin window'}`);
		} catch (error) {
			warn('BroadcastService: Failed to initialize BroadcastChannel', error);
		}
	}

	cleanup(): void {
		if (this.channel) {
			this.channel.close();
			this.channel = null;
		}
		this.listeners.clear();
		this.isInitialized = false;
		info('BroadcastService: Cleaned up');
	}

	addListener(listener: (message: BroadcastMessage) => void): void {
		this.listeners.add(listener);
	}

	removeListener(listener: (message: BroadcastMessage) => void): void {
		this.listeners.delete(listener);
	}

	private notifyListeners(message: BroadcastMessage): void {
		this.listeners.forEach((listener) => {
			try {
				listener(message);
			} catch (error) {
				warn('BroadcastService: Error in listener', error);
			}
		});
	}

	broadcastStateUpdate(
		gameType: string,
		gameState: BasePublicGameState | Record<string, unknown>
	): void {
		if (!this.isInitialized || !this.channel || this.isStreamWindow) {
			return; // Only admin windows should broadcast
		}

		const message: StateUpdateMessage = {
			type: 'STATE_UPDATE',
			gameType,
			gameState
		};

		try {
			// Deep clone and serialize to ensure data is safe for broadcast
			const serializedMessage = JSON.parse(JSON.stringify(message));
			this.channel.postMessage(serializedMessage);
			info('BroadcastService: State update sent', { gameType });
		} catch (error) {
			warn('BroadcastService: Failed to broadcast state update', error);
		}
	}

	broadcastStreamEvent(gameType: string, event: StreamEvent): void {
		if (!this.isInitialized || !this.channel || this.isStreamWindow) {
			return; // Only admin windows should broadcast
		}

		const message: StreamEventMessage = {
			type: 'STREAM_EVENT',
			gameType,
			event
		};

		try {
			// Deep clone and serialize to ensure data is safe for broadcast
			const serializedMessage = JSON.parse(JSON.stringify(message));
			this.channel.postMessage(serializedMessage);
			info('BroadcastService: Stream event sent', { gameType, eventType: event.type });
		} catch (error) {
			warn('BroadcastService: Failed to broadcast stream event', error);
		}
	}

	broadcastStreamControl(action: 'show' | 'hide'): void {
		if (!this.isInitialized || !this.channel) {
			return;
		}

		const message: StreamControlMessage = {
			type: 'STREAM_CONTROL',
			action
		};

		try {
			// Deep clone and serialize to ensure data is safe for broadcast
			const serializedMessage = JSON.parse(JSON.stringify(message));
			this.channel.postMessage(serializedMessage);
			info('BroadcastService: Stream control sent', { action });
		} catch (error) {
			warn('BroadcastService: Failed to broadcast stream control', error);
		}
	}

	broadcastServerMessage(gameType: string, message: Record<string, unknown>): void {
		if (!this.isInitialized || !this.channel || this.isStreamWindow || !this.hasActiveStreams) {
			return; // Only admin windows should broadcast, and only when streams are active
		}

		const broadcastMessage: ServerMessageMessage = {
			type: 'SERVER_MESSAGE',
			gameType,
			message
		};

		try {
			// Deep clone and serialize to ensure data is safe for broadcast
			const serializedMessage = JSON.parse(JSON.stringify(broadcastMessage));
			this.channel.postMessage(serializedMessage);
			info('BroadcastService: Server message relayed', { gameType, messageType: message.type });
		} catch (error) {
			warn('BroadcastService: Failed to broadcast server message', error);
		}
	}

	broadcastInitialState(
		gameType: string,
		joinCode: string,
		gameState: BasePublicGameState | Record<string, unknown>
	): void {
		if (!this.isInitialized || !this.channel || this.isStreamWindow) {
			return; // Only admin windows should broadcast
		}

		const message: InitialStateMessage = {
			type: 'INITIAL_STATE',
			gameType,
			joinCode,
			gameState
		};

		try {
			// Deep clone and serialize to ensure data is safe for broadcast
			const serializedMessage = JSON.parse(JSON.stringify(message));
			this.channel.postMessage(serializedMessage);
			info('BroadcastService: Initial state sent', { gameType, joinCode });
		} catch (error) {
			warn('BroadcastService: Failed to broadcast initial state', error);
		}
	}

	broadcastStreamReady(): void {
		if (!this.isInitialized || !this.channel || !this.isStreamWindow) {
			return; // Only stream windows should broadcast ready signal
		}

		const message: StreamReadyMessage = {
			type: 'STREAM_READY'
		};

		try {
			this.channel.postMessage(message);
			info('BroadcastService: Stream ready signal sent');
		} catch (error) {
			warn('BroadcastService: Failed to broadcast stream ready', error);
		}
	}

	broadcastStreamClose(): void {
		if (!this.isInitialized || !this.channel || this.isStreamWindow) {
			return; // Only admin windows should broadcast close signal
		}

		const message: StreamCloseMessage = {
			type: 'STREAM_CLOSE'
		};

		try {
			this.channel.postMessage(message);
			info('BroadcastService: Stream close signal sent');
		} catch (error) {
			warn('BroadcastService: Failed to broadcast stream close', error);
		}
	}

	broadcastStreamDisconnected(): void {
		if (!this.isInitialized || !this.channel || !this.isStreamWindow) {
			return; // Only stream windows should broadcast disconnect signal
		}

		const message: StreamDisconnectedMessage = {
			type: 'STREAM_DISCONNECTED'
		};

		try {
			this.channel.postMessage(message);
			info('BroadcastService: Stream disconnected signal sent');
		} catch (error) {
			warn('BroadcastService: Failed to broadcast stream disconnected', error);
		}
	}

	getIsInitialized(): boolean {
		return this.isInitialized;
	}

	getIsStreamWindow(): boolean {
		return this.isStreamWindow;
	}

	getHasActiveStreams(): boolean {
		return this.hasActiveStreams;
	}

	setHasActiveStreams(hasStreams: boolean): void {
		this.hasActiveStreams = hasStreams;
		info('BroadcastService: Active streams status updated', { hasActiveStreams: hasStreams });
	}
}

export const broadcastService = new BroadcastService();
export type {
	BroadcastMessage,
	StateUpdateMessage,
	StreamEventMessage,
	StreamControlMessage,
	ServerMessageMessage,
	InitialStateMessage,
	StreamReadyMessage,
	StreamCloseMessage,
	StreamDisconnectedMessage
};
