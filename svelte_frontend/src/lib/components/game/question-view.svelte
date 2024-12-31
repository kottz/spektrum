<script lang="ts">
	import { gameStore } from '../../stores/game';
	import { gameActions } from '../../stores/game-actions';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import AnswerProgress from './answer-progress.svelte';
	import RoundTimer from './round-timer.svelte';
	import { PUBLIC_SPEKTRUM_SERVER_URL } from '$env/static/public';

	const imageBaseUrl = $derived(`${PUBLIC_SPEKTRUM_SERVER_URL}/img_avif`);

	/**
	 * Subscribe to necessary parts of the game store
	 */
	const alternatives = $derived($gameStore.currentQuestion?.alternatives || []);
	const questionType = $derived($gameStore.currentQuestion?.type || 'default');
	const currentPlayer = $derived(
		$gameStore.playerName ? $gameStore.players.get($gameStore.playerName) : undefined
	);
	const hasAnswered = $derived(currentPlayer?.hasAnswered || false);
	const selectedAnswer = $derived(currentPlayer?.answer);
	const currentAnswers = $derived($gameStore.currentAnswers);

	/**
	 * Determine if this player's answer was correct.
	 */
	const myAnswer = $derived(currentAnswers.find((a) => a.name === $gameStore.playerName));
	const wasCorrect = $derived(myAnswer?.correct);

	let clickedAnswer = $state<string | null>(null);

	/**
	 * Map of color names to hex codes
	 */
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

	/**
	 * Submit an answer if the player hasn't already answered.
	 */
	function handleAnswer(answer: string) {
		if (!hasAnswered) {
			clickedAnswer = answer;
			gameActions.submitAnswer(answer);
		}
	}

	/**
	 * Build the list of CSS classes for a given alternative.
	 * This handles rings, hover effects, and metallic classes.
	 */
	function getButtonStyles(alternative: string) {
		const styles: string[] = [];
		styles.push('aspect-square', 'rounded-lg', 'transition-all', 'duration-150', 'relative');

		// If this alternative is the one the player clicked
		if (alternative === clickedAnswer) {
			styles.push('ring-4', 'ring-offset-2', 'z-10');

			// If we already have an answer result for this player
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

		// Dim the other buttons if one has already been clicked
		if (clickedAnswer && alternative !== clickedAnswer) {
			styles.push('opacity-40');
		}

		// Only show hover ring/scale if no button has been clicked yet
		if (!clickedAnswer) {
			styles.push('hover:ring-2', 'hover:ring-muted-foreground', 'hover:scale-[1.02]');
		}

		// If it's a character question, show an image instead
		if (questionType === 'character') {
			styles.push('p-0', 'overflow-hidden');
		} else if (questionType !== 'color') {
			// Non-color questions get a muted background
			styles.push('bg-muted');
		}

		// For color questions, add the metallic class if needed
		if (questionType === 'color') {
			const lower = alternative.toLowerCase();
			if (lower === 'gold') {
				styles.push('metallic-gold');
			} else if (lower === 'silver') {
				styles.push('metallic-silver');
			}
		}

		// Disable pointer events after clicking
		styles.push(clickedAnswer ? 'cursor-not-allowed' : 'cursor-pointer');

		return styles.join(' ');
	}
</script>

<div class="container mx-auto max-w-2xl p-4">
	<!-- Current progress in the quiz -->
	<Card class="mb-4">
		<CardContent class="p-4">
			<AnswerProgress />
		</CardContent>
	</Card>

	<!-- Round timer (time left for the player to answer) -->
	<Card>
		<CardContent class="p-4">
			<RoundTimer class="mb-4" />
		</CardContent>
	</Card>

	<!-- Main question card -->
	<Card>
		<CardHeader>
			<CardTitle class="flex items-center justify-between">
				<span>Choose your answer</span>
			</CardTitle>
		</CardHeader>
		<CardContent>
			<!-- Grid of answer buttons -->
			<div class="grid grid-cols-3 gap-2">
				{#each alternatives as alternative}
					<button
						class={getButtonStyles(alternative)}
						disabled={clickedAnswer !== null}
						onclick={() => handleAnswer(alternative)}
						style={questionType === 'color' &&
						!(alternative.toLowerCase() === 'gold' || alternative.toLowerCase() === 'silver')
							? `background-color: ${colorMap[alternative.toLowerCase()] || '#000000'};`
							: ''}
					>
						{#if questionType === 'character'}
							<!-- Character question: show an image -->
							<div class="aspect-square w-full">
								<img
									src={`${imageBaseUrl}/${alternative}.avif`}
									alt={alternative}
									class="h-full w-full object-contain"
								/>
							</div>
						{:else if questionType === 'color'}
							<!-- For color questions, label is hidden with sr-only -->
							<span class="sr-only">{alternative}</span>
						{:else}
							<!-- Default text fallback for other question types -->
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

<style>
	.container {
		/* Optional container styling */
	}
</style>
