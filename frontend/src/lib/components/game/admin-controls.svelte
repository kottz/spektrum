<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import { gameActions } from '../../stores/game-actions';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Separator } from '$lib/components/ui/separator';
	import UpcomingQuestions from '$lib/components/game/upcoming-questions.svelte';
	import AnswerProgress from '$lib/components/game/answer-progress.svelte';
	import RoundTimer from './round-timer.svelte';

	// Reactive values from store using the new state structure
	const phase = $derived(gameStore.state.phase?.toLowerCase() || 'lobby');
	const players = $derived(Array.from(gameStore.state.players.values()));
	const playerCount = $derived(players.length);
	const outOfQuestions = $derived(gameStore.state.upcomingQuestions?.length === 0);

	// Game state checks remain the same logic but use the new reactive values
	const isGameRunning = $derived(phase !== 'lobby' && phase !== 'gameover');
	const isInQuestion = $derived(phase === 'question');
	const isGameOver = $derived(phase === 'gameover');
</script>

<div class="space-y-4 p-4">
	<!-- Main Control Buttons -->
	<div class="grid grid-cols-2 gap-2">
		<Button
			variant="destructive"
			on:click={() => (isGameOver ? gameActions.leaveGame() : gameActions.closeGame())}
		>
			{isGameOver ? 'Leave Lobby' : 'Close Lobby'}
		</Button>

		{#if !isGameOver}
			<Button
				variant={isGameRunning ? 'destructive' : 'default'}
				on:click={() => (isGameRunning ? gameActions.endGame() : gameActions.startGame())}
			>
				{isGameRunning ? 'End Game' : 'Start Game'}
			</Button>
		{/if}
	</div>

	<!-- Round Controls -->
	{#if isGameRunning}
		<div class="grid grid-cols-2 gap-2">
			<Button
				disabled={outOfQuestions}
				on:click={() => (isInQuestion ? gameActions.endRound() : gameActions.startRound())}
			>
				{isInQuestion ? 'End Round' : 'Start Round'}
			</Button>

			<Button
				variant="outline"
				disabled={isInQuestion || !isGameRunning || outOfQuestions}
				on:click={() => gameActions.skipQuestion()}
			>
				Skip Question
			</Button>
		</div>
	{/if}

	<!-- Game Status -->
	<div class="flex justify-between text-sm">
		<span class="text-muted-foreground">Phase:</span>
		<span class="font-medium">{phase}</span>
	</div>

	<!-- Players List -->
	<div class="space-y-2">
		<div class="text-sm text-muted-foreground">
			Players ({playerCount})
		</div>
		<ScrollArea class="h-32">
			<div class="space-y-1">
				{#each players as player}
					<div class="flex items-center justify-between rounded-md bg-muted p-2">
						<span>{player.name}</span>
						<span class="text-muted-foreground">{player.score}</span>
					</div>
				{/each}
			</div>
		</ScrollArea>
	</div>

	<!-- Timer and Progress -->
	{#if !isGameOver}
		<RoundTimer />
		<AnswerProgress />
		<div class="border-t border-border pt-4">
			<UpcomingQuestions />
		</div>
	{/if}
</div>
