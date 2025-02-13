<script lang="ts">
	import { websocketStore, ConnectionState } from '$lib/stores/websocket.svelte';
	import { gameStore } from '$lib/stores/game.svelte';
	import { gameActions } from '$lib/stores/game-actions';
	import { Button } from '$lib/components/ui/button';
	import { fade } from 'svelte/transition';

	// Get state values using runes
	let connectionState = $derived(websocketStore.state.connectionState);
	let reconnectAttempts = $derived(websocketStore.state.reconnectAttempts);
	let timeUntilReconnect = $derived(websocketStore.timeUntilReconnect);
	let error = $derived(websocketStore.state.error);
	let isReconnecting = $derived(websocketStore.isReconnecting);
	let canReconnect = $derived(websocketStore.canReconnect);

	// Derived values for UI
	let shouldShow = $derived(
		connectionState === ConnectionState.ERROR || connectionState === ConnectionState.RECONNECTING
	);

	let title = $derived(() => {
		switch (connectionState) {
			case ConnectionState.RECONNECTING:
				return `Reconnecting (Attempt ${reconnectAttempts}/5)`;
			case ConnectionState.ERROR:
				return error || 'Connection Lost';
			default:
				return 'Connection Lost';
		}
	});

	let message = $derived(() => {
		if (connectionState === ConnectionState.RECONNECTING && timeUntilReconnect() !== null) {
			const secondsUntilReconnect = Math.ceil((timeUntilReconnect() as number) / 1000);
			return `Next attempt in ${secondsUntilReconnect} seconds...`;
		}
		return '';
	});

	let showManualReconnect = $derived(connectionState === ConnectionState.ERROR && !canReconnect);

	async function handleManualReconnect() {
		const playerId = gameStore.state.playerId;
		if (playerId) {
			await gameActions.joinGame(playerId);
		}
	}
</script>

{#if shouldShow}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm"
		transition:fade={{ duration: 150 }}
	>
		<div
			class="flex flex-col items-center space-y-4 rounded-lg border bg-background/95 p-6 shadow-lg"
		>
			{#if isReconnecting()}
				<div class="h-8 w-8 animate-spin rounded-full border-b-2 border-primary"></div>
			{/if}
			<p class="text-lg font-medium text-foreground">{title()}</p>
			{#if message()}
				<p class="text-sm text-muted-foreground">{message()}</p>
			{/if}
			{#if showManualReconnect}
				<div class="flex flex-col items-center gap-2 pt-2">
					<p class="text-sm text-muted-foreground">
						Automatic reconnection failed. Would you like to try manually?
					</p>
					<Button variant="default" on:click={handleManualReconnect}>Try Reconnecting</Button>
				</div>
			{/if}
		</div>
	</div>
{/if}
