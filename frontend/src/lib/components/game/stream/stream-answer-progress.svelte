<script lang="ts">
	import type { PublicGameState } from '$lib/types/game';
	import { Progress } from '$lib/components/ui/progress';
	import { ScrollArea } from '$lib/components/ui/scroll-area';

	interface Props {
		gameState: PublicGameState | null;
	}
	let { gameState }: Props = $props();

	interface DisplayListItem {
		name: string;
		isAnswered: boolean;
		correct?: boolean; // Only if revealed
	}

	const MAX_DISPLAYED_NAMES = 50;

	const allPlayersList = $derived(gameState?.players || []);
	const currentAnswersList = $derived(gameState?.currentAnswersPublic || []);

	const totalPlayerCount = $derived(allPlayersList.length);
	const answeredCount = $derived(currentAnswersList.length);
	const progress = $derived(totalPlayerCount > 0 ? (answeredCount / totalPlayerCount) * 100 : 0);

	// Show correctness only in Score or GameOver phases
	const shouldRevealCorrectness = $derived(
		gameState?.phase.type === 'score' || gameState?.phase.type === 'gameover'
	);

	const allSortedDisplayItems = $derived(() => {
		const answeredPlayerNames = new Set(currentAnswersList.map((ans) => ans.name));

		const answeredDisplayItems: DisplayListItem[] = currentAnswersList.map((answer) => ({
			name: answer.name,
			isAnswered: true,
			correct: shouldRevealCorrectness ? answer.isCorrect : undefined
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

<div class="space-y-2">
	<div class="flex justify-between text-sm">
		<span class="text-muted-foreground">Answers</span>
		<span>{answeredCount}/{totalPlayerCount}</span>
	</div>

	<Progress value={progress} class="h-2" />

	{#if renderedDisplayItems.length > 0}
		<ScrollArea class="h-32 w-full rounded border">
			<div class="space-y-0.5 p-2">
				{#each renderedDisplayItems as item (item.name)}
					<div
						class="flex items-center justify-between rounded px-2 py-1 text-sm transition-colors
						{item.isAnswered
							? shouldRevealCorrectness && item.correct === true
								? 'bg-green-500/10 text-green-700 dark:text-green-400'
								: shouldRevealCorrectness && item.correct === false
									? 'bg-red-500/10 text-red-700 dark:text-red-400'
									: 'bg-blue-500/10 text-blue-700 dark:text-blue-400'
							: 'text-muted-foreground'}"
					>
						<span class="truncate">{item.name}</span>
						<span class="ml-2 flex-none">
							{#if item.isAnswered}
								{#if shouldRevealCorrectness && item.correct !== undefined}
									{item.correct ? '✓' : '✗'}
								{:else}
									✓
								{/if}
							{:else}
								⋯
							{/if}
						</span>
					</div>
				{/each}

				{#if remainingHiddenPlayersCount > 0}
					<div class="py-1 text-center text-xs text-muted-foreground">
						+{remainingHiddenPlayersCount} more players
					</div>
				{/if}
			</div>
		</ScrollArea>
	{/if}
</div>
