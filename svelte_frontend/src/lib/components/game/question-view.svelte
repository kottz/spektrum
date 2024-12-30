<script lang="ts">
	import { gameStore } from '../../stores/game';
	import { gameActions } from '../../stores/game-actions';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import AnswerProgress from './answer-progress.svelte';
	import RoundTimer from './round-timer.svelte';

	const alternatives = $derived($gameStore.currentQuestion?.alternatives || []);
	const questionType = $derived($gameStore.currentQuestion?.type || 'default');
	const currentPlayer = $derived(
		$gameStore.playerName ? $gameStore.players.get($gameStore.playerName) : undefined
	);
	const hasAnswered = $derived(currentPlayer?.hasAnswered || false);
	const selectedAnswer = $derived(currentPlayer?.answer);
	const currentAnswers = $derived($gameStore.currentAnswers);

	const myAnswer = $derived(currentAnswers.find((a) => a.name === $gameStore.playerName));
	const wasCorrect = $derived(myAnswer?.correct);

	let clickedAnswer = $state<string | null>(null);

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
			clickedAnswer = answer;
			gameActions.submitAnswer(answer);
		}
	}

	function getButtonStyles(alternative: string) {
		const styles = [];

		styles.push('aspect-square', 'rounded-lg', 'transition-all', 'duration-150', 'relative');

		if (alternative === clickedAnswer) {
			styles.push('ring-4', 'ring-offset-2', 'z-10');

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

		if (clickedAnswer && alternative !== clickedAnswer) {
			styles.push('opacity-40');
		}

		if (!clickedAnswer) {
			styles.push('hover:ring-2', 'hover:ring-muted-foreground', 'hover:scale-[1.02]');
		}

		if (questionType === 'character') {
			styles.push('p-0', 'overflow-hidden');
		} else if (questionType !== 'color') {
			styles.push('bg-muted');
		}

		styles.push(clickedAnswer ? 'cursor-not-allowed' : 'cursor-pointer');

		return styles.join(' ');
	}
</script>

<div class="container mx-auto max-w-2xl p-4">
	<Card class="mb-4">
		<CardContent class="p-4">
			<AnswerProgress />
		</CardContent>
	</Card>

	<Card>
		<CardContent class="p-4">
			<RoundTimer class="mb-4" />
		</CardContent>
	</Card>
	<Card>
		<CardHeader>
			<CardTitle class="flex items-center justify-between">
				<span>Choose your answer</span>
			</CardTitle>
		</CardHeader>
		<CardContent>
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
