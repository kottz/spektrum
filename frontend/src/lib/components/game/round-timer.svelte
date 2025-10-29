<script lang="ts">
	import { timerStore } from '$lib/stores/timer-store.svelte';
	import { gameStore } from '$lib/stores/game.svelte';

	const actualTimeLeftInRound = $derived(timerStore.state.timeLeft);
	const timeLeftForPoints = $derived(
		timerStore.state.answeredTimeSnapshot !== null
			? timerStore.state.answeredTimeSnapshot
			: actualTimeLeftInRound
	);

	const totalRoundDuration = $derived(gameStore.state.roundDuration || 60);
	const answeredBarProgress = $derived(
		timerStore.state.answeredTimeSnapshot !== null && totalRoundDuration > 0
			? (timerStore.state.answeredTimeSnapshot / totalRoundDuration) * 100
			: 0
	);

	const mainBarProgress = $derived(
		totalRoundDuration > 0 ? (actualTimeLeftInRound / totalRoundDuration) * 100 : 0
	);

	const points = $derived(() => {
		const playerAnswer = gameStore.state.currentAnswers.find(
			(a) => a.name === gameStore.state.playerName
		);

		if (playerAnswer?.score != null) {
			return playerAnswer.score;
		}

		return totalRoundDuration > 0 ? (timeLeftForPoints / totalRoundDuration) * 5000 : 0;
	});
</script>

<div class="space-y-2">
	<div class="flex justify-between text-sm">
		<span class="text-muted-foreground">Time Remaining</span>
		<span class="w-24 text-right text-sm font-medium">{points().toFixed(0)}</span>
	</div>
	<div class="flex items-center gap-4">
		<div class="bg-secondary relative h-2 flex-1 overflow-hidden rounded-full">
			{#if timerStore.state.answeredTimeSnapshot !== null}
				<div
					class="absolute top-0 left-0 z-10 h-full bg-gray-700"
					style:width="{answeredBarProgress}%"
					title="Answered at this time"
				></div>
			{/if}

			<div
				class="bg-primary absolute top-0 left-0 z-20 h-full transition-transform duration-100 ease-linear"
				style:width="{mainBarProgress}%"
			></div>
		</div>
		<span class="w-12 text-right text-sm font-medium">{actualTimeLeftInRound.toFixed(1)}s</span>
	</div>
</div>
