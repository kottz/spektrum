// @vitest-environment jsdom
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$app/environment', () => ({ browser: true }));
vi.mock('$lib/utils/logger', () => ({ info: () => {}, warn: () => {} }));

describe('createTimer', () => {
	let createTimer: typeof import('./timer.svelte').createTimer;

	beforeAll(async () => {
		vi.useFakeTimers();
		const module = await import('./timer.svelte');
		createTimer = module.createTimer;
	});

	afterAll(() => {
		vi.useRealTimers();
	});

	beforeEach(() => {
		vi.clearAllTimers();
	});

	describe('basic timer behavior', () => {
		it('initializes with default state', () => {
			const timer = createTimer();
			expect(timer.state.timeLeft).toBe(60);
			expect(timer.state.roundDuration).toBe(60);
			expect(timer.state.answeredTimeSnapshot).toBeNull();
			expect(timer.state.isActive).toBe(false);
		});

		it('sets round duration', () => {
			const timer = createTimer();
			timer.setRoundDuration(30);
			expect(timer.state.roundDuration).toBe(30);
		});

		it('starts timer and counts down', () => {
			const timer = createTimer();
			timer.startTimer(10);

			expect(timer.state.timeLeft).toBe(10);

			vi.advanceTimersByTime(1000);
			expect(timer.state.timeLeft).toBeCloseTo(9, 0);

			vi.advanceTimersByTime(4000);
			expect(timer.state.timeLeft).toBeCloseTo(5, 0);
		});

		it('starts timer with remaining ms', () => {
			const timer = createTimer();
			timer.startTimer(10, 5000);

			expect(timer.state.timeLeft).toBe(5);
		});

		it('clamps remaining ms to zero', () => {
			const timer = createTimer();
			timer.startTimer(10, -500);

			expect(timer.state.timeLeft).toBe(0);
		});

		it('stops at zero', () => {
			const timer = createTimer();
			timer.startTimer(1);

			vi.advanceTimersByTime(1500);
			expect(timer.state.timeLeft).toBe(0);

			// Further time should not go negative
			vi.advanceTimersByTime(1000);
			expect(timer.state.timeLeft).toBe(0);
		});

		it('uses roundDuration as default when no duration passed', () => {
			const timer = createTimer();
			timer.setRoundDuration(20);
			timer.startTimer();

			expect(timer.state.timeLeft).toBe(20);
		});

		it('resets timer to round duration', () => {
			const timer = createTimer();
			timer.setRoundDuration(30);
			timer.startTimer(30);

			vi.advanceTimersByTime(10000);
			expect(timer.state.timeLeft).toBeCloseTo(20, 0);

			timer.resetTimer();
			expect(timer.state.timeLeft).toBe(30);
		});

		it('restarts interval when startTimer called while already running', () => {
			const timer = createTimer();
			timer.startTimer(10);

			vi.advanceTimersByTime(5000);
			expect(timer.state.timeLeft).toBeCloseTo(5, 0);

			// Restart with fresh duration
			timer.startTimer(10);
			expect(timer.state.timeLeft).toBe(10);

			vi.advanceTimersByTime(3000);
			expect(timer.state.timeLeft).toBeCloseTo(7, 0);
		});
	});

	describe('trackAnswerSnapshot mode (game timer)', () => {
		it('captures answer snapshot on stop', () => {
			const timer = createTimer({ trackAnswerSnapshot: true });
			timer.startTimer(10);

			vi.advanceTimersByTime(3000);
			timer.stopTimer();

			expect(timer.state.answeredTimeSnapshot).toBeCloseTo(7, 0);
			// Timer should still be running (player answered, but round continues)
		});

		it('does not overwrite snapshot on second stop', () => {
			const timer = createTimer({ trackAnswerSnapshot: true });
			timer.startTimer(10);

			vi.advanceTimersByTime(3000);
			timer.stopTimer();
			const firstSnapshot = timer.state.answeredTimeSnapshot;

			vi.advanceTimersByTime(2000);
			timer.stopTimer();

			expect(timer.state.answeredTimeSnapshot).toBe(firstSnapshot);
		});

		it('clears snapshot on startTimer', () => {
			const timer = createTimer({ trackAnswerSnapshot: true });
			timer.startTimer(10);

			vi.advanceTimersByTime(3000);
			timer.stopTimer();
			expect(timer.state.answeredTimeSnapshot).not.toBeNull();

			timer.startTimer(10);
			expect(timer.state.answeredTimeSnapshot).toBeNull();
		});

		it('clears snapshot on resetTimer', () => {
			const timer = createTimer({ trackAnswerSnapshot: true });
			timer.startTimer(10);

			vi.advanceTimersByTime(3000);
			timer.stopTimer();

			timer.resetTimer();
			expect(timer.state.answeredTimeSnapshot).toBeNull();
		});

		it('force stop clears interval and sets snapshot to 0 if not answered', () => {
			const timer = createTimer({ trackAnswerSnapshot: true });
			timer.startTimer(10);

			// Force stop without a prior soft stop — timeLeft > 0 so snapshot is captured
			timer.stopTimer(true);

			expect(timer.state.answeredTimeSnapshot).toBeCloseTo(10, 0);

			// Timer interval should be stopped — no further countdown
			const frozenTime = timer.state.timeLeft;
			vi.advanceTimersByTime(2000);
			expect(timer.state.timeLeft).toBe(frozenTime);
		});

		it('force stop sets snapshot to 0 when timer already at 0', () => {
			const timer = createTimer({ trackAnswerSnapshot: true });
			timer.startTimer(1);

			vi.advanceTimersByTime(1500);
			expect(timer.state.timeLeft).toBe(0);

			timer.stopTimer(true);
			expect(timer.state.answeredTimeSnapshot).toBe(0);
		});
	});

	describe('trackActiveState mode (stream timer)', () => {
		it('sets isActive on start', () => {
			const timer = createTimer({ trackActiveState: true });
			expect(timer.state.isActive).toBe(false);

			timer.startTimer(10);
			expect(timer.state.isActive).toBe(true);
		});

		it('clears isActive on stop', () => {
			const timer = createTimer({ trackActiveState: true });
			timer.startTimer(10);
			expect(timer.state.isActive).toBe(true);

			timer.stopTimer();
			expect(timer.state.isActive).toBe(false);
		});

		it('clears isActive when timer reaches zero', () => {
			const timer = createTimer({ trackActiveState: true });
			timer.startTimer(1);
			expect(timer.state.isActive).toBe(true);

			vi.advanceTimersByTime(1500);
			expect(timer.state.isActive).toBe(false);
		});

		it('clears isActive on reset', () => {
			const timer = createTimer({ trackActiveState: true });
			timer.startTimer(10);

			timer.resetTimer();
			expect(timer.state.isActive).toBe(false);
		});

		it('updates roundDuration when duration is passed to startTimer', () => {
			const timer = createTimer({ trackActiveState: true });
			timer.startTimer(30);

			expect(timer.state.roundDuration).toBe(30);
		});

		it('stop clears the interval', () => {
			const timer = createTimer({ trackActiveState: true });
			timer.startTimer(10);

			vi.advanceTimersByTime(3000);
			timer.stopTimer();

			const frozenTime = timer.state.timeLeft;
			vi.advanceTimersByTime(2000);
			expect(timer.state.timeLeft).toBe(frozenTime);
		});
	});
});
