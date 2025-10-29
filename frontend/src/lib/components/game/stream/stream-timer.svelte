<script lang="ts">
	import { streamTimerStore } from '$lib/stores/stream-timer.store.svelte';
	import { streamStore } from '$lib/stores/stream.store.svelte';
	import { GamePhase } from '$lib/types/game';
	import { Separator } from '$lib/components/ui/separator';

	const timeLeft = $derived(streamTimerStore.state.timeLeft);
	const isActive = $derived(streamTimerStore.state.isActive);
	const totalDuration = $derived(streamTimerStore.state.roundDuration);
	const gameState = $derived(streamStore.state.gameState);
	const phase = $derived(gameState?.phase);

	const progress = $derived(
		isActive && totalDuration > 0 && timeLeft > 0 ? (timeLeft / totalDuration) * 100 : 0
	);

	const timeDisplay = $derived(isActive && timeLeft > 0 ? timeLeft.toFixed(1) : '0.0');
	const points = $derived(totalDuration > 0 ? (timeLeft / totalDuration) * 5000 : 0);
	const shouldShow = $derived(phase === GamePhase.Question);
</script>

{#if shouldShow}
	<div class="flex items-center gap-4">
		<div class="bg-secondary relative h-4 w-[20vw] overflow-hidden rounded-full">
			<div
				class="bg-primary absolute top-0 left-0 h-full transition-all duration-100 ease-linear"
				style="width: {progress}%"
			></div>
		</div>
		<div class="flex items-center gap-2 text-4xl font-bold">
			<span class="w-24 text-right">{timeDisplay}s</span>
			<Separator orientation="vertical" />
			<span class="w-24 text-left">{points.toFixed(0)}</span>
		</div>
	</div>
{/if}
