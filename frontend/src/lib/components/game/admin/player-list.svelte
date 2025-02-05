<script lang="ts">
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { gameStore } from '$lib/stores/game.svelte';

	const players = $derived(Array.from(gameStore.state.players.values()));
	const playerCount = $derived(players.length);
</script>

<div class="flex h-full w-full flex-col">
	<h3 class="mb-2 font-semibold">Connected Players ({playerCount})</h3>
	<ScrollArea type="always" class="flex-1 rounded-md border">
		<div class="flex flex-wrap gap-2">
			{#each players as player}
				<div class="flex items-center rounded bg-muted p-2">
					<span class="font-medium">
						{player.name}
					</span>
				</div>
			{/each}
		</div>
	</ScrollArea>
</div>
