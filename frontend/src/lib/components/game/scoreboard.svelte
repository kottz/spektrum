<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';

	// Convert reactive statements to derived computations
	const players = $derived(
		Array.from(gameStore.state.players.values()).sort((a, b) => b.score - a.score)
	);
	const maxScore = $derived(players[0]?.score || 0);
	const maxRoundScore = $derived(Math.max(...players.map((p) => p.roundScore), 0));
	const gameOver = $derived(gameStore.state.phase === 'gameover');

	// Helper functions
	function getScoreWidth(score: number): string {
		if (maxScore === 0) return '0%';
		return `${(score / maxScore) * 100}%`;
	}

	function getRoundScoreClass(roundScore: number): string {
		if (roundScore === 0) return 'text-muted-foreground';
		if (roundScore === maxRoundScore) return 'text-green-500 font-bold';
		return 'text-primary';
	}
</script>

<div class="space-y-4">
	<h2 class="mb-8 text-center text-2xl font-bold">Leaderboard</h2>
	<ScrollArea class="h-72 h-[50vh] rounded-md border p-4">
		<div class="space-y-3">
			{#each players as player, i}
				<div class="relative">
					<!-- Background for full width context -->
					<div class="absolute inset-0 rounded-lg bg-muted"></div>
					<!-- Score bar -->
					<div
						class="absolute inset-0 rounded-lg bg-primary/20 transition-all duration-500 ease-out"
						style="width: {getScoreWidth(player.score)}"
					></div>
					<!-- Content -->
					<div class="relative flex items-center justify-between px-4 py-3">
						<div class="flex items-center gap-3">
							<span class="text-muted-foreground">{i + 1}.</span>
							<span class="font-medium">
								{player.name}
								{#if player.name === gameStore.state.playerName}
									<span class="text-muted-foreground">(You)</span>
								{/if}
							</span>
						</div>
						<div class="flex items-center gap-2">
							<!-- Round score with indicator -->
							{#if player.roundScore > 0 && !gameOver}
								<span class={getRoundScoreClass(player.roundScore)}>
									+{player.roundScore}
								</span>
							{/if}
							<!-- Total score -->
							<span class="font-medium">{player.score} pts</span>
						</div>
					</div>
				</div>
			{/each}
		</div>
	</ScrollArea>
</div>
