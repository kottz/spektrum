<script lang="ts">
	import { Progress } from '$lib/components/ui/progress';
	import { timerStore } from '../../stores/timer-store';
	import { gameStore } from '../../stores/game';

	// Just subscribe to the time
	const timeLeft = $derived($timerStore);
	const progress = $derived((timeLeft / $gameStore.roundDuration) * 100);
</script>

<div class="space-y-2">
	<div class="flex justify-between text-sm">
		<span class="text-muted-foreground">Time Remaining</span>
	</div>
	<div class="flex items-center gap-4">
		<Progress value={progress} class="h-2 flex-1 bg-muted" />
		<span class="w-12 text-right text-sm font-medium">{timeLeft.toFixed(1)}s</span>
	</div>
</div>
