// src/lib/stores/timer-store.svelte.ts
import { browser } from '$app/environment';

function createTimerStore() {
	const state = $state({
		timeLeft: 60,
		answeredTimeSnapshot: null as number | null,
		roundDuration: 60
	});

	let interval: number | undefined;
	let endTime: number = 0;

	function setRoundDuration(duration: number) {
		state.roundDuration = duration;
	}

	function startTimer(totalSeconds?: number, remainingMs?: number) {
		if (browser) {
			const durationSeconds = totalSeconds ?? state.roundDuration;
			const startRemainingMs =
				remainingMs !== undefined ? Math.max(0, remainingMs) : durationSeconds * 1000;

			endTime = Date.now() + startRemainingMs;
			state.timeLeft = Math.round((startRemainingMs / 1000) * 10) / 10;
			state.answeredTimeSnapshot = null;

			if (interval) {
				clearInterval(interval);
			}

			interval = window.setInterval(() => {
				const now = Date.now();
				const remaining = Math.max(0, Math.round(((endTime - now) / 1000) * 10) / 10);
				state.timeLeft = remaining;

				if (remaining <= 0) {
					if (interval) {
						// If time truly runs out, clear the interval
						clearInterval(interval);
						interval = undefined;
					}
				}
			}, 100);
		}
	}

	function stopTimer(forceStopActualTimer: boolean = false) {
		if (state.answeredTimeSnapshot === null && state.timeLeft > 0) {
			state.answeredTimeSnapshot = state.timeLeft;
		}

		// If admin forces a full stop, clear the interval.
		if (forceStopActualTimer) {
			if (interval) {
				clearInterval(interval);
				interval = undefined;
			}
			if (state.answeredTimeSnapshot === null) {
				state.answeredTimeSnapshot = 0;
			}
		}
	}

	function resetTimer() {
		if (interval) {
			clearInterval(interval);
			interval = undefined;
		}
		state.timeLeft = state.roundDuration;
		state.answeredTimeSnapshot = null;
	}

	return {
		get state() {
			return state;
		},
		setRoundDuration,
		startTimer,
		stopTimer,
		resetTimer
	};
}

export const timerStore = createTimerStore();
