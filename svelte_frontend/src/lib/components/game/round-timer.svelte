<script lang="ts">
	import { Card, CardContent } from '$lib/components/ui/card';
	import { Progress } from '$lib/components/ui/progress';
	import { gameStore } from '../../stores/game';

	let timeLeft = $state($gameStore.roundDuration);
	let progress = $state(100);
	let timer: number;

	$effect(() => {
		const phase = $gameStore.phase;
		const isAdmin = $gameStore.isAdmin;

		// Start timer in question phase
		if (phase === 'question') {
			// Always reset timer for admin, reset for players only if no timer running
			if (isAdmin || !timer) {
				timeLeft = $gameStore.roundDuration;
				progress = 100;
				timer = window.setInterval(() => {
					if (timeLeft > 0) {
						timeLeft = Math.max(0.0, timeLeft - 0.1);
						progress = (timeLeft / $gameStore.roundDuration) * 100;
					} else if (timer && !isAdmin) {
						// Only clear interval for non-admin when reaching 0
						clearInterval(timer);
						timer = undefined;
					}
				}, 100);
			}
		} else if (phase !== 'question') {
			// Clear timer when leaving question phase
			if (timer) {
				clearInterval(timer);
				timer = undefined;
			}
		}

		return () => {
			if (timer) {
				clearInterval(timer);
			}
		};
	});
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
