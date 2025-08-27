<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import { Card, CardContent } from '$lib/components/ui/card';
	import { PUBLIC_SPEKTRUM_CDN_URL } from '$env/static/public';

	const imageBaseUrl = $derived(`${PUBLIC_SPEKTRUM_CDN_URL}/img`);
	const alternatives = $derived(gameStore.state.currentQuestion?.alternatives || []);
	const questionType = $derived(gameStore.state.currentQuestion?.type || 'default');
	const upcomingQuestions = $derived(gameStore.state.upcomingQuestions || []);

	const currentQuestionText = $derived(upcomingQuestions[0]?.question_text);

	// Create placeholder array of 6 items (for the grid layout)
	const placeholderCount = 6;

	const correctAnswers = $derived(
		upcomingQuestions[0]?.options
			.filter((opt) => opt.is_correct)
			.map((opt) => opt.option)
			.filter(Boolean) || []
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

	function getButtonStyles(alternative: string) {
		const isCorrect = correctAnswers.includes(alternative);
		const styles = [
			'rounded-lg',
			'transition-all',
			'duration-150',
			'relative',
			'pointer-events-none'
		];

		// Type-specific base styles and layout hints
		if (questionType === 'character') {
			styles.push(
				'aspect-square',
				'p-0',
				'overflow-hidden',
				'bg-gray-200',
				'dark:bg-gray-800',
				'flex',
				'items-center',
				'justify-center'
			);
		} else if (questionType === 'color') {
			styles.push('aspect-square');
		} else if (questionType === 'text') {
			styles.push('py-2', 'px-4', 'bg-muted', 'flex', 'items-center', 'justify-center'); // Rectangular text button
		} else {
			// Default/Other type
			styles.push('aspect-square', 'bg-muted', 'flex', 'items-center', 'justify-center');
		}

		// Correct answer styling
		if (isCorrect) {
			styles.push('ring-4', 'ring-green-500', 'z-10');
			if (questionType === 'character' || questionType === 'default') {
				styles.push('bg-green-500/50');
			} else if (questionType === 'text') {
				styles.push('bg-green-500/20'); // Subtle green background for text
			}
			// Color type gets ring only, background is set by color itself or inline style
		}

		// Color specific borders/metallic styles (only if type is color)
		if (questionType === 'color') {
			const lower = alternative.toLowerCase();
			if (lower === 'white') {
				styles.push('border-2', 'border-black');
			}
			if (lower === 'black') {
				styles.push('border-2', 'dark:border-white');
			}
			if (lower === 'gold') {
				styles.push('metallic-gold');
			} else if (lower === 'silver') {
				styles.push('metallic-silver');
			}
		}

		return styles.join(' ');
	}

	function getPlaceholderStyles() {
		return ['aspect-square', 'rounded-lg', 'bg-muted/50', 'animate-pulse'].join(' ');
	}
</script>

<Card>
	<CardContent class="p-2">
		{#if questionType === 'text' && currentQuestionText}
			<p class="mb-3 text-center text-base font-semibold">{currentQuestionText}</p>
		{/if}

		{#if alternatives.length > 0}
			{#if questionType === 'text'}
				<div class="flex flex-wrap justify-center gap-2">
					{#each alternatives as alternative}
						<button class={getButtonStyles(alternative)} disabled={true}>
							<span class="text-center text-sm font-medium">{alternative}</span>
						</button>
					{/each}
				</div>
			{:else}
				<div class="grid grid-cols-6 gap-2">
					{#each alternatives as alternative}
						<button
							class={getButtonStyles(alternative)}
							disabled={true}
							style={questionType === 'color' &&
							!(alternative.toLowerCase() === 'gold' || alternative.toLowerCase() === 'silver')
								? `background-color: ${colorMap[alternative.toLowerCase()]};`
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
										></video>
									{:else}
										<img
											src={`${imageBaseUrl}/${alternative}.avif`}
											alt={alternative}
											class="h-full w-full object-contain text-transparent"
										/>
									{/if}
								</div>
							{:else if questionType === 'color'}
								<span class="sr-only">{alternative}</span>
							{:else}
								<div class="flex h-full w-full items-center justify-center text-sm font-medium">
									{alternative}
								</div>
							{/if}
						</button>
					{/each}
				</div>
			{/if}
		{:else if questionType !== 'text'}
			<!-- Show placeholders only for non-text grid layouts when empty -->
			<div class="grid grid-cols-6 gap-2">
				{#each Array(placeholderCount) as _}
					<div class={getPlaceholderStyles()}></div>
				{/each}
			</div>
		{/if}
		<!-- No placeholders needed for 'text' type when empty, the flex container will just be empty -->
	</CardContent>
</Card>
