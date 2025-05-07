<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import { Progress } from '$lib/components/ui/progress';
	import { ScrollArea } from '$lib/components/ui/scroll-area';

	interface DisplayListItem {
		name: string;
		isAnswered: boolean;
		correct?: boolean; // Only if isAnswered is true
	}

	const MAX_DISPLAYED_NAMES = 50; // Cutoff limit

	const allPlayersList = $derived(Array.from(gameStore.state.players.values()));
	const currentAnswersList = $derived(gameStore.state.currentAnswers || []);

	const totalPlayerCount = $derived(allPlayersList.length);
	const answeredCount = $derived(currentAnswersList.length);
	const progress = $derived(totalPlayerCount > 0 ? (answeredCount / totalPlayerCount) * 100 : 0);

	const allSortedDisplayItems = $derived(() => {
		const answeredPlayerNames = new Set(currentAnswersList.map((ans) => ans.name));

		const answeredDisplayItems: DisplayListItem[] = currentAnswersList.map((answer) => ({
			name: answer.name,
			isAnswered: true,
			correct: answer.correct
		}));

		const unansweredDisplayItems: DisplayListItem[] = allPlayersList
			.filter((player) => !answeredPlayerNames.has(player.name))
			.sort((a, b) => b.score - a.score) // Sort by score, descending
			.map((player) => ({
				name: player.name,
				isAnswered: false
			}));

		return [...answeredDisplayItems, ...unansweredDisplayItems];
	});

	const renderedDisplayItems = $derived(allSortedDisplayItems().slice(0, MAX_DISPLAYED_NAMES));
	const remainingHiddenPlayersCount = $derived(
		Math.max(0, totalPlayerCount - MAX_DISPLAYED_NAMES) // totalPlayerCount comes from allPlayersList.length
	);
</script>

<div class="space-y-2">
	<div class="flex justify-between text-sm">
		<span class="text-muted-foreground">Answers</span>
		<span>{answeredCount}/{totalPlayerCount}</span>
	</div>
	<Progress value={progress} class="h-2 bg-muted" />
	<ScrollArea orientation="horizontal" class="min-h-[32px] whitespace-nowrap">
		<div class="flex w-max items-center gap-1.5">
			{#each renderedDisplayItems as item (item.name)}
				<div
					class="rounded px-2 py-1 text-sm font-medium {item.isAnswered
						? item.correct
							? 'bg-emerald-500/20 text-emerald-400' // Answered and correct
							: 'bg-red-500/20 text-red-400' // Answered and incorrect
						: 'bg-neutral-200 text-neutral-500 dark:bg-neutral-700 dark:text-neutral-400'}"
				>
					{item.name}
				</div>
			{/each}
			{#if remainingHiddenPlayersCount > 0}
				<div
					class="rounded bg-neutral-300 px-2 py-1 text-sm font-medium text-neutral-600 dark:bg-neutral-600 dark:text-neutral-300"
					title="{remainingHiddenPlayersCount} more players not shown"
				>
					+{remainingHiddenPlayersCount}
				</div>
			{/if}
		</div>
	</ScrollArea>
</div>
