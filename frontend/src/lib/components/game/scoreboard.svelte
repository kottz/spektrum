<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import { Button } from '$lib/components/ui/button';
	import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
	import { gameActions } from '$lib/stores/game-actions';

	const PAGE_SIZE = 25;

	const allSortedPlayers = $derived(
		Array.from(gameStore.state.players.values()).sort((a, b) => b.score - a.score)
	);

	let currentPage = $state(1);
	const totalPlayers = $derived(allSortedPlayers.length);
	const maxPages = $derived(Math.ceil(totalPlayers / PAGE_SIZE));

	// The subset of players to be rendered in the DOM.
	const renderedPlayers = $derived(allSortedPlayers.slice(0, currentPage * PAGE_SIZE));
	let loaderElement: HTMLDivElement | undefined = $state();

	const isAdmin = $derived(gameStore.state.isAdmin);
	const myName = $derived(gameStore.state.playerName);
	const maxScore = $derived(allSortedPlayers[0]?.score || 0);
	const maxRoundScore = $derived(
		Math.max(...allSortedPlayers.map((p) => p.roundScore).filter(isFinite), 0)
	);

	const gameOver = $derived(gameStore.state.phase === 'gameover');
	const playingQuestion = $derived(gameStore.state.phase === 'question');

	function getScoreWidth(score: number): string {
		if (maxScore === 0) return '0%';
		return `${(score / maxScore) * 100}%`;
	}

	function getRoundScoreClass(roundScore: number): string {
		if (roundScore === 0) return 'text-muted-foreground';
		// Highlight if it's the max round score AND that score isn't zero
		if (roundScore === maxRoundScore && maxRoundScore !== 0)
			return 'text-emerald-700 dark:text-emerald-400 font-bold';
		return 'text-primary-foreground';
	}

	function kickPlayer(playerName: string) {
		gameActions.kickPlayer(playerName);
	}

	$effect(() => {
		const currentLoaderEl = loaderElement;

		if (!currentLoaderEl) {
			return;
		}

		// Find the Radix ScrollArea Viewport to use as the observer's root.
		// This ensures observations are relative to the ScrollArea's scrolling, not the browser window.
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

<div class="flex h-full min-h-0 w-full">
	<ScrollArea class="min-h-0 w-1 flex-1 rounded-md">
		<div class="space-y-3 p-4">
			{#each renderedPlayers as player, i (player.name)}
				<div class="relative">
					<div class="absolute inset-0 rounded-lg"></div>
					<div
						class="bg-primary/20 absolute inset-0 rounded-lg transition-all duration-500 ease-out"
						style="width: {getScoreWidth(player.score)}"
					></div>
					<div class="relative flex items-center justify-between px-4 py-2">
						<div class="flex items-center gap-3">
							<span class="text-muted-foreground">{i + 1}.</span>
							<span class="font-medium">
								{player.name}
								{#if player.name === myName}
									<span class="text-muted-foreground ml-1 text-xs">(You)</span>
								{/if}
								{#if player.consecutiveMisses >= 3}
									<span class="text-muted-foreground ml-1 text-xs"
										>(AFK: {player.consecutiveMisses})</span
									>
								{/if}
							</span>
						</div>
						<div class="flex items-center gap-2">
							{#if !gameOver && player.roundScore !== 0}
								<span class={getRoundScoreClass(player.roundScore)}>
									{player.roundScore > 0 ? '+' : ''}{player.roundScore}
								</span>
							{/if}
							<span class="font-medium">{player.score}</span>
							{#if isAdmin && player.name !== myName && !gameOver && !playingQuestion}
								<Button
									variant="destructive"
									size="sm"
									class="ml-2 h-6 px-2"
									type="button"
									onclick={() => kickPlayer(player.name)}
									aria-label={`Kick ${player.name}`}
								>
									Kick
								</Button>
							{/if}
						</div>
					</div>
				</div>
			{/each}

			{#if currentPage < maxPages}
				<div bind:this={loaderElement} class="h-1 w-full" aria-hidden="true">
					<!--Invisible loader element. -->
				</div>
			{/if}
		</div>
	</ScrollArea>
</div>
