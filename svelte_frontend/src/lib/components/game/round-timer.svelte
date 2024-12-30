<script lang="ts">
	import { Card, CardContent } from '$lib/components/ui/card';
	import { Progress } from '$lib/components/ui/progress';
	import { timerStore } from '../../stores/timer-store';

	const { compact = false } = $props<{
		compact?: boolean;
	}>();

	const progress = $derived(($timerStore / 60) * 100);
</script>

{#if !compact}
	<Card>
		<CardContent class="p-4">
			<div class="space-y-2">
				<div class="flex justify-between text-sm">
					<span class="text-muted-foreground">Time Remaining</span>
				</div>
				<div class="flex items-center gap-4">
					<Progress value={progress} class="h-2 flex-1 bg-muted" />
					<span class="w-12 text-right text-sm font-medium">{$timerStore.toFixed(1)}s</span>
				</div>
			</div>
		</CardContent>
	</Card>
{:else}
	<div class="flex items-center gap-4">
		<Progress value={progress} class="h-2 flex-1 bg-muted" />
		<span class="w-12 text-right text-sm font-medium">{$timerStore.toFixed(1)}s</span>
	</div>
{/if}
