<script lang="ts">
	import { Card } from '$lib/components/ui/card';
	import { streamStore } from '$lib/stores/stream.store.svelte';
	import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';

	const PAGE_SIZE = 25;

	const gameState = $derived(streamStore.state.gameState);
	const allSortedPlayers = $derived(
		gameState?.realtimeScoreboard
			? [...gameState.realtimeScoreboard].sort((a, b) => b.score - a.score)
			: []
	);
	const winner = $derived(allSortedPlayers[0]);

	let currentPage = $state(1);
	const totalPlayers = $derived(allSortedPlayers.length);
	const maxPages = $derived(Math.ceil(totalPlayers / PAGE_SIZE));

	// The subset of players to be rendered in the DOM
	const renderedPlayers = $derived(allSortedPlayers.slice(0, currentPage * PAGE_SIZE));
	let loaderElement: HTMLDivElement | undefined = $state();

	$effect(() => {
		const currentLoaderEl = loaderElement;

		if (!currentLoaderEl) {
			return;
		}

		// Find the Radix ScrollArea Viewport to use as the observer's root
		const scrollViewportRoot: HTMLElement | null = currentLoaderEl.closest(
			'[data-radix-scroll-area-viewport]'
		);

		const observer = new IntersectionObserver(
			(entries) => {
				if (entries[0].isIntersecting) {
					if (currentPage < maxPages) {
						currentPage += 1;
					}
				}
			},
			{
				root: scrollViewportRoot,
				threshold: 0.01
			}
		);

		observer.observe(currentLoaderEl);

		return () => {
			observer.unobserve(currentLoaderEl);
			observer.disconnect();
		};
	});
</script>

<div class="container mx-auto flex h-full w-full max-w-6xl flex-col overflow-hidden p-8">
	<!-- Winner announcement -->
	<Card class="mb-12">
		<div class="px-6 py-8 text-center">
			<h1 class="text-6xl font-bold">🎉 Game Over! 🎉</h1>
			{#if winner}
				<div class="mb-3 text-5xl font-bold">
					Winner: {winner.name}
				</div>
				<div class="text-muted-foreground text-3xl">
					Final Score: {winner.score.toLocaleString()} points
				</div>
			{/if}
		</div>
	</Card>

	<!-- Final Scoreboard - working height structure with original styling -->
	<Card class="flex h-full w-full flex-col overflow-hidden shadow-sm">
		<div class="flex-none px-6 py-6">
			<h2 class="mb-6 text-center text-4xl font-bold">Final Leaderboard</h2>
		</div>

		<div class="flex min-h-0 flex-1">
			<ScrollArea class="min-h-0 w-1 flex-1">
				<div class="px-6 pb-6">
					{#each renderedPlayers as player, index (player.name)}
						<div
							class="relative mb-3 overflow-hidden rounded-md {index === 0
								? 'border-2 border-yellow-500 bg-yellow-500/20'
								: 'bg-muted/30'}"
						>
							<!-- Background score bar for visual appeal -->
							{#if allSortedPlayers[0]?.score > 0}
								<div
									class="bg-primary/10 absolute inset-0 transition-all duration-500"
									style="width: {(player.score / allSortedPlayers[0].score) * 100}%"
								></div>
							{/if}

							<!-- Content -->
							<div class="relative flex items-center gap-4 p-4">
								<!-- Rank -->
								<div
									class="flex-none text-2xl font-bold {index === 0
										? 'text-yellow-600'
										: 'text-muted-foreground'}"
								>
									#{index + 1}
								</div>

								<!-- Player name and scores -->
								<div class="min-w-0 flex-1 overflow-hidden">
									<div class="flex items-center justify-between gap-3">
										<span
											class="flex-1 truncate text-2xl font-bold {index === 0
												? 'text-yellow-600'
												: ''}"
											title={player.name}
										>
											{player.name}
											{#if index === 0}
												<span class="ml-2">👑</span>
											{/if}
										</span>
										<div class="flex flex-none items-center gap-3 text-xl font-bold">
											{#if player.roundScore > 0}
												<span class="text-emerald-400">+{player.roundScore.toLocaleString()}</span>
											{/if}
											<span class={index === 0 ? 'text-yellow-600' : ''}
												>{player.score.toLocaleString()}</span
											>
										</div>
									</div>
								</div>
							</div>
						</div>
					{/each}

					{#if currentPage < maxPages}
						<div bind:this={loaderElement} class="flex justify-center py-4">
							<div class="text-muted-foreground text-sm">Loading more players...</div>
						</div>
					{/if}
				</div>
			</ScrollArea>
		</div>
	</Card>
</div>
