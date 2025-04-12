<script lang="ts">
	import { Progress } from '$lib/components/ui/progress';
	import { timerStore } from '$lib/stores/timer-store.svelte';
	import { gameStore } from '$lib/stores/game.svelte';

	// Access the time directly from the store's state
	const timeLeft = $derived(timerStore.state.timeLeft);
	const progress = $derived((timeLeft / gameStore.state.roundDuration) * 100);
	const points = $derived((timeLeft / gameStore.state.roundDuration) * 5000); // Assuming 5000 is the max points
</script>

<div class="space-y-2">
	<div class="flex justify-between text-sm">
		<span class="text-muted-foreground">Time Remaining</span>
		<span class="w-24 text-right text-sm font-medium">{points.toFixed(0)}</span>
	</div>
	<div class="flex items-center gap-4">
		<Progress value={progress} class="h-2 flex-1 bg-muted" />
		<span class="w-12 text-right text-sm font-medium">{timeLeft.toFixed(1)}s</span>
	</div>
</div>
