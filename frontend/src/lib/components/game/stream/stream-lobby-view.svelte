<script lang="ts">
	import { streamStore } from '$lib/stores/stream.store.svelte';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';

	const gameState = $derived(streamStore.state.gameState);
	const players = $derived(gameState?.realtimeScoreboard || []);
</script>

<div class="mx-auto max-w-4xl space-y-8 p-8">
	<Card>
		<CardHeader class="text-center">
			<CardTitle class="text-5xl font-bold">Waiting for Game to Start</CardTitle>
		</CardHeader>
		<CardContent class="space-y-8">
			<!-- Connected players list -->
			<div class="space-y-4">
				<h3 class="text-muted-foreground text-center text-2xl font-semibold">
					Connected Players ({players.length})
				</h3>
				<ScrollArea class="h-96 rounded-md border p-6">
					<div class="flex flex-wrap gap-3">
						{#each players as player (player.name)}
							<div class="bg-muted flex items-center rounded-lg p-4">
								<span class="text-xl font-semibold">
									{player.name}
								</span>
							</div>
						{/each}
					</div>
				</ScrollArea>
			</div>

			<div class="text-center">
				<p class="text-muted-foreground text-2xl">Waiting for admin to start the game...</p>
			</div>
		</CardContent>
	</Card>
</div>
