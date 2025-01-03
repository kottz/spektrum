<script lang="ts">
	import { gameStore } from '../../stores/game';
	import { gameActions } from '../../stores/game-actions';
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Separator } from '$lib/components/ui/separator';
	import UpcomingQuestions from '$lib/components/game/upcoming-questions.svelte';
	import AnswerProgress from '$lib/components/game/answer-progress.svelte';
	import RoundTimer from './round-timer.svelte';

	// Reactive values from store
	const phase = $derived($gameStore.phase?.toLowerCase() || 'lobby');
	const players = $derived(Array.from($gameStore.players.values()));
	const playerCount = $derived(players.length);

	// Game state checks
	const isGameRunning = $derived(phase !== 'lobby' && phase !== 'gameover');
	const isInQuestion = $derived(phase === 'question');
	const isGameOver = $derived(phase === 'gameover');
</script>

<Card>
	<CardHeader>
		<CardTitle>Admin Controls</CardTitle>
	</CardHeader>
	<CardContent class="space-y-6">
		<!-- Control buttons at the top -->
		<div class="flex flex-col gap-2">
			<!-- Leave/Close button always first -->
			<Button
				variant="destructive"
				on:click={() => (isGameOver ? gameActions.leaveGame() : gameActions.closeGame())}
			>
				{isGameOver ? 'Leave Lobby' : 'Close Lobby'}
			</Button>

			<!-- Start/End Game button -->
			{#if !isGameOver}
				<Button
					variant={isGameRunning ? 'destructive' : 'default'}
					on:click={() => (isGameRunning ? gameActions.endGame() : gameActions.startGame())}
				>
					{isGameRunning ? 'End Game' : 'Start Game'}
				</Button>
			{/if}
			<Separator class="my-4" />
			<!-- Start/End Round button -->
			{#if isGameRunning}
				<Button on:click={() => (isInQuestion ? gameActions.endRound() : gameActions.startRound())}>
					{isInQuestion ? 'End Round' : 'Start Round'}
				</Button>
			{/if}

			<!-- Skip Question button - always visible but conditionally disabled -->
			<Button
				variant="outline"
				disabled={isInQuestion || !isGameRunning}
				on:click={() => gameActions.skipQuestion()}
			>
				Skip Question
			</Button>
		</div>

		<!-- Current game status -->
		<div class="space-y-2">
			<div class="flex justify-between text-sm">
				<span class="text-muted-foreground">Current Phase</span>
				<span class="font-medium">{phase}</span>
			</div>

			<!-- Players list -->
			<div class="space-y-2">
				<div class="flex justify-between text-sm">
					<span class="text-muted-foreground">Players ({playerCount})</span>
				</div>
				<div class="space-y-1">
					{#each players as player}
						<div class="flex items-center justify-between rounded bg-muted p-2 text-sm">
							<span>{player.name}</span>
							<span class="text-muted-foreground">{player.score}</span>
						</div>
					{/each}
				</div>
			</div>
		</div>

		{#if !isGameOver}
			<RoundTimer />
			<AnswerProgress />
			<div class="border-t border-border pt-4">
				<UpcomingQuestions />
			</div>
		{/if}
	</CardContent>
</Card>
