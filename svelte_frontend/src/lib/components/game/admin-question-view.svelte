<script lang="ts">
	import { gameStore } from '../../stores/game';
	import { Card, CardContent } from '$lib/components/ui/card';
	import { PUBLIC_SPEKTRUM_CDN_URL } from '$env/static/public';

	const imageBaseUrl = $derived(`${PUBLIC_SPEKTRUM_CDN_URL}/img`);

	/**
	 * Subscribe to parts of the game store
	 */
	const alternatives = $derived($gameStore.currentQuestion?.alternatives || []);
	const questionType = $derived($gameStore.currentQuestion?.type || 'default');
	const upcomingQuestions = $derived($gameStore.upcomingQuestions || []);

	/**
	 * Identify the correct answer from the next question in queue
	 */
	const correctAnswer = $derived(
		upcomingQuestions[0]?.correct_character || upcomingQuestions[0]?.colors?.[0] || ''
	);

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
	 * Builds the list of CSS classes for each alternative.
	 * Uses the same metallic approach as the updated question-view.
	 */
	function getButtonStyles(alternative: string) {
		const styles = [
			'aspect-square',
			'rounded-lg',
			'transition-all',
			'duration-150',
			'relative',
			'pointer-events-none' // admin view is read-only
		];

		// Check if this is the correct answer
		if (alternative === correctAnswer) {
			styles.push('ring-4', 'ring-offset-2', 'ring-green-500', 'bg-green-500/20', 'z-10');
		}

		// If it's a color question, apply metallic class if needed
		if (questionType === 'color') {
			const lower = alternative.toLowerCase();
			if (lower === 'gold') {
				styles.push('metallic-gold');
			} else if (lower === 'silver') {
				styles.push('metallic-silver');
			}
		}

		// If it's a character question, show an image
		if (questionType === 'character') {
			styles.push('p-0', 'overflow-hidden');
		}
		// For non-color questions (other than character), use a muted background
		else if (questionType !== 'color') {
			styles.push('bg-muted');
		}

		return styles.join(' ');
	}
</script>

<Card>
	<CardContent class="py-2">
		<div class="flex items-center gap-4">
			<!-- Question options -->
			<div class="grid flex-1 grid-cols-6 gap-2">
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
							<!-- Fallback text for non-color, non-character questions -->
							<div class="flex h-full w-full items-center justify-center text-sm font-medium">
								{alternative}
							</div>
						{/if}
					</button>
				{/each}
			</div>
		</div>
	</CardContent>
</Card>
