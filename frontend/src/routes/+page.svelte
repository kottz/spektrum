<script lang="ts">
	import HomeScreen from '$lib/components/game/home-screen.svelte';
	import PlayerGameScreen from '$lib/components/game/player-game-screen.svelte';
	import AdminGameScreen from '$lib/components/game/admin/admin-game-screen.svelte';
	import { gameStore } from '$lib/stores/game.svelte';

	let playerName = $state('');
	let lobbyCode = $state('');
</script>

{#if gameStore.state.lobbyId}
	{#if gameStore.state.isAdmin}
		<AdminGameScreen />
	{:else}
		<PlayerGameScreen />
	{/if}
{:else}
	<HomeScreen bind:playerName bind:lobbyCode />
{/if}
