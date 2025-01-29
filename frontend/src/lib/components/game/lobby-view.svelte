<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import { gameActions } from '../../stores/game-actions';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';

	$: players = Array.from(gameStore.state.players.values());

	function handleLeaveGame() {
		gameActions.leaveGame();
	}
</script>

<div class="container mx-auto max-w-2xl space-y-6 p-6">
	<Card>
		<CardHeader>
			<CardTitle>Waiting for Game to Start</CardTitle>
		</CardHeader>
		<CardContent class="space-y-6">
			<!-- Connected players list -->
			<div class="space-y-2">
				<h3 class="text-sm text-muted-foreground">Connected Players ({players.length})</h3>
				<ScrollArea class="h-72 h-[50vh] rounded-md border p-4">
					<div class="flex flex-wrap gap-2">
						{#each players as player}
							<div class="flex items-center rounded bg-muted p-2">
								<span class="font-medium">
									{player.name}
									{#if player.name === gameStore.state.playerName}
										<span class="text-muted-foreground">(You)</span>
									{/if}
								</span>
							</div>
						{/each}
					</div>
				</ScrollArea>
			</div>
			<div class="text-center text-muted-foreground">Waiting for admin to start the game...</div>
			<!-- Leave button -->
			<Button variant="outline" class="w-full" on:click={handleLeaveGame}>Leave Game</Button>
		</CardContent>
	</Card>
</div>
