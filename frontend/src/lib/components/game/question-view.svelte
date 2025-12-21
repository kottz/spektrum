<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import { gameActions } from '../../stores/game-actions';
	import { Card, CardContent } from '$lib/components/ui/card';
	import AnswerProgress from './answer-progress.svelte';
	import RoundTimer from './round-timer.svelte';
	import { PUBLIC_SPEKTRUM_CDN_URL } from '$env/static/public';
	import { timerStore } from '$lib/stores/timer-store.svelte';

	const imageBaseUrl = $derived(`${PUBLIC_SPEKTRUM_CDN_URL}/img`);

	const alternatives = $derived(gameStore.state.currentQuestion?.alternatives || []);
	const questionType = $derived(gameStore.state.currentQuestion?.type || 'default');
	const questionText = $derived(gameStore.state.currentQuestion?.text ?? '');

	const currentPlayer = $derived(
		gameStore.state.playerName ? gameStore.state.players.get(gameStore.state.playerName) : undefined
	);

	const hasAnswered = $derived(currentPlayer?.hasAnswered || false);
	const currentAnswers = $derived(gameStore.state.currentAnswers);

	const myAnswer = $derived(currentAnswers.find((a) => a.name === gameStore.state.playerName));
	const wasCorrect = $derived(myAnswer?.score ? myAnswer.score > 0 : false);

	const currentQuestionKey = $derived(() => {
		const question = gameStore.state.currentQuestion;
		if (!question) return null;
		return `${question.type}|${question.text ?? ''}|${question.alternatives.join('|')}`;
	});

	let clickedAnswer = $state<{ key: string | null; answer: string } | null>(null);
	let activeClickedAnswer = $derived(
		clickedAnswer?.key === currentQuestionKey() ? clickedAnswer.answer : null
	);

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
			clickedAnswer = { key: currentQuestionKey(), answer };
			timerStore.stopTimer();
			gameActions.submitAnswer(answer);
		}
	}

	function preventContextMenu(event: MouseEvent) {
		event.preventDefault();
	}

	function getButtonStyles(alternative: string) {
		const styles: string[] = [];
		styles.push('rounded-lg', 'transition-all', 'duration-150', 'relative');

		// Type-specific layout and base appearance
		if (questionType === 'character') {
			styles.push(
				'aspect-square',
				'p-0',
				'overflow-hidden',
				'flex',
				'items-center',
				'justify-center'
			);
			if (!(alternative === clickedAnswer && myAnswer)) {
				styles.push('bg-gray-200', 'dark:bg-gray-800');
			}
		} else if (questionType === 'color') {
			styles.push('aspect-square'); // Color buttons are squares, bg handled by style/later classes
		} else if (questionType === 'text') {
			styles.push('py-3', 'px-4', 'bg-muted', 'flex', 'items-center', 'justify-center', 'h-full'); // Text buttons are rectangular, fill height
		} else {
			// Fallback (matches original 'default' type style if needed)
			styles.push('aspect-square', 'bg-muted', 'flex', 'items-center', 'justify-center');
		}

		// --- Interaction and Feedback Styles (Common Logic) ---
		if (alternative === activeClickedAnswer) {
			styles.push('ring-4', 'z-10');
			if (myAnswer) {
				// Feedback after answer revealed
				if (wasCorrect) {
					styles.push('ring-green-500');
					// Apply original background feedback for character type
					if (questionType === 'character') {
						styles.push('bg-green-500/50');
					} else if (questionType === 'text') {
						styles.push('!bg-green-500/20'); // Subtle tint for text
					}
				} else {
					styles.push('ring-red-500');
					// Apply original background feedback for character type
					if (questionType === 'character') {
						styles.push('bg-red-500/50');
					} else if (questionType === 'text') {
						styles.push('bg-red-500/20'); // Subtle tint for text
					}
				}
			} else {
				// Ring while waiting for result
				styles.push('ring-primary');
			}
		}

		if (activeClickedAnswer && alternative !== activeClickedAnswer) {
			styles.push('opacity-40'); // Fade out unselected alternatives
		}

		if (!activeClickedAnswer) {
			// Hover effects only when clickable
			styles.push('hover:ring-2', 'hover:ring-muted-foreground');
			if (questionType === 'character' || questionType === 'color') {
				styles.push('hover:scale-[1.02]');
			} else if (questionType === 'text') {
				styles.push('hover:brightness-95', 'dark:hover:brightness-110'); // Subtle brightness change for text
			} else {
				styles.push('hover:scale-[1.02]'); // Default hover
			}
		}

		// --- Color Specific Styling ---
		if (questionType === 'color') {
			const lower = alternative.toLowerCase();
			if (lower === 'white') {
				styles.push('border-2', 'border-black');
			}
			if (lower === 'black') {
				styles.push('border-2', 'dark:border-white');
			}
			// Assuming metallic classes exist in global CSS
			if (lower === 'gold') {
				styles.push('metallic-gold');
			} else if (lower === 'silver') {
				styles.push('metallic-silver');
			}
			// Background color for standard colors is handled by inline style attribute
		}

		// Cursor state
		styles.push(activeClickedAnswer ? 'cursor-not-allowed' : 'cursor-pointer');

		return styles.join(' ');
	}
</script>

<div class="mx-auto max-w-2xl p-4">
	<Card class="mb-4">
		<CardContent class="p-4">
			<AnswerProgress />
		</CardContent>
	</Card>

	<Card class="mb-4">
		<CardContent class="p-4">
			<RoundTimer />
		</CardContent>
	</Card>

	<Card>
		<CardContent class="p-4">
			{#if questionType === 'text'}
				<p class="mb-4 text-center text-xl font-semibold">{questionText}</p>
			{/if}

			<div class="grid gap-2 {questionType === 'text' ? 'grid-cols-2' : 'grid-cols-3'}">
				{#each alternatives as alternative}
					<button
						class={getButtonStyles(alternative)}
						disabled={hasAnswered || activeClickedAnswer !== null}
						onclick={() => handleAnswer(alternative)}
						style={questionType === 'color' &&
						!(alternative.toLowerCase() === 'gold' || alternative.toLowerCase() === 'silver')
							? `background-color: ${colorMap[alternative.toLowerCase()] || '#000000'};`
							: ''}
					>
						{#if questionType === 'character'}
							<div class="aspect-square w-full">
								{#if alternative.endsWith('_video')}
									<video
										src={`${imageBaseUrl}/${alternative}.webm`}
										class="h-full w-full object-contain"
										autoplay
										loop
										muted
										playsinline
										oncontextmenu={preventContextMenu}
									></video>
								{:else}
									<img
										src={`${imageBaseUrl}/${alternative}.avif`}
										alt={alternative}
										class="h-full w-full object-contain text-transparent"
										oncontextmenu={preventContextMenu}
									/>
								{/if}
							</div>
						{:else if questionType === 'color'}
							<span class="sr-only">{alternative}</span>
						{:else if questionType === 'text'}
							<span class="text-center text-lg font-medium">{alternative}</span>
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
