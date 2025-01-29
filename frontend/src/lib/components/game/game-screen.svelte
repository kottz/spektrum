<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import QuestionView from './question-view.svelte';
	import ScoreView from './score-view.svelte';
	import LobbyView from './lobby-view.svelte';
	import GameOver from './game-over.svelte';
	import { GamePhase } from '$lib/types/game';

	//let { playerName, lobbyCode } = $props<{ playerName: string; lobbyCode: string }>();

	const phase = $derived(gameStore.state.phase);
</script>

<!-- Players see different views based on game phase -->
{#if phase === GamePhase.Lobby}
	<LobbyView />
{:else if phase === GamePhase.Question}
	<QuestionView />
{:else if phase === GamePhase.Score}
	<ScoreView />
{:else if phase === GamePhase.GameOver}
	<GameOver />
{/if}
