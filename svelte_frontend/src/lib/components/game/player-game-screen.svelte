<script lang="ts">
  import { gameStore } from "../../stores/game";
  import QuestionView from "./question-view.svelte";
  import ScoreView from "./score-view.svelte";
  import GameOver from "./game-over.svelte";
  import { GamePhase } from "../../types/game";

  export let playerName: string;
  export let lobbyCode: string;

  $: phase = $gameStore.phase;
</script>

{#if phase === GamePhase.Question}
  <QuestionView />
{:else if phase === GamePhase.Score}
  <ScoreView />
{:else if phase === GamePhase.GameOver}
  <GameOver />
{/if}
