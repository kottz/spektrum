import { info } from '$lib/utils/logger';
import { gameStore } from '$lib/stores/game.svelte';
import { GamePhase } from '$lib/types/game';

export type AppScreen = 'home' | 'lobby' | 'adminGame' | 'playerGame';

interface StreamWindowInfo {
	window: Window | null;
	isOpen: boolean;
}

function createUiStore() {
	const streamWindow = $state<StreamWindowInfo>({
		window: null,
		isOpen: false
	});

	function openStreamWindow(): void {
		if (streamWindow.window && !streamWindow.window.closed) {
			streamWindow.window.focus();
			return;
		}

		try {
			const windowFeatures = 'width=1280,height=720,left=100,top=100,resizable=yes,scrollbars=yes';
			const newWindow = window.open('/stream', '_blank', windowFeatures);

			if (newWindow) {
				streamWindow.window = newWindow;
				streamWindow.isOpen = true;

				// Check if window is closed periodically
				const checkClosed = setInterval(() => {
					if (newWindow.closed) {
						streamWindow.window = null;
						streamWindow.isOpen = false;
						clearInterval(checkClosed);
						info('UI Store: Stream window was closed by user.');
					}
				}, 1000);

				info('UI Store: Stream window opened.');
			} else {
				info('UI Store: Failed to open stream window (popup blocked?).');
			}
		} catch (error) {
			info('UI Store: Error opening stream window:', error);
		}
	}

	function closeStreamWindow(): void {
		if (streamWindow.window && !streamWindow.window.closed) {
			streamWindow.window.close();
		}
		info('UI Store: Attempted to close stream window.');
	}

	// Determine if stream window can be opened (when admin is in a game)
	const canOpenStreamWindow = $derived(
		gameStore.state.isAdmin && gameStore.state.phase !== GamePhase.GameClosed
	);

	return {
		get streamWindow() {
			return streamWindow;
		},
		get canOpenStreamWindow() {
			return canOpenStreamWindow;
		},
		openStreamWindow,
		closeStreamWindow
	};
}

export const uiStore = createUiStore();
