// src/routes/+page.svelte
<script lang="ts">
  import HomeScreen from "$lib/components/game/home-screen.svelte";
  import GameScreen from "$lib/components/game/game-screen.svelte";
  import AdminScreen from "$lib/components/game/admin-screen.svelte";
  import { gameStore } from "$lib/stores/game";

  // Shared state between components
  let screen: "home" | "game" | "admin" = "home";
  let playerName = "";
  let lobbyCode = "";

  // Watch for game state changes
  $: if (!$gameStore.lobbyId) {
    screen = "home";
  } else if ($gameStore.isAdmin) {
    screen = "admin";
  }
</script>

{#if $gameStore.lobbyId}
  {#if $gameStore.isAdmin}
    <AdminScreen />
  {:else}
    <GameScreen {playerName} {lobbyCode} />
  {/if}
{:else}
  <HomeScreen
    bind:screen
    bind:playerName
    bind:lobbyCode
  />
{/if}
