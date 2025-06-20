<script lang="ts">
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { streamStore } from '$lib/stores/stream.store.svelte';

	interface DisplayListItem {
		name: string;
		isAnswered: boolean;
		correct?: boolean; // Only if revealed
	}

	const MAX_DISPLAYED_NAMES = 100;

	const gameState = $derived(streamStore.state.gameState);
	const allPlayersList = $derived(gameState?.realtimeScoreboard || []);
	const currentAnswersList = $derived(gameState?.currentAnswers || []);

	const totalPlayerCount = $derived(allPlayersList.length);

	const allSortedDisplayItems = $derived(() => {
		const answeredPlayerNames = new Set(currentAnswersList.map((ans) => ans.name));

		const answeredDisplayItems: DisplayListItem[] = currentAnswersList.map((answer) => ({
			name: answer.name,
			isAnswered: true,
			correct: answer.score > 0
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
	const remainingHiddenPlayersCount = $derived(Math.max(0, totalPlayerCount - MAX_DISPLAYED_NAMES));
</script>

<div class="flex h-full flex-col overflow-hidden rounded-lg bg-card shadow">
	<ScrollArea class="flex-1 p-2">
		<div class="flex flex-wrap gap-1.5">
			{#each renderedDisplayItems as item (item.name)}
				<div
					class="rounded px-2 py-1 text-4xl font-medium {item.isAnswered
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
