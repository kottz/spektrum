<script lang="ts">
	import { gameStore } from '../../stores/game';
	import { gameActions } from '../../stores/game-actions';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import AnswerProgress from './answer-progress.svelte';
	import RoundTimer from './round-timer.svelte';

	// Get current game state
	$: alternatives = $gameStore.currentQuestion?.alternatives || [];
	$: questionType = $gameStore.currentQuestion?.type || 'default';
	$: currentPlayer = $gameStore.playerName
		? $gameStore.players.get($gameStore.playerName)
		: undefined;
	$: hasAnswered = currentPlayer?.hasAnswered || false;
	$: selectedAnswer = currentPlayer?.answer;
	$: timeRemaining = $gameStore.roundDuration;
	$: currentAnswers = $gameStore.currentAnswers;

	// Get if answer was correct
	$: myAnswer = currentAnswers.find((a) => a.name === $gameStore.playerName);
	$: wasCorrect = myAnswer?.correct;

	// Track which answer was clicked (separate from server state)
	let clickedAnswer: string | null = null;

	// Color mapping with explicit type
	const colorMap: Record<string, string> = {
		red: '#FF0000',
		green: '#00FF00',
		blue: '#0000FF',
		yellow: '#FFFF00',
		purple: '#800080',
		gold: '#FFD700',
		silver: '#C0C0C0',
		pink: '#FFC0CB',
		black: '#000000',
		white: '#FFFFFF',
		brown: '#3D251E',
		orange: '#FFA500',
		gray: '#808080'
	};

	function handleAnswer(answer: string) {
		if (!hasAnswered) {
			clickedAnswer = answer; // Track local state immediately
			gameActions.submitAnswer(answer);
		}
	}

	function getButtonStyles(alternative: string) {
		const styles = [];

		styles.push('aspect-square', 'rounded-lg', 'transition-all', 'duration-150', 'relative');

		// Add a strong border and background to the selected answer
		if (alternative === clickedAnswer) {
			// Added faster animation for the pulse effect
			styles.push('ring-4', 'ring-offset-2', 'z-10'); //, 'animate-[pulse_1s_ease-in-out]');

			if (myAnswer) {
				if (wasCorrect) {
					styles.push('ring-green-500', 'bg-green-500/20');
				} else {
					styles.push('ring-red-500', 'bg-red-500/20');
				}
			} else {
				styles.push('ring-primary');
			}
		}

		// If any answer is clicked, reduce opacity of non-selected answers
		if (clickedAnswer && alternative !== clickedAnswer) {
			styles.push('opacity-40');
		}

		// Hover state only before answer is selected
		if (!clickedAnswer) {
			// Made the scale effect smaller and faster
			styles.push('hover:ring-2', 'hover:ring-muted-foreground', 'hover:scale-[1.02]');
		}

		// Question type specific styles
		if (questionType === 'character') {
			styles.push('p-0', 'overflow-hidden');
		} else if (questionType !== 'color') {
			styles.push('bg-muted');
		}

		// Cursor styles
		styles.push(clickedAnswer ? 'cursor-not-allowed' : 'cursor-pointer');

		return styles.join(' ');
	}
</script>

<!-- Game content -->
<div class="container mx-auto max-w-2xl p-4">
	<!-- Answer Progress -->
	<Card class="mb-4">
		<CardContent class="p-4">
			<AnswerProgress />
		</CardContent>
	</Card>

	<RoundTimer class="mb-4" />

	<!-- Answer Options -->
	<Card>
		<CardHeader>
			<CardTitle class="flex items-center justify-between">
				<span>Choose your answer</span>
				<span class="text-muted-foreground">{timeRemaining}s</span>
			</CardTitle>
		</CardHeader>
		<CardContent>
			<!-- Changed from grid-cols-2 to grid-cols-3 -->
			<div class="grid grid-cols-3 gap-2">
				{#each alternatives as alternative}
					<button
						class={getButtonStyles(alternative)}
						disabled={clickedAnswer !== null}
						on:click={() => handleAnswer(alternative)}
						style={questionType === 'color'
							? `background-color: ${colorMap[alternative.toLowerCase()]};`
							: ''}
					>
						{#if questionType === 'character'}
							<div class="aspect-square w-full">
								<img
									src={`http://192.168.1.155:8765/img_avif/${alternative}.avif`}
									alt={alternative}
									class="h-full w-full object-contain"
								/>
							</div>
						{:else if questionType === 'color'}
							<span class="sr-only">{alternative}</span>
						{:else}
							<div class="flex h-full w-full items-center justify-center text-lg font-medium">
								{alternative}
							</div>
						{/if}
					</button>
				{/each}
			</div>
		</CardContent>
	</Card>
</div>
