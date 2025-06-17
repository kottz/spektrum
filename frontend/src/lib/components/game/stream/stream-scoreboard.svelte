<script lang="ts">
	import { GamePhase } from '$lib/types/game';
	import { streamStore } from '$lib/stores/stream.store.svelte';
	import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
	import { flip } from 'svelte/animate';
	import { fly } from 'svelte/transition';

	const PAGE_SIZE = 25;

	const gameState = $derived(streamStore.state.gameState);
	const allSortedPlayers = $derived(gameState?.realtimeScoreboard || []);

	let currentPage = $state(1);
	const totalPlayers = $derived(allSortedPlayers.length);
	const maxPages = $derived(Math.ceil(totalPlayers / PAGE_SIZE));

	// The subset of players to be rendered in the DOM.
	const renderedPlayers = $derived(allSortedPlayers.slice(0, currentPage * PAGE_SIZE));
	let loaderElement: HTMLDivElement | undefined = $state();

	const maxScore = $derived(allSortedPlayers[0]?.score || 0);

	const gameOver = $derived(gameState?.phase === GamePhase.GameOver);
	const playingQuestion = $derived(gameState?.phase === GamePhase.Question);

	// Create a map of players who have answered with their answer details
	const playerAnswers = $derived(
		new Map(gameState?.currentAnswers.map((answer) => [answer.name, answer]) || [])
	);

	function getScoreWidth(score: number): string {
		if (maxScore === 0) return '0%';
		return `${(score / maxScore) * 100}%`;
	}

	function hasAnswered(playerName: string): boolean {
		return playerAnswers.has(playerName);
	}

	function getAnswerStatus(playerName: string): 'correct' | 'incorrect' | 'none' {
		const answer = playerAnswers.get(playerName);
		if (!answer) return 'none';
		return answer.score > 0 ? 'correct' : 'incorrect';
	}

	function getPlayerBorderClass(playerName: string): string {
		if (!playingQuestion) return 'border-2 border-transparent';

		const status = getAnswerStatus(playerName);
		switch (status) {
			case 'correct':
				return 'border-2 border-emerald-500';
			case 'incorrect':
				return 'border-2 border-red-500';
			default:
				return 'border-2 border-transparent';
		}
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

<div class="flex h-full w-full flex-col overflow-hidden rounded-lg bg-card shadow">
	<div class="flex-none border-b border-border bg-muted/50 px-4 py-3">
		<h2 class="text-xl font-bold">
			{#if gameOver}
				Final Scores
			{:else}
				Scoreboard ({totalPlayers} players)
			{/if}
		</h2>
	</div>

	<div class="flex min-h-0 flex-1">
		<ScrollArea class="min-h-0 w-1 flex-1">
			<div class="space-y-1 p-2">
				{#each renderedPlayers as player, index (player.name)}
					<div
						class="relative overflow-hidden rounded-md transition-all duration-300 {getPlayerBorderClass(
							player.name
						)}"
						animate:flip={{ duration: 500 }}
						in:fly={{ x: -20, duration: 300 }}
					>
						<!-- Background score bar -->
						{#if !gameOver && maxScore > 0}
							<div
								class="absolute inset-0 bg-primary/20 transition-all duration-500"
								style="width: {getScoreWidth(player.score)}"
							></div>
						{/if}

						<!-- Content -->
						<div class="relative flex items-center gap-4 p-3">
							<!-- Rank -->
							<div class="flex-none text-lg font-semibold text-muted-foreground">
								#{index + 1}
							</div>

							<!-- Player name and scores -->
							<div class="min-w-0 flex-1 overflow-hidden">
								<div class="flex items-center justify-between gap-3">
									<span class="flex-1 truncate text-lg font-semibold" title={player.name}>
										{player.name}
									</span>
									<div class="flex flex-none items-center gap-3 text-lg font-bold">
										{#if player.roundScore > 0}
											<span class="text-emerald-400">+{player.roundScore}</span>
										{/if}
										<span>{player.score.toLocaleString()}</span>
									</div>
								</div>
							</div>
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
</div>
