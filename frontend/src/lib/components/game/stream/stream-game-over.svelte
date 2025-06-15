<script lang="ts">
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { streamStore } from '$lib/stores/stream.store.svelte';
	import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';

	const gameState = $derived(streamStore.state.gameState);
	const players = $derived(gameState?.realtimeScoreboard?.sort((a, b) => b.score - a.score) || []);
	const winner = $derived(players[0]);
</script>

<div class="container mx-auto max-w-6xl p-8">
	<!-- Winner announcement -->
	<Card class="mb-12">
		<div class="px-6 py-8 text-center">
			<h1 class="text-6xl font-bold">🎉 Game Over! 🎉</h1>
			{#if winner}
				<div class="mb-3 text-5xl font-bold">
					Winner: {winner.name}
				</div>
				<div class="text-3xl text-muted-foreground">
					Final Score: {winner.score.toLocaleString()} points
				</div>
			{/if}
		</div>
	</Card>

	<!-- Final Scoreboard -->
	<Card>
		<div class="px-6 py-6">
			<h2 class="mb-6 text-center text-4xl font-bold">Final Leaderboard</h2>
			<ScrollArea class="h-96">
				<div>
					{#each players as player, index (player.name)}
						<div
							class="relative mb-3 overflow-hidden rounded-md {index === 0
								? 'border-2 border-yellow-500 bg-yellow-500/20'
								: 'bg-muted/30'}"
						>
							<!-- Background score bar for visual appeal -->
							{#if players[0]?.score > 0}
								<div
									class="absolute inset-0 bg-primary/10 transition-all duration-500"
									style="width: {(player.score / players[0].score) * 100}%"
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
				</div>
			</ScrollArea>
		</div>
	</Card>
</div>
