import { browser } from '$app/environment';
import { info } from '$lib/utils/logger';

interface TimerOptions {
	trackAnswerSnapshot?: boolean;
	trackActiveState?: boolean;
	label?: string;
}

interface TimerState {
	timeLeft: number;
	roundDuration: number;
	answeredTimeSnapshot: number | null;
	isActive: boolean;
}

export function createTimer(options: TimerOptions = {}) {
	const { trackAnswerSnapshot = false, trackActiveState = false, label } = options;

	const state = $state<TimerState>({
		timeLeft: 60,
		roundDuration: 60,
		answeredTimeSnapshot: null,
		isActive: false
	});

	let interval: number | undefined;
	let endTime: number = 0;

	function log(message: string, data?: Record<string, unknown>) {
		if (label) {
			info(`${label}: ${message}`, data);
		}
	}

	function setRoundDuration(duration: number) {
		state.roundDuration = duration;
	}

	function startTimer(duration?: number, remainingMs?: number) {
		if (!browser) return;

		const durationSeconds = duration ?? state.roundDuration;
		log('Starting timer', { duration: durationSeconds });

		if (trackActiveState || duration !== undefined) {
			state.roundDuration = durationSeconds;
		}

		const startRemainingMs =
			remainingMs !== undefined ? Math.max(0, remainingMs) : durationSeconds * 1000;

		endTime = Date.now() + startRemainingMs;
		state.timeLeft = Math.round((startRemainingMs / 1000) * 10) / 10;

		if (trackAnswerSnapshot) {
			state.answeredTimeSnapshot = null;
		}
		if (trackActiveState) {
			state.isActive = true;
		}

		if (interval) {
			clearInterval(interval);
		}

		interval = window.setInterval(() => {
			const now = Date.now();
			const remaining = Math.max(0, Math.round(((endTime - now) / 1000) * 10) / 10);
			state.timeLeft = remaining;

			if (remaining <= 0) {
				if (trackActiveState) {
					state.isActive = false;
				}
				if (interval) {
					clearInterval(interval);
					interval = undefined;
				}
				log('Timer finished');
			}
		}, 100);
	}

	function stopTimer(forceStopActualTimer: boolean = false) {
		if (trackAnswerSnapshot) {
			if (state.answeredTimeSnapshot === null && state.timeLeft > 0) {
				state.answeredTimeSnapshot = state.timeLeft;
			}

			if (forceStopActualTimer) {
				if (interval) {
					clearInterval(interval);
					interval = undefined;
				}
				if (state.answeredTimeSnapshot === null) {
					state.answeredTimeSnapshot = 0;
				}
			}
		} else {
			log('Stopping timer');
			if (trackActiveState) {
				state.isActive = false;
			}
			if (interval) {
				clearInterval(interval);
				interval = undefined;
			}
		}
	}

	function resetTimer() {
		log('Resetting timer');
		if (interval) {
			clearInterval(interval);
			interval = undefined;
		}
		state.timeLeft = state.roundDuration;
		if (trackAnswerSnapshot) {
			state.answeredTimeSnapshot = null;
		}
		if (trackActiveState) {
			state.isActive = false;
		}
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

export const timerStore = createTimer({ trackAnswerSnapshot: true });
export const streamTimerStore = createTimer({ trackActiveState: true, label: 'StreamTimer' });
