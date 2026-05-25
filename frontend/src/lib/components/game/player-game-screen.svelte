<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import { gameActions } from '../../stores/game-actions';
	import QuestionView from './question-view.svelte';
	import ScoreView from './score-view.svelte';
	import LobbyView from './lobby-view.svelte';
	import GameOver from './game-over.svelte';
	import { GamePhase } from '../../types/game';
	import { Button } from '$lib/components/ui/button';
	import LightSwitch from '$lib/components/ui/light-switch.svelte';
	const phase = $derived(gameStore.state.phase);
</script>

<div class="flex h-dvh flex-col">
	<!-- Top bar with leave button and light switch -->
	<div class="flex flex-none items-center justify-end p-4">
		<div class="flex gap-4">
			<Button variant="outline" onclick={() => gameActions.leaveGame()}>Leave Game</Button>
			<LightSwitch />
		</div>
	</div>

	<!-- Game content based on phase - now flex-1 with min-h-0 -->
	<div class="min-h-0 flex-1">
		{#if phase === GamePhase.Lobby}
			<LobbyView />
		{:else if phase === GamePhase.Question}
			<QuestionView />
		{:else if phase === GamePhase.Score}
			<ScoreView />
		{:else if phase === GamePhase.GameOver}
			<GameOver />
		{/if}
	</div>
</div>
