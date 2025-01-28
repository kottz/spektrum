<script lang="ts">
	import HomeScreen from '$lib/components/game/home-screen.svelte';
	import PlayerGameScreen from '$lib/components/game/player-game-screen.svelte';
	import AdminGameScreen from '$lib/components/game/admin-game-screen.svelte';
	import { gameStore } from '$lib/stores/game';

	let playerName = $state('');
	let lobbyCode = $state('');
</script>

{#if $gameStore.lobbyId}
	{#if $gameStore.isAdmin}
		<AdminGameScreen />
	{:else}
		<PlayerGameScreen {playerName} {lobbyCode} />
	{/if}
{:else}
	<HomeScreen bind:playerName bind:lobbyCode />
{/if}
