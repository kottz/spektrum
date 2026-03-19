<script lang="ts">
	import HomeScreen from '$lib/components/game/home-screen.svelte';
	import PlayerGameScreen from '$lib/components/game/player-game-screen.svelte';
	import AdminGameScreen from '$lib/components/game/admin/admin-game-screen.svelte';
	import ConnectionLostOverlay from '$lib/components/websocket-disconnect-overlay.svelte';
	import { gameStore } from '$lib/stores/game.svelte';
</script>

<ConnectionLostOverlay />

{#if gameStore.state.playerId}
	{#if gameStore.state.isAdmin}
		<AdminGameScreen />
	{:else}
		<PlayerGameScreen />
	{/if}
{:else}
	<HomeScreen />
{/if}
