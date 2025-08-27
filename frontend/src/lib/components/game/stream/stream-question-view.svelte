<script lang="ts">
	import { streamStore } from '$lib/stores/stream.store.svelte';
	import { Card, CardContent } from '$lib/components/ui/card';
	import { PUBLIC_SPEKTRUM_CDN_URL } from '$env/static/public';

	const imageBaseUrl = PUBLIC_SPEKTRUM_CDN_URL + '/img';

	// Get question data from stream store
	const gameState = $derived(streamStore.state.gameState);
	const question = $derived(gameState?.currentQuestion);
	const alternatives = $derived(question?.alternatives || []);
	const questionType = $derived(question?.type || 'default');
	const questionText = $derived(question?.text ?? '');

	// Similar styling logic from your existing question views, but non-interactive
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

	function getAlternativeStyles(alternative: string) {
		const styles: string[] = [
			'rounded-lg',
			'relative',
			'transition-all',
			'duration-150',
			'pointer-events-none'
		]; // Non-interactive

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
			styles.push('py-3', 'px-4', 'bg-muted', 'flex', 'items-center', 'justify-center', 'h-full');
		} else {
			styles.push('aspect-square', 'bg-muted', 'flex', 'items-center', 'justify-center');
		}

		// Color-specific styling
		if (questionType === 'color') {
			const lower = alternative.toLowerCase();
			if (lower === 'white') styles.push('border-2', 'border-black');
			if (lower === 'black') styles.push('border-2', 'dark:border-white');
			if (lower === 'gold') styles.push('metallic-gold');
			else if (lower === 'silver') styles.push('metallic-silver');
		}

		return styles.join(' ');
	}
</script>

{#if question}
	<Card>
		<CardContent class="p-4">
			{#if questionType === 'text' && questionText}
				<p class="mb-4 text-center text-xl font-semibold">{questionText}</p>
			{/if}

			<div class="grid gap-2 {questionType === 'text' ? 'grid-cols-2' : 'grid-cols-3'}">
				{#each alternatives as alt}
					<div
						class={getAlternativeStyles(alt)}
						style={questionType === 'color' &&
						!(alt.toLowerCase() === 'gold' || alt.toLowerCase() === 'silver')
							? `background-color: ${colorMap[alt.toLowerCase()] || '#000000'};`
							: ''}
					>
						{#if questionType === 'character'}
							<div class="aspect-square w-full">
								{#if alt.endsWith('_video')}
									<video
										src="{imageBaseUrl}/{alt}.webm"
										class="h-full w-full object-contain"
										autoplay
										loop
										muted
										playsinline
									></video>
								{:else}
									<img
										src="{imageBaseUrl}/{alt}.avif"
										{alt}
										class="h-full w-full object-contain text-transparent"
									/>
								{/if}
							</div>
						{:else if questionType === 'color'}
							<span class="sr-only">{alt}</span>
						{:else if questionType === 'text'}
							<span class="text-center text-lg font-medium">{alt}</span>
						{:else}
							<div class="flex h-full w-full items-center justify-center text-lg font-medium">
								{alt}
							</div>
						{/if}
					</div>
				{/each}
			</div>
		</CardContent>
	</Card>
{/if}
