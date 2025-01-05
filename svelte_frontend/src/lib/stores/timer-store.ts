// src/lib/stores/timer-store.ts
import { writable } from 'svelte/store';
import { browser } from '$app/environment';

function createTimerStore() {
	const timeLeft = writable(60);
	let interval: number | undefined;

	function startTimer() {
		if (browser) {
			timeLeft.set(60);
			if (interval) clearInterval(interval);
			interval = window.setInterval(() => {
				timeLeft.update((t) => Math.max(0, t - 0.1));
			}, 100);
		}
	}

	function stopTimer() {
		if (interval) {
			clearInterval(interval);
			interval = undefined;
		}
	}

	return {
		subscribe: timeLeft.subscribe,
		startTimer,
		stopTimer
	};
}

export const timerStore = createTimerStore();
