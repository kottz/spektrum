<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { websocketStore } from '$lib/stores/websocket';
	import { gameStore } from '$lib/stores/game';

	let { children } = $props();

	let unsubscribe: () => void;

	onMount(() => {
		// Subscribe to websocket changes
		unsubscribe = websocketStore.subscribe((ws) => {
			if (ws.messages.length > 0) {
				const lastMessage = ws.messages[ws.messages.length - 1];
				// Forward the message to the game store
				gameStore.processServerMessage(lastMessage);
			}
		});

		// Clean up subscription on unmount
		return () => {
			if (unsubscribe) {
				unsubscribe();
			}
		};
	});
</script>

{@render children()}
