import { browser } from '$app/environment';
import { info } from '$lib/utils/logger';

function createStreamTimerStore() {
	const state = $state({
		timeLeft: 60,
		isActive: false,
		roundDuration: 60
	});

	let interval: number | undefined;
	let endTime: number = 0;

	function startTimer(duration: number = 60) {
		if (!browser) return;

		info('StreamTimer: Starting timer', { duration });

		state.roundDuration = duration;
		endTime = Date.now() + duration * 1000;
		state.timeLeft = duration;
		state.isActive = true;

		if (interval) {
			clearInterval(interval);
		}

		interval = window.setInterval(() => {
			const now = Date.now();
			const remaining = Math.max(0, Math.round(((endTime - now) / 1000) * 10) / 10);
			state.timeLeft = remaining;

			if (remaining <= 0) {
				state.isActive = false;
				if (interval) {
					clearInterval(interval);
					interval = undefined;
				}
				info('StreamTimer: Timer finished');
			}
		}, 100);
	}

	function stopTimer() {
		info('StreamTimer: Stopping timer');
		state.isActive = false;
		if (interval) {
			clearInterval(interval);
			interval = undefined;
		}
	}

	function resetTimer() {
		info('StreamTimer: Resetting timer');
		if (interval) {
			clearInterval(interval);
			interval = undefined;
		}
		state.timeLeft = state.roundDuration;
		state.isActive = false;
	}

	return {
		get state() {
			return state;
		},
		startTimer,
		stopTimer,
		resetTimer
	};
}

export const streamTimerStore = createStreamTimerStore();
