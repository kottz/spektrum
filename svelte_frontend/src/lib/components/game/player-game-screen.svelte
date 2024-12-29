<script lang="ts">
	import { gameStore } from '../../stores/game';
	import { gameActions } from '../../stores/game-actions';
	import QuestionView from './question-view.svelte';
	import ScoreView from './score-view.svelte';
	import LobbyView from './lobby-view.svelte';
	import GameOver from './game-over.svelte';
	import { GamePhase } from '../../types/game';
	import { Card } from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';

	$: phase = $gameStore.phase;
	$: joinCode = $gameStore.joinCode;
</script>

<div class="container mx-auto space-y-6 p-6">
	<!-- Top bar with join code and leave button -->
	<div class="flex items-center justify-between">
		{#if joinCode}
			<Card>
				<div class="flex items-center gap-2 p-4">
					<span class="text-muted-foreground">Join Code:</span>
					<span class="font-mono text-lg font-bold">{joinCode}</span>
				</div>
			</Card>
		{/if}
		<Button variant="outline" on:click={() => gameActions.leaveGame()}>Leave Game</Button>
	</div>

	<!-- Game content based on phase -->
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
