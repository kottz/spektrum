<script lang="ts">
	import { gameStore } from '../../stores/game';
	import { gameActions } from '../../stores/game-actions';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import AnswerProgress from './answer-progress.svelte';

	// Get current game state
	$: alternatives = $gameStore.currentQuestion?.alternatives || [];
	$: currentPlayer = $gameStore.playerName
		? $gameStore.players.get($gameStore.playerName)
		: undefined;
	$: hasAnswered = currentPlayer?.hasAnswered || false;
	$: selectedAnswer = currentPlayer?.answer;
	$: timeRemaining = $gameStore.roundDuration;

	function handleAnswer(answer: string) {
		if (!hasAnswered) {
			gameActions.submitAnswer(answer);
		}
	}

	function getButtonStyles(alternative: string) {
		if (hasAnswered) {
			if (alternative === selectedAnswer) {
				return 'bg-primary/20 border-primary border-2';
			}
			return 'bg-muted opacity-50';
		}
		return 'bg-muted hover:bg-muted/80 hover:border-border border-2 border-transparent';
	}
</script>

<div class="container mx-auto max-w-2xl space-y-6 p-6">
	<!-- Answer Progress -->
	<Card>
		<CardContent class="p-4">
			<AnswerProgress />
		</CardContent>
	</Card>

	<!-- Answer Options -->
	<Card>
		<CardHeader>
			<CardTitle class="flex items-center justify-between">
				<span>Choose your answer</span>
				<span class="text-muted-foreground">{timeRemaining}s</span>
			</CardTitle>
		</CardHeader>
		<CardContent>
			<div class="grid grid-cols-2 gap-4">
				{#each alternatives as alternative}
					<button
						class="aspect-square rounded-lg transition-all duration-200 {getButtonStyles(
							alternative
						)}"
						disabled={hasAnswered}
						on:click={() => handleAnswer(alternative)}
					>
						<div class="flex h-full w-full items-center justify-center text-lg font-medium">
							{alternative}
						</div>
					</button>
				{/each}
			</div>
		</CardContent>
	</Card>
</div>
