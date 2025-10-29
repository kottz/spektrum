<script lang="ts">
	import { Card, CardContent } from '$lib/components/ui/card';
	import { PUBLIC_SPEKTRUM_CDN_URL } from '$env/static/public';

	const imageBaseUrl = `${PUBLIC_SPEKTRUM_CDN_URL}/img`;

	let clickedAnswer = $state<string | null>(null);
	let showResult = $state(false);

	const questions = [
		{
			text: 'Californication - Red Hot Chili Peppers',
			correctAnswer: 'red',
			alternatives: ['blue', 'red', 'green', 'yellow', 'purple', 'orange'],
			type: 'color'
		},
		{
			text: 'Yellow - Coldplay',
			correctAnswer: 'yellow',
			alternatives: ['red', 'blue', 'green', 'purple', 'yellow', 'orange'],
			type: 'color'
		},
		{
			text: 'Purple Rain - Prince',
			correctAnswer: 'purple',
			alternatives: ['red', 'purple', 'blue', 'yellow', 'green', 'orange'],
			type: 'color'
		},
		{
			text: 'Back to Black - Amy Winehouse',
			correctAnswer: 'black',
			alternatives: ['red', 'blue', 'green', 'orange', 'purple', 'black'],
			type: 'color'
		},
		{
			text: 'Here Comes The Sun - The Beatles',
			correctAnswer: 'sun',
			alternatives: ['wind', 'smoke', 'sun', 'snow', 'lightning', 'ice'],
			type: 'character'
		},
		{
			text: 'I Saw Her Standing There - The Beatles',
			correctAnswer: 'saw',
			alternatives: ['wrench', 'whisk', 'sledgehammer', 'saw', 'hammer', 'spoon'],
			type: 'character'
		}
	];

	const colorQuestions = questions.filter((q) => q.type === 'color');
	const characterQuestions = questions.filter((q) => q.type === 'character');

	let currentType = $state<'color' | 'character'>(Math.random() < 0.5 ? 'color' : 'character');
	let colorIndex = $state(Math.floor(Math.random() * colorQuestions.length));
	let characterIndex = $state(Math.floor(Math.random() * characterQuestions.length));

	let currentQuestion = $derived(
		currentType === 'color' ? colorQuestions[colorIndex] : characterQuestions[characterIndex]
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
		if (!clickedAnswer) {
			clickedAnswer = answer;
			showResult = true;
		}
	}

	function resetExample() {
		clickedAnswer = null;
		showResult = false;

		// Alternate between color and character questions
		currentType = currentType === 'color' ? 'character' : 'color';

		if (currentType === 'color') {
			colorIndex = (colorIndex + 1) % colorQuestions.length;
		} else {
			characterIndex = (characterIndex + 1) % characterQuestions.length;
		}
	}

	function getButtonStyles(alternative: string) {
		const styles: string[] = [];
		styles.push('rounded-lg', 'transition-all', 'duration-150', 'relative', 'aspect-square');

		// Type-specific layout and base appearance
		if (currentQuestion.type === 'character') {
			styles.push('p-0', 'overflow-hidden', 'flex', 'items-center', 'justify-center');
			if (!(alternative === clickedAnswer && showResult)) {
				styles.push('bg-gray-200', 'dark:bg-gray-800');
			}
		} else {
			// Color question styling
			const lower = alternative.toLowerCase();
			if (lower === 'white') {
				styles.push('border-2', 'border-black');
			}
			if (lower === 'black') {
				styles.push('border-2', 'dark:border-white');
			}
		}

		if (alternative === clickedAnswer) {
			styles.push('ring-4', 'z-10');
			if (showResult) {
				if (alternative === currentQuestion.correctAnswer) {
					styles.push('ring-green-500');
					// Apply original background feedback for character type
					if (currentQuestion.type === 'character') {
						styles.push('bg-green-500/50');
					}
				} else {
					styles.push('ring-red-500');
					// Apply original background feedback for character type
					if (currentQuestion.type === 'character') {
						styles.push('bg-red-500/50');
					}
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

		styles.push(clickedAnswer ? 'cursor-not-allowed' : 'cursor-pointer');

		return styles.join(' ');
	}
</script>

<Card>
	<CardContent>
		<div class="space-y-4">
			<div class="text-center">
				<p class="text-muted-foreground mb-2 text-sm">Imagine this song is playing</p>
				<p class="mb-2 text-lg font-semibold">{currentQuestion.text}</p>
				<p class="text-muted-foreground text-sm">
					then which {currentQuestion.type === 'character' ? 'image' : 'color'} should you click?
				</p>
			</div>

			<div class="grid grid-cols-3 gap-2">
				{#each currentQuestion.alternatives as alternative}
					<button
						class={getButtonStyles(alternative)}
						disabled={clickedAnswer !== null}
						onclick={() => handleAnswer(alternative)}
						style={currentQuestion.type === 'color'
							? `background-color: ${colorMap[alternative.toLowerCase()] || '#000000'};`
							: ''}
					>
						{#if currentQuestion.type === 'character'}
							<div class="aspect-square w-full">
								<img
									src={`${imageBaseUrl}/${alternative}.avif`}
									alt={alternative}
									class="h-full w-full object-contain text-transparent"
								/>
							</div>
						{:else}
							<span class="sr-only">{alternative}</span>
						{/if}
					</button>
				{/each}
			</div>

			{#if showResult}
				<div class="space-y-2 text-center">
					{#if clickedAnswer === currentQuestion.correctAnswer}
						<p class="font-semibold text-green-600 dark:text-green-400">Correct! 🎉</p>
						<p class="text-muted-foreground text-sm">Fast answers score more points!</p>
					{:else}
						<p class="font-semibold text-red-600 dark:text-red-400">Wrong answer</p>
						<p class="text-muted-foreground text-sm">
							The correct answer was {currentQuestion.correctAnswer}
						</p>
					{/if}
					<button
						class="bg-primary text-primary-foreground hover:bg-primary/90 mt-2 rounded px-3 py-1 text-sm"
						onclick={resetExample}
					>
						Try Again
					</button>
				</div>
			{/if}
		</div>
	</CardContent>
</Card>
