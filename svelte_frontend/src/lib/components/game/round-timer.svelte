<script lang="ts">
	import { Card, CardContent } from '$lib/components/ui/card';
	import { Progress } from '$lib/components/ui/progress';
	import { gameStore } from '../../stores/game';
	import { onMount, onDestroy } from 'svelte';

	let timeLeft = $gameStore.roundDuration;
	let progress = 100;
	let timer: number;

	onMount(() => {
		timer = window.setInterval(() => {
			if (timeLeft > 0) {
				timeLeft -= 0.1;
				progress = (timeLeft / $gameStore.roundDuration) * 100;
			}
		}, 100);
	});

	onDestroy(() => {
		clearInterval(timer);
	});

	$: formattedTime = timeLeft.toFixed(1);
</script>

<!-- Round Timer Card -->
<Card>
	<CardContent class="p-4">
		<div class="space-y-2">
			<div class="flex justify-between text-sm">
				<span class="text-muted-foreground">Time Remaining</span>
			</div>
			<div class="flex items-center gap-4">
				<Progress value={progress} class="h-2 flex-1 bg-muted" />
				<span class="w-12 text-right text-sm font-medium">{formattedTime}s</span>
			</div>
		</div>
	</CardContent>
</Card>
