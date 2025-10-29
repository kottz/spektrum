<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import { gameActions } from '../../stores/game-actions';
	import QuestionView from './question-view.svelte';
	import ScoreView from './score-view.svelte';
	import LobbyView from './lobby-view.svelte';
	import GameOver from './game-over.svelte';
	import { GamePhase } from '../../types/game';
	import { Card } from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import LightSwitch from '$lib/components/ui/light-switch.svelte';
	const phase = $derived(gameStore.state.phase);
	const joinCode = $derived(gameStore.state.joinCode);
</script>

<div class="flex h-dvh flex-col">
	<!-- Top bar with join code and leave button - now flex-none -->
	<div class="flex flex-none items-center justify-between p-4">
		{#if joinCode}
			<Card>
				<div class="flex items-center gap-2 p-4">
					<span class="text-muted-foreground">Join Code:</span>
					<span class="font-mono text-lg font-bold">{joinCode}</span>
				</div>
			</Card>
		{/if}
		<div class="flex gap-4">
			<Button variant="outline" on:click={() => gameActions.leaveGame()}>Leave Game</Button>
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
