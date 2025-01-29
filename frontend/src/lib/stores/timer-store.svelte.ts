// src/lib/stores/timer-store.ts
import { browser } from '$app/environment';

function createTimerStore() {
	const state = $state({
		timeLeft: 60
	});

	let interval: number | undefined;

	function startTimer() {
		if (browser) {
			state.timeLeft = 60;

			if (interval) {
				clearInterval(interval);
			}

			interval = window.setInterval(() => {
				state.timeLeft = Math.max(0, state.timeLeft - 0.1);
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
		state,
		startTimer,
		stopTimer
	};
}

export const timerStore = createTimerStore();
