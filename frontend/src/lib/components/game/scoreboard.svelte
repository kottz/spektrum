<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import { Button } from '$lib/components/ui/button';
	import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
	import { gameActions } from '$lib/stores/game-actions';

	const players = $derived(
		Array.from(gameStore.state.players.values()).sort((a, b) => b.score - a.score)
	);

	const isAdmin = $derived(gameStore.state.isAdmin);
	const myName = $derived(gameStore.state.playerName);
	const maxScore = $derived(players[0]?.score || 0);
	const maxRoundScore = $derived(Math.max(...players.map((p) => p.roundScore), 0));
	const gameOver = $derived(gameStore.state.phase === 'gameover');
	const playingQuestion = $derived(gameStore.state.phase === 'question');

	function getScoreWidth(score: number): string {
		if (maxScore === 0) return '0%';
		return `${(score / maxScore) * 100}%`;
	}

	function getRoundScoreClass(roundScore: number): string {
		if (roundScore === 0) return 'text-muted-foreground';
		if (roundScore === maxRoundScore) return 'text-emerald-700 dark:text-emerald-400 font-bold';
		return 'text-muted-white';
	}

	function kickPlayer(playerName: string) {
		gameActions.kickPlayer(playerName);
	}
</script>

<div class="flex h-full min-h-0 w-full">
	<ScrollArea class="min-h-0 w-1 flex-1 rounded-md">
		<div class="space-y-3 p-4">
			{#each players as player, i}
				<div class="relative">
					<div class="absolute inset-0 rounded-lg"></div>
					<div
						class="absolute inset-0 rounded-lg bg-primary/20 transition-all duration-500 ease-out"
						style="width: {getScoreWidth(player.score)}"
					></div>
					<div class="relative flex items-center justify-between px-4 py-2">
						<div class="flex items-center gap-3">
							<span class="text-muted-foreground">{i + 1}.</span>
							<span class="font-medium">
								{player.name}
								{#if player.name === myName}
									<span class="ml-1 text-xs text-muted-foreground">(You)</span>
								{/if}
								{#if player.consecutiveMisses >= 3}
									<span class="ml-1 text-xs text-muted-foreground"
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
							<!-- Add Kick Button Here -->
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
		</div>
	</ScrollArea>
</div>
