<script lang="ts">
	import { gameStore } from '../../stores/game';
	import { Card, CardContent } from '$lib/components/ui/card';

	const alternatives = $derived($gameStore.currentQuestion?.alternatives || []);
	const questionType = $derived($gameStore.currentQuestion?.type || 'default');
	const upcomingQuestions = $derived($gameStore.upcomingQuestions || []);
	const correctAnswer = $derived(
		upcomingQuestions[0]?.correct_character || upcomingQuestions[0]?.colors?.[0] || ''
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
		brown: '#3D251E',
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

		if (alternative === correctAnswer) {
			styles.push('ring-4', 'ring-offset-2', 'ring-green-500', 'bg-green-500/20', 'z-10');
		}

		if (questionType === 'character') {
			styles.push('p-0', 'overflow-hidden');
		} else if (questionType !== 'color') {
			styles.push('bg-muted');
		}

		return styles.join(' ');
	}
</script>

<Card>
	<CardContent class="py-2">
		<div class="grid max-h-[100px] grid-cols-6 gap-2">
			{#each alternatives as alternative}
				<button
					class={getButtonStyles(alternative)}
					disabled={true}
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
						<div class="flex h-full w-full items-center justify-center text-sm font-medium">
							{alternative}
						</div>
					{/if}
				</button>
			{/each}
		</div>
	</CardContent>
</Card>
