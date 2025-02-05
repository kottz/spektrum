<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import { Card, CardContent } from '$lib/components/ui/card';
	import { PUBLIC_SPEKTRUM_CDN_URL } from '$env/static/public';

	const imageBaseUrl = $derived(`${PUBLIC_SPEKTRUM_CDN_URL}/img`);
	const alternatives = $derived(gameStore.state.currentQuestion?.alternatives || []);
	const questionType = $derived(gameStore.state.currentQuestion?.type || 'default');
	const upcomingQuestions = $derived(gameStore.state.upcomingQuestions || []);

	// Create placeholder array of 6 items (1 rows of 6 in the grid)
	const placeholderCount = 6;

	const correctAnswers = $derived(
		questionType === 'character'
			? [upcomingQuestions[0]?.options.find((opt) => opt.is_correct)?.option].filter(Boolean)
			: upcomingQuestions[0]?.options
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
		const styles = [
			'aspect-square',
			'rounded-lg',
			'transition-all',
			'duration-150',
			'relative',
			'pointer-events-none'
		];
		if (questionType === 'character') {
			styles.push('p-0', 'overflow-hidden', 'bg-gray-200');
		} else if (questionType !== 'color') {
			styles.push('bg-muted');
		}
		if (correctAnswers.includes(alternative)) {
			styles.push('ring-4', 'ring-green-500', 'bg-green-500/50', 'z-10');
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
		return styles.join(' ');
	}

	function getPlaceholderStyles() {
		return ['aspect-square', 'rounded-lg', 'bg-muted/50', 'animate-pulse'].join(' ');
	}
</script>

<Card>
	<CardContent class="p-2">
		<div class="grid grid-cols-6 gap-2">
			{#if alternatives.length > 0}
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
								<img
									src={`${imageBaseUrl}/${alternative}.avif`}
									alt={alternative}
									class="h-full w-full object-contain"
								/>
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
			{:else}
				{#each Array(placeholderCount) as _}
					<div class={getPlaceholderStyles()}></div>
				{/each}
			{/if}
		</div>
	</CardContent>
</Card>
