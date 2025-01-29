<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import { gameActions } from '../../stores/game-actions';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import AnswerProgress from './answer-progress.svelte';
	import RoundTimer from './round-timer.svelte';
	import { PUBLIC_SPEKTRUM_CDN_URL } from '$env/static/public';
	import { timerStore } from '$lib/stores/timer-store.svelte';

	const imageBaseUrl = $derived(`${PUBLIC_SPEKTRUM_CDN_URL}/img`);

	// Access state directly through the new store structure
	const alternatives = $derived(gameStore.state.currentQuestion?.alternatives || []);
	const questionType = $derived(gameStore.state.currentQuestion?.type || 'default');

	// Get current player info
	const currentPlayer = $derived(
		gameStore.state.playerName ? gameStore.state.players.get(gameStore.state.playerName) : undefined
	);

	const hasAnswered = $derived(currentPlayer?.hasAnswered || false);
	const selectedAnswer = $derived(currentPlayer?.answer);
	const currentAnswers = $derived(gameStore.state.currentAnswers);

	// Determine if player's answer was correct
	const myAnswer = $derived(currentAnswers.find((a) => a.name === gameStore.state.playerName));
	const wasCorrect = $derived(myAnswer?.correct);

	let clickedAnswer = $state<string | null>(null);

	// Color map remains unchanged
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
		brown: '#964B00',
		orange: '#FFA500',
		gray: '#808080'
	};

	function handleAnswer(answer: string) {
		if (!hasAnswered) {
			clickedAnswer = answer;
			timerStore.stopTimer();
			gameActions.submitAnswer(answer);
		}
	}

	function getButtonStyles(alternative: string) {
		const styles: string[] = [];
		styles.push('aspect-square', 'rounded-lg', 'transition-all', 'duration-150', 'relative');

		if (questionType === 'character') {
			styles.push('p-0', 'overflow-hidden');
			if (!(alternative === clickedAnswer && myAnswer)) {
				styles.push('bg-gray-200');
			}
		} else if (questionType !== 'color') {
			styles.push('bg-muted');
		}

		if (alternative === clickedAnswer) {
			styles.push('ring-4', 'z-10');
			if (myAnswer) {
				if (wasCorrect) {
					styles.push('ring-green-500', 'bg-green-500/50');
				} else {
					styles.push('ring-red-500', 'bg-red-500/50');
				}
			} else {
				styles.push('ring-primary');
			}
		}

		if (clickedAnswer && alternative !== clickedAnswer) {
			styles.push('opacity-40');
		}

		if (!clickedAnswer) {
			styles.push('hover:ring-2', 'hover:ring-muted-foreground', 'hover:scale-[1.02]');
		}

		if (questionType === 'color') {
			const lower = alternative.toLowerCase();
			if (lower === 'white') {
				styles.push('border-2', 'border-black');
			}
			if (lower === 'gold') {
				styles.push('metallic-gold');
			} else if (lower === 'silver') {
				styles.push('metallic-silver');
			}
		}

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
	<Card class="mb-4">
		<CardContent class="p-4">
			<RoundTimer />
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
								{#if alternative.endsWith('_video')}
									<video
										src={`${imageBaseUrl}/${alternative}.webm`}
										class="h-full w-full object-contain"
										autoplay
										loop
										muted
										playsinline
									></video>
								{:else}
									<img
										src={`${imageBaseUrl}/${alternative}.avif`}
										alt={alternative}
										class="h-full w-full object-contain"
									/>
								{/if}
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
