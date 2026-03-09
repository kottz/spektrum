// @vitest-environment jsdom
import { beforeAll, beforeEach, afterAll, describe, expect, it, vi } from 'vitest';

describe('createNotificationStore', () => {
	let createNotificationStore: typeof import('./notification-store.svelte').createNotificationStore;

	beforeAll(async () => {
		vi.useFakeTimers();
		const module = await import('./notification-store.svelte');
		createNotificationStore = module.createNotificationStore;
	});

	afterAll(() => {
		vi.useRealTimers();
	});

	beforeEach(() => {
		vi.clearAllTimers();
	});

	it('starts with empty list', () => {
		const store = createNotificationStore();
		expect(store.list).toEqual([]);
	});

	it('adds a notification with default type', () => {
		const store = createNotificationStore();
		store.add('hello');

		expect(store.list).toHaveLength(1);
		expect(store.list[0].message).toBe('hello');
		expect(store.list[0].type).toBe('default');
		expect(store.list[0].id).toBeTypeOf('number');
	});

	it('adds a notification with specified type', () => {
		const store = createNotificationStore();
		store.add('error occurred', 'destructive');

		expect(store.list[0].type).toBe('destructive');
	});

	it('adds multiple notifications', () => {
		const store = createNotificationStore();
		store.add('first');
		store.add('second', 'success');
		store.add('third', 'destructive');

		expect(store.list).toHaveLength(3);
		expect(store.list.map((n) => n.message)).toEqual(['first', 'second', 'third']);
	});

	it('removes a notification by id', () => {
		const store = createNotificationStore();
		store.add('to keep');
		vi.advanceTimersByTime(1); // ensure different Date.now() ids
		store.add('to remove');

		const idToRemove = store.list[1].id;
		store.remove(idToRemove);

		expect(store.list).toHaveLength(1);
		expect(store.list[0].message).toBe('to keep');
	});

	it('remove with non-existent id is a no-op', () => {
		const store = createNotificationStore();
		store.add('hello');

		store.remove(999999);
		expect(store.list).toHaveLength(1);
	});

	it('auto-dismisses after 5 seconds', () => {
		const store = createNotificationStore();
		store.add('will disappear');

		expect(store.list).toHaveLength(1);

		vi.advanceTimersByTime(4999);
		expect(store.list).toHaveLength(1);

		vi.advanceTimersByTime(1);
		expect(store.list).toHaveLength(0);
	});

	it('auto-dismisses only the specific notification', () => {
		const store = createNotificationStore();
		store.add('first');

		vi.advanceTimersByTime(2000);
		store.add('second');

		// 3 more seconds: first should be gone, second still present
		vi.advanceTimersByTime(3000);
		expect(store.list).toHaveLength(1);
		expect(store.list[0].message).toBe('second');

		// 2 more seconds: second should be gone too
		vi.advanceTimersByTime(2000);
		expect(store.list).toHaveLength(0);
	});

	it('manual remove before auto-dismiss does not cause errors', () => {
		const store = createNotificationStore();
		store.add('manual remove');

		const id = store.list[0].id;
		store.remove(id);
		expect(store.list).toHaveLength(0);

		// Auto-dismiss fires but notification already gone — should not throw
		vi.advanceTimersByTime(5000);
		expect(store.list).toHaveLength(0);
	});
});
