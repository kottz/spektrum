<script lang="ts">
	import HomeScreen from '$lib/components/game/home-screen.svelte';
	import PlayerGameScreen from '$lib/components/game/player-game-screen.svelte'; // For players
	import AdminGameScreen from '$lib/components/game/admin-game-screen.svelte'; // For admins
	import { gameStore } from '$lib/stores/game';

	let playerName = '';
	let lobbyCode = '';
</script>

{#if $gameStore.lobbyId}
	{#if $gameStore.isAdmin}
		<AdminGameScreen /> <!-- Only admin controls and info -->
	{:else}
		<PlayerGameScreen {playerName} {lobbyCode} /> <!-- Full game interface for players -->
	{/if}
{:else}
	<HomeScreen bind:playerName bind:lobbyCode />
{/if}
