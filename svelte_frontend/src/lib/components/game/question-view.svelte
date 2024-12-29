<script lang="ts">
	import { gameStore } from '../../stores/game';
	import { gameActions } from '../../stores/game-actions';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import AnswerProgress from './answer-progress.svelte';

	// Get current game state
	$: alternatives = $gameStore.currentQuestion?.alternatives || [];
	$: questionType = $gameStore.currentQuestion?.type || 'default';
	$: currentPlayer = $gameStore.playerName
		? $gameStore.players.get($gameStore.playerName)
		: undefined;
	$: hasAnswered = currentPlayer?.hasAnswered || false;
	$: selectedAnswer = currentPlayer?.answer;
	$: timeRemaining = $gameStore.roundDuration;

	// Color mapping
	const colorMap = {
		Red: '#FF0000',
		Green: '#00FF00',
		Blue: '#0000FF',
		Yellow: '#FFFF00',
		Purple: '#800080',
		Gold: 'linear-gradient(45deg, #FFD700, #FDB931, #FFD700)',
		Silver: 'linear-gradient(45deg, #C0C0C0, #E8E8E8, #C0C0C0)',
		Pink: '#FFC0CB',
		Black: '#000000',
		White: '#FFFFFF',
		Brown: '#3D251E',
		Orange: '#FFA500',
		Gray: '#808080'
	};

	function handleAnswer(answer: string) {
		if (!hasAnswered) {
			gameActions.submitAnswer(answer);
		}
	}

	function getButtonStyles(alternative: string) {
		let baseStyles = 'transition-all duration-200 ';

		if (hasAnswered) {
			if (alternative === selectedAnswer) {
				baseStyles += 'border-primary border-2 ';
			} else {
				baseStyles += 'opacity-50 ';
			}
			baseStyles += 'cursor-not-allowed ';
		} else {
			baseStyles += 'hover:border-border border-2 border-transparent cursor-pointer ';
		}

		if (questionType === 'color') {
			const color = alternative.toLowerCase();
			if (color === 'gold' || color === 'silver') {
				baseStyles += `bg-gradient-to-r from-${color}-400 via-${color}-500 to-${color}-400 `;
			} else {
				baseStyles += 'text-white ';
			}
		} else if (questionType === 'character') {
			baseStyles += 'bg-muted p-0 overflow-hidden ';
		} else {
			baseStyles += 'bg-muted ';
		}

		return baseStyles;
	}
</script>

<div class="container mx-auto max-w-2xl space-y-6 p-6">
	<!-- Answer Progress -->
	<Card>
		<CardContent class="p-4">
			<AnswerProgress />
		</CardContent>
	</Card>

	<!-- Answer Options -->
	<Card>
		<CardHeader>
			<CardTitle class="flex items-center justify-between">
				<span>Choose your answer</span>
				<span class="text-muted-foreground">{timeRemaining}s</span>
			</CardTitle>
		</CardHeader>
		<CardContent>
			<div class="grid grid-cols-2 gap-4">
				{#each alternatives as alternative}
					<button
						class="aspect-square rounded-lg {getButtonStyles(alternative)}"
						disabled={hasAnswered}
						on:click={() => handleAnswer(alternative)}
						style={questionType === 'color'
							? alternative.toLowerCase() === 'gold' || alternative.toLowerCase() === 'silver'
								? `background: ${colorMap[alternative]}`
								: `background-color: ${colorMap[alternative]}`
							: ''}
					>
						{#if questionType === 'character'}
							<img
								src={`http://192.168.1.155:8765/img_avif/${alternative}.avif`}
								alt={alternative}
								class="h-full w-full object-cover"
							/>
						{:else if questionType === 'color'}
							<div class="sr-only">{alternative}</div>
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
