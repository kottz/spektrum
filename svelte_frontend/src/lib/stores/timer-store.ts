// src/lib/stores/timer-store.ts
import { browser } from '$app/environment';
import { gameStore } from './game';
import { writable, derived, get } from 'svelte/store';

function createTimerStore() {
    const timeLeft = writable(60);
    let intervalId: number | undefined;
    let startTime: number | null = null;

    if (browser) {
        // Subscribe to phase changes
        gameStore.subscribe(state => {
            if (state.phase === 'question' && !intervalId) {
                // Only start timer if not already running
                startTimer();
            } else if (state.phase !== 'question') {
                // Stop and reset only on phase change
                stopTimer();
                timeLeft.set(60);
            }
        });
    }

    function startTimer() {
        startTime = Date.now();
        if (intervalId) clearInterval(intervalId);

        intervalId = window.setInterval(() => {
            if (startTime) {
                const elapsed = (Date.now() - startTime) / 1000;
                timeLeft.set(Math.max(0, 60 - elapsed));

                const currentTimeLeft = get(timeLeft);
                if (currentTimeLeft <= 0 && intervalId) {
                    clearInterval(intervalId);
                }
            }
        }, 100);
    }

    function stopTimer() {
        if (intervalId) {
            clearInterval(intervalId);
            intervalId = undefined;
        }
    }

    return {
        subscribe: timeLeft.subscribe,
        startTimer,
        stopTimer
    };
}

export const timerStore = createTimerStore();
