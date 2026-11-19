// @vitest-environment jsdom
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$app/env', () => ({ browser: true }));
vi.mock('$app/env/public', () => ({ PUBLIC_SPEKTRUM_WS_SERVER_URL: 'ws://test' }));
vi.mock('#lib/utils/logger.ts', () => ({ info: () => {}, warn: () => {} }));

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

	/** Opens the transport and has the server accept the session. */
	function openAndAccept(socket: MockWebSocket) {
		socket.triggerOpen();
		socket.triggerMessage(
			JSON.stringify({ type: 'Connected', player_id: 'p1', name: 'P', round_duration: 60 })
		);
	}

	it('stops reconnecting on GameClosed', async () => {
		const connectPromise = store.connect('token-3').catch(() => {});
		const socket = MockWebSocket.last();
		openAndAccept(socket);
		await connectPromise;

		socket.triggerMessage(JSON.stringify({ type: 'GameClosed', reason: 'closed' }));

		expect(store.state.connectionState).toBe(ConnectionStateEnum.DISCONNECTED);
		vi.advanceTimersByTime(20000);
		expect(MockWebSocket.instances.length).toBe(1);
	});

	it('stops reconnecting when the server reports the session invalid', async () => {
		const connectPromise = store.connect('token-4').catch(() => {});
		const socket = MockWebSocket.last();
		openAndAccept(socket);
		await connectPromise;

		socket.triggerMessage(
			JSON.stringify({ type: 'Error', code: 'session_invalid', message: 'Lobby not found' })
		);

		expect(store.state.connectionState).toBe(ConnectionStateEnum.DISCONNECTED);
		vi.advanceTimersByTime(20000);
		expect(MockWebSocket.instances.length).toBe(1);
	});

	it('connect resolves only once the server accepts the session', async () => {
		let settled: 'resolved' | 'rejected' | null = null;
		const connectPromise = store.connect('token-5').then(
			() => (settled = 'resolved'),
			() => (settled = 'rejected')
		);
		const socket = MockWebSocket.last();
		socket.triggerOpen();
		await Promise.resolve();
		expect(settled).toBeNull();

		socket.triggerMessage(
			JSON.stringify({ type: 'Error', code: 'session_invalid', message: 'Player not found' })
		);
		await connectPromise;
		expect(settled).toBe('rejected');
	});

	it('heartbeat timeout fires when echoes stop arriving', async () => {
		const connectPromise = store.connect('token-6').catch(() => {});
		const socket = MockWebSocket.last();
		openAndAccept(socket);
		await connectPromise;

		// Beats keep being sent every 3s but the server never echoes them
		vi.advanceTimersByTime(3000 + 4000 + 100);
		expect(store.state.connectionState).toBe(ConnectionStateEnum.RECONNECTING);
	});

	it('superseded session stops without auto-reconnect and offers a manual reconnect', async () => {
		const connectPromise = store.connect('token-7').catch(() => {});
		const socket = MockWebSocket.last();
		openAndAccept(socket);
		await connectPromise;

		socket.triggerMessage(
			JSON.stringify({ type: 'Error', code: 'session_superseded', message: 'Opened elsewhere' })
		);
		expect(store.state.connectionState).toBe(ConnectionStateEnum.ERROR);
		vi.advanceTimersByTime(30000);
		expect(MockWebSocket.instances.length).toBe(1);

		store.manualReconnect();
		expect(MockWebSocket.instances.length).toBe(2);
	});

	it('an error state survives the tab becoming visible again', async () => {
		const connectPromise = store.connect('token-8').catch(() => {});
		const socket = MockWebSocket.last();
		openAndAccept(socket);
		await connectPromise;
		socket.triggerMessage(
			JSON.stringify({ type: 'Error', code: 'session_superseded', message: 'Opened elsewhere' })
		);

		setVisibility('hidden');
		setVisibility('visible');
		setOnline(true);
		expect(store.state.connectionState).toBe(ConnectionStateEnum.ERROR);
	});

	it('an unparseable message is ignored rather than breaking the connection', async () => {
		const connectPromise = store.connect('token-9').catch(() => {});
		const socket = MockWebSocket.last();
		openAndAccept(socket);
		await connectPromise;

		socket.triggerMessage('{not json');
		expect(store.state.connectionState).toBe(ConnectionStateEnum.CONNECTED);
	});

	it('send reports failure while the socket is not open', async () => {
		store.connect('token-10').catch(() => {});
		expect(store.send({ type: 'Answer', answer: 'Red' })).toBe(false);
		openAndAccept(MockWebSocket.last());
		expect(store.send({ type: 'Answer', answer: 'Red' })).toBe(true);
	});
});
