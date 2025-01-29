<script lang="ts">
	import '../app.css';
	import { websocketStore } from '$lib/stores/websocket.svelte';
	import { gameStore } from '$lib/stores/game';

	let { children } = $props();

	// Use effect to watch for changes in the websocket messages
	$effect(() => {
		const messages = websocketStore.state.messages;
		if (messages.length > 0) {
			const lastMessage = messages[messages.length - 1];
			// Forward the message to the game store
			gameStore.processServerMessage(lastMessage);
		}
	});
</script>

{@render children()}
