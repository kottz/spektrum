<script lang="ts">
	import type { PublicGameState } from '$lib/types/game';
	import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';

	interface Props {
		gameState: PublicGameState | null;
	}
	let { gameState }: Props = $props();

	const PAGE_SIZE = 25;

	const allSortedPlayers = $derived(
		gameState?.players ? [...gameState.players].sort((a, b) => b.score - a.score) : []
	);

	let currentPage = $state(1);
	const totalPlayers = $derived(allSortedPlayers.length);
	const maxPages = $derived(Math.ceil(totalPlayers / PAGE_SIZE));

	// The subset of players to be rendered in the DOM.
	const renderedPlayers = $derived(allSortedPlayers.slice(0, currentPage * PAGE_SIZE));
	let loaderElement: HTMLDivElement | undefined = $state();

	const maxScore = $derived(allSortedPlayers[0]?.score || 0);

	const gameOver = $derived(gameState?.phase.type === 'gameover');
	const playingQuestion = $derived(gameState?.phase.type === 'question');

	function getScoreWidth(score: number): string {
		if (maxScore === 0) return '0%';
		return `${(score / maxScore) * 100}%`;
	}

	$effect(() => {
		const currentLoaderEl = loaderElement;
		if (!currentLoaderEl || currentPage >= maxPages) return;

		const observer = new IntersectionObserver(
			(entries) => {
				if (entries[0].isIntersecting && currentPage < maxPages) {
					currentPage++;
				}
			},
			{ threshold: 0.1 }
		);

		observer.observe(currentLoaderEl);

		return () => {
			observer.disconnect();
		};
	});
</script>

<div class="flex h-full flex-col overflow-hidden rounded-lg bg-card shadow">
	<div class="flex-none border-b border-border bg-muted/50 px-4 py-3">
		<h2 class="text-lg font-semibold">
			{#if gameOver}
				Final Scores
			{:else}
				Scoreboard ({totalPlayers} players)
			{/if}
		</h2>
	</div>

	<ScrollArea class="flex-1">
		<div class="space-y-1 p-2">
			{#each renderedPlayers as player, index (player.name)}
				<div
					class="flex items-center gap-3 rounded-md p-2 transition-colors {playingQuestion &&
					player.hasAnsweredPublic
						? 'bg-green-500/10'
						: 'bg-muted/30'}"
				>
					<!-- Rank -->
					<div class="flex-none text-sm font-medium text-muted-foreground">
						#{index + 1}
					</div>

					<!-- Player name and score bar -->
					<div class="min-w-0 flex-1">
						<div class="flex items-center justify-between">
							<span class="truncate font-medium" title={player.name}>
								{player.name}
								{#if playingQuestion && player.hasAnsweredPublic}
									<span class="ml-1 text-xs text-green-600">✓</span>
								{/if}
							</span>
							<span class="text-sm font-semibold">{player.score}</span>
						</div>
						{#if !gameOver && maxScore > 0}
							<div class="mt-1 h-1.5 w-full rounded-full bg-muted">
								<div
									class="h-full rounded-full bg-primary transition-all duration-300"
									style="width: {getScoreWidth(player.score)}"
								></div>
							</div>
						{/if}
					</div>
				</div>
			{/each}

			{#if currentPage < maxPages}
				<div bind:this={loaderElement} class="flex justify-center py-4">
					<div class="text-sm text-muted-foreground">Loading more players...</div>
				</div>
			{/if}
		</div>
	</ScrollArea>
</div>
