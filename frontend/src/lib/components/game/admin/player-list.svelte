<script lang="ts">
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { gameStore } from '$lib/stores/game.svelte';

	const players = $derived(Array.from(gameStore.state.players.values()));
	const playerCount = $derived(players.length);
</script>

<div class="flex h-full w-full flex-col">
	<h3 class="mb-2 font-semibold">Connected Players ({playerCount})</h3>
	<ScrollArea class="flex-1 rounded-md">
		<div class="flex flex-wrap gap-2">
			{#each players as player}
				<div class="bg-muted flex items-center rounded p-2">
					<span class="font-medium">
						{player.name}
					</span>
				</div>
			{/each}
		</div>
	</ScrollArea>
</div>
