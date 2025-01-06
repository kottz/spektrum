<script lang="ts">
	import { gameStore } from '../../stores/game';
	import { Progress } from '$lib/components/ui/progress';
	// Get total players and answers
	$: players = Array.from($gameStore.players.values());
	$: answers = $gameStore.currentAnswers || [];
	$: progress = (answers.length / players.length) * 100;
</script>

<div class="space-y-2">
	<div class="flex justify-between text-sm">
		<span class="text-muted-foreground">Answers</span>
		<span>{answers.length}/{players.length}</span>
	</div>
	<!-- Progress bar -->
	<Progress value={progress} class="h-2 bg-muted" />
	<!-- Answer pills -->
	<div class="flex min-h-[32px] flex-wrap gap-1.5">
		{#each answers as answer}
			<div
				class="rounded px-2 py-1 text-sm font-medium {answer.correct
					? 'bg-emerald-500/20 text-emerald-400'
					: 'bg-red-500/20 text-red-400'}"
			>
				{answer.name}
			</div>
		{/each}
	</div>
</div>
