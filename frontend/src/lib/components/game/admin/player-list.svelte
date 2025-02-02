<script lang="ts">
	import { Card } from '$lib/components/ui/card';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { gameStore } from '$lib/stores/game.svelte';

	const players = $derived(Array.from(gameStore.state.players.values()));
	const playerCount = $derived(players.length);
</script>

<Card class="h-full w-full">
	<div class="flex h-full flex-col p-4">
		<h3 class="mb-2 font-semibold">Connected Players ({playerCount})</h3>
		<ScrollArea type="always" class="flex-1 rounded-md border">
			<div class="space-y-2">
				{#each players as player}
					<div class="flex items-center justify-between rounded bg-muted p-2">
						<span>{player.name}</span>
						<span class="text-muted-foreground">{player.score}</span>
					</div>
				{/each}
			</div>
		</ScrollArea>
	</div>
</Card>
