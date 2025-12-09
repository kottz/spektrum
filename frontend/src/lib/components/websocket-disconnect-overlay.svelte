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

	let showManualReconnect = $derived(connectionState === ConnectionState.ERROR && !canReconnect());

	async function handleManualReconnect() {
		const sessionToken = gameStore.state.sessionToken;
		if (sessionToken) {
			await gameActions.joinGame(sessionToken);
		}
	}
</script>

{#if shouldShow}
	<div
		class="bg-background/80 fixed inset-0 z-50 flex items-center justify-center backdrop-blur-xs"
		transition:fade={{ duration: 150 }}
	>
		<div
			class="bg-background/95 flex flex-col items-center space-y-4 rounded-lg border p-6 shadow-lg"
		>
			{#if isReconnecting()}
				<div class="border-primary h-8 w-8 animate-spin rounded-full border-b-2"></div>
			{/if}
			<p class="text-foreground text-lg font-medium">{title()}</p>
			{#if message()}
				<p class="text-muted-foreground text-sm">{message()}</p>
			{/if}
			{#if showManualReconnect}
				<div class="flex flex-col items-center gap-2 pt-2">
					<p class="text-muted-foreground text-sm">
						Automatic reconnection failed. Try to reconnect manually.
					</p>
					<Button variant="default" onclick={handleManualReconnect}>Reconnect</Button>
				</div>
			{/if}
		</div>
	</div>
{/if}
