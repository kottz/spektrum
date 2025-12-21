// @vitest-environment jsdom
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$app/environment', () => ({ browser: true }));
vi.mock('$env/static/public', () => ({ PUBLIC_SPEKTRUM_WS_SERVER_URL: 'ws://test' }));
vi.mock('$lib/utils/logger', () => ({ info: () => {}, warn: () => {} }));

class MockWebSocket {
	static CONNECTING = 0;
	static OPEN = 1;
	static CLOSING = 2;
	static CLOSED = 3;
	static instances: MockWebSocket[] = [];

	readyState = MockWebSocket.CONNECTING;
	onopen: (() => void) | null = null;
	onmessage: ((event: MessageEvent) => void) | null = null;
	onclose: ((event: CloseEvent) => void) | null = null;
	onerror: ((event: Event) => void) | null = null;
	sent: unknown[] = [];

	constructor(_url: string) {
		MockWebSocket.instances.push(this);
	}

	send(data: unknown) {
		this.sent.push(data);
	}

	close(code: number = 1000, reason: string = '') {
		if (this.readyState === MockWebSocket.CLOSED) return;
		this.readyState = MockWebSocket.CLOSED;
		this.onclose?.({ code, reason } as CloseEvent);
	}

	triggerOpen() {
		this.readyState = MockWebSocket.OPEN;
		this.onopen?.();
	}

	triggerError() {
		this.onerror?.(new Event('error'));
	}

	triggerMessage(data: string) {
		this.onmessage?.({ data } as MessageEvent);
	}

	static last(): MockWebSocket {
		const instance = MockWebSocket.instances.at(-1);
		if (!instance) {
			throw new Error('No MockWebSocket instances created');
		}
		return instance;
	}

	static reset() {
		MockWebSocket.instances = [];
	}
}

function setVisibility(state: 'visible' | 'hidden', dispatch: boolean = true) {
	Object.defineProperty(document, 'visibilityState', {
		value: state,
		configurable: true
	});
	if (dispatch) {
		document.dispatchEvent(new Event('visibilitychange'));
	}
}

function setOnline(online: boolean, dispatch: boolean = true) {
	Object.defineProperty(navigator, 'onLine', {
		value: online,
		configurable: true
	});
	if (dispatch) {
		window.dispatchEvent(new Event(online ? 'online' : 'offline'));
	}
}

describe('websocket FSM transitions', () => {
	let createWebSocketStore: typeof import('./websocket.svelte').createWebSocketStore;
	let ConnectionStateEnum: typeof import('./websocket.svelte').ConnectionState;
	let store: ReturnType<typeof import('./websocket.svelte').createWebSocketStore>;

	beforeAll(async () => {
		const module = await import('./websocket.svelte');
		createWebSocketStore = module.createWebSocketStore;
		ConnectionStateEnum = module.ConnectionState;
		vi.useFakeTimers();
		globalThis.WebSocket = MockWebSocket as unknown as typeof WebSocket;
		setVisibility('visible', false);
		setOnline(true, false);
		store = createWebSocketStore();
	});

	beforeEach(() => {
		store.disconnect();
		MockWebSocket.reset();
		vi.clearAllTimers();
		setVisibility('visible');
		setOnline(true);
	});

	afterAll(() => {
		vi.useRealTimers();
	});

	it('pauses retries while hidden and resumes on visible', async () => {
		const connectPromise = store.connect('token-1').catch(() => {});
		const socket = MockWebSocket.last();
		socket.triggerError();
		await connectPromise;

		expect(store.state.connectionState).toBe(ConnectionStateEnum.RECONNECTING);

		setVisibility('hidden');
		expect(store.state.connectionState).toBe(ConnectionStateEnum.SUSPENDED);

		vi.advanceTimersByTime(20000);
		expect(MockWebSocket.instances.length).toBe(1);

		setVisibility('visible');
		expect(MockWebSocket.instances.length).toBe(2);
		expect(store.state.reconnectAttempts).toBe(0);

		MockWebSocket.last().triggerOpen();
	});

	it('stays offline until network returns', async () => {
		const connectPromise = store.connect('token-2').catch(() => {});
		const socket = MockWebSocket.last();
		socket.triggerError();
		await connectPromise;

		expect(store.state.connectionState).toBe(ConnectionStateEnum.RECONNECTING);

		setOnline(false);
		expect(store.state.connectionState).toBe(ConnectionStateEnum.OFFLINE);

		vi.advanceTimersByTime(20000);
		expect(MockWebSocket.instances.length).toBe(2);

		setOnline(true);
		expect(MockWebSocket.instances.length).toBe(3);

		MockWebSocket.last().triggerOpen();
	});

	it('stops reconnecting on GameClosed', async () => {
		const connectPromise = store.connect('token-3').catch(() => {});
		const socket = MockWebSocket.last();
		socket.triggerOpen();
		await connectPromise;

		socket.triggerMessage(JSON.stringify({ type: 'GameClosed', reason: 'closed' }));

		expect(store.state.connectionState).toBe(ConnectionStateEnum.DISCONNECTED);
		expect(store.canReconnect()).toBe(false);
	});

	it('allows manual reconnect after session invalid', async () => {
		const connectPromise = store.connect('token-4').catch(() => {});
		const socket = MockWebSocket.last();
		socket.triggerOpen();
		await connectPromise;

		socket.triggerMessage(JSON.stringify({ type: 'Error', message: 'LobbyNotFound' }));

		expect(store.state.connectionState).toBe(ConnectionStateEnum.ERROR);

		store.manualReconnect();
		expect(MockWebSocket.instances.length).toBe(2);
		expect(store.state.reconnectAttempts).toBe(0);

		MockWebSocket.last().triggerOpen();
		expect(store.state.connectionState).toBe(ConnectionStateEnum.CONNECTED);
	});
});
