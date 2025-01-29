<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import { Progress } from '$lib/components/ui/progress';
	import { ScrollArea } from '$lib/components/ui/scroll-area';

	const players = $derived(Array.from(gameStore.state.players.values()));
	const answers = $derived(gameStore.state.currentAnswers || []);
	const progress = $derived((answers.length / players.length) * 100);
</script>

<div class="space-y-2">
	<div class="flex justify-between text-sm">
		<span class="text-muted-foreground">Answers</span>
		<span>{answers.length}/{players.length}</span>
	</div>
	<Progress value={progress} class="h-2 bg-muted" />
	<ScrollArea orientation="horizontal" class="min-h-[32px] whitespace-nowrap">
		<div class="flex w-max gap-1.5">
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
	</ScrollArea>
</div>
