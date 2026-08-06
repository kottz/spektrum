<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent } from '$lib/components/ui/card';
	import { uiStore } from '$lib/stores/ui.store.svelte';
	import { gameStore } from '$lib/stores/game.svelte';
	import { broadcastService } from '$lib/services/broadcast.service';
	import type { BroadcastMessage } from '$lib/services/broadcast.service';
	import { MonitorPlay, Eye, EyeOff, ExternalLink } from '@lucide/svelte';
	import { onDestroy } from 'svelte';
	import { info } from '$lib/utils/logger';
	import { createStreamStateSnapshot } from '$lib/utils/stream-state';

	const streamWindowState = $derived(uiStore.streamWindow);
	const gameState = $derived(gameStore.state);
	let isStreamContentVisible = $state(true);

	function handleStreamReady() {
		if (gameState.joinCode) {
			const streamState = createStreamStateSnapshot(gameState);

			info('StreamControls: Stream ready, sending initial state with join code and phase', {
				joinCode: gameState.joinCode,
				phase: gameState.phase
			});
			broadcastService.broadcastInitialState(
				'SpektrumGame',
				gameState.joinCode,
				gameState.roundDuration,
				streamState
			);
		}
	}

	function handleOpenStreamWindow(): void {
		info('StreamControls: Opening stream window');

		// Initialize broadcast service for admin if not already done
		if (!broadcastService.getIsInitialized() && !broadcastService.getIsStreamWindow()) {
			broadcastService.initialize(false); // false = admin window
		}

		// Listen before opening so the new window's ready signal cannot race us.
		broadcastService.addListener(handleBroadcastMessage);
		uiStore.openStreamWindow();
	}

	function handleBroadcastMessage(message: BroadcastMessage) {
		if (message.type === 'STREAM_READY') {
			info('StreamControls: Received stream ready signal');
			broadcastService.setHasActiveStreams(true);
			handleStreamReady();
		} else if (message.type === 'STREAM_DISCONNECTED') {
			info('StreamControls: Stream window disconnected');
			broadcastService.setHasActiveStreams(false);
		}
	}

	function handleCloseStreamWindow(): void {
		info('StreamControls: Closing stream window');
		broadcastService.setHasActiveStreams(false);
		uiStore.closeStreamWindow();
		// Remove the listener when closing the stream window
		broadcastService.removeListener(handleBroadcastMessage);
	}

	// Clean up listener when component is destroyed
	onDestroy(() => {
		broadcastService.removeListener(handleBroadcastMessage);
	});

	function handleToggleVisibility(): void {
		if (isStreamContentVisible) {
			info('StreamControls: Hiding stream content');
			broadcastService.broadcastStreamControl('hide');
		} else {
			info('StreamControls: Showing stream content');
			broadcastService.broadcastStreamControl('show');
		}
		isStreamContentVisible = !isStreamContentVisible;
	}
</script>

<Card class="flex h-full">
	<CardContent class="flex flex-1 items-center p-4">
		{#if streamWindowState.isOpen}
			<div class="flex w-full gap-2">
				<Button onclick={handleToggleVisibility} variant="outline" size="sm">
					{#if isStreamContentVisible}
						<Eye class="h-4 w-4" />
					{:else}
						<EyeOff class="h-4 w-4" />
					{/if}
				</Button>
				<Button onclick={handleCloseStreamWindow} variant="destructive" size="sm" class="flex-1">
					<ExternalLink class="mr-2 h-4 w-4" /> Close
				</Button>
			</div>
		{:else}
			<Button onclick={handleOpenStreamWindow} variant="default" class="w-full">
				<MonitorPlay class="mr-2 h-4 w-4" /> Stream View
			</Button>
		{/if}
	</CardContent>
</Card>
