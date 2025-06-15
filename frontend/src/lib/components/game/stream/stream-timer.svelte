<script lang="ts">
	import { streamTimerStore } from '$lib/stores/stream-timer.store.svelte';

	const timeLeft = $derived(streamTimerStore.state.timeLeft);
	const isActive = $derived(streamTimerStore.state.isActive);
	const totalDuration = $derived(streamTimerStore.state.roundDuration);

	const progress = $derived(totalDuration > 0 ? (timeLeft / totalDuration) * 100 : 0);

	const timeDisplay = $derived(timeLeft.toFixed(1));
</script>

{#if isActive}
	<div class="rounded-lg bg-card p-4 shadow">
		<div class="flex items-center gap-4">
			<div class="relative h-4 flex-1 overflow-hidden rounded-full bg-secondary">
				<div
					class="absolute left-0 top-0 h-full bg-primary transition-all duration-100 ease-linear"
					style="width: {progress}%"
				></div>
			</div>
			<span class="w-16 text-right text-lg font-bold">{timeDisplay}s</span>
		</div>
	</div>
{/if}
