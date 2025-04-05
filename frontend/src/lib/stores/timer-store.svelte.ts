// src/lib/stores/timer-store.ts
import { browser } from '$app/environment';

function createTimerStore() {
	const state = $state({
		timeLeft: 60
	});

	let interval: number | undefined;
	let endTime: number = 0;

	function startTimer() {
		if (browser) {
			// Set the end time based on the current time
			endTime = Date.now() + 60 * 1000;
			state.timeLeft = 60;

			if (interval) {
				clearInterval(interval);
			}

			interval = window.setInterval(() => {
				const now = Date.now();
				const remaining = Math.max(0, (endTime - now) / 1000);
				state.timeLeft = remaining;

				if (remaining <= 0) {
					stopTimer();
				}
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
