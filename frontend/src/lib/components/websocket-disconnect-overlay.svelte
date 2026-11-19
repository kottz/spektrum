<script lang="ts">
	import { websocketStore, ConnectionState } from '#lib/stores/websocket.svelte.ts';
	import { Button } from '#lib/components/ui/button/index.ts';
	import { fade } from 'svelte/transition';

	// Get state values using runes
	let connectionState = $derived(websocketStore.state.connectionState);
	let reconnectAttempts = $derived(websocketStore.state.reconnectAttempts);
	let nextAttemptTime = $derived(websocketStore.state.nextAttemptTime);
	let error = $derived(websocketStore.state.error);
	let isReconnecting = $derived(websocketStore.isReconnecting);
	let maxReconnectAttempts = $derived(websocketStore.maxReconnectAttempts);

	// Derived values for UI
	let shouldShow = $derived(
		connectionState === ConnectionState.ERROR ||
			connectionState === ConnectionState.RECONNECTING ||
			connectionState === ConnectionState.OFFLINE
	);

	let title = $derived.by(() => {
		switch (connectionState) {
			case ConnectionState.RECONNECTING:
				return `Reconnecting (Attempt ${reconnectAttempts}/${maxReconnectAttempts})`;
			case ConnectionState.OFFLINE:
				return 'You are offline';
			case ConnectionState.ERROR:
				return error || 'Connection Lost';
			default:
				return 'Connection Lost';
		}
	});

	// Date.now() is not reactive; tick while a countdown is on screen
	let now = $state(Date.now());
	$effect(() => {
		if (nextAttemptTime === null) return;
		now = Date.now();
		const interval = window.setInterval(() => (now = Date.now()), 250);
		return () => window.clearInterval(interval);
	});

	let message = $derived.by(() => {
		if (connectionState === ConnectionState.RECONNECTING && nextAttemptTime !== null) {
			const secondsUntilReconnect = Math.max(0, Math.ceil((nextAttemptTime - now) / 1000));
			return `Next attempt in ${secondsUntilReconnect} seconds...`;
		}
		if (connectionState === ConnectionState.OFFLINE) {
			return 'Waiting for network connectivity...';
		}
		return '';
	});

	let showManualReconnect = $derived(connectionState === ConnectionState.ERROR);
</script>

{#if shouldShow}
	<div
		class="bg-background/80 fixed inset-0 z-50 flex items-center justify-center backdrop-blur-xs"
		transition:fade={{ duration: 150 }}
	>
		<div
			class="bg-background/95 flex flex-col items-center space-y-4 rounded-lg border p-6 shadow-lg"
		>
			{#if isReconnecting}
				<div class="border-primary h-8 w-8 animate-spin rounded-full border-b-2"></div>
			{/if}
			<p class="text-foreground text-lg font-medium">{title}</p>
			{#if message}
				<p class="text-muted-foreground text-sm">{message}</p>
			{/if}
			{#if showManualReconnect}
				<div class="flex flex-col items-center gap-2 pt-2">
					<p class="text-muted-foreground text-sm">Reconnect to continue in this window.</p>
					<Button variant="default" onclick={websocketStore.manualReconnect}>Reconnect</Button>
				</div>
			{/if}
		</div>
	</div>
{/if}
