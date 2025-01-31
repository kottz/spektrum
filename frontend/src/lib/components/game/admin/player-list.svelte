<script lang="ts">
    import { Card } from '$lib/components/ui/card';
    import { ScrollArea } from '$lib/components/ui/scroll-area';
    import { gameStore } from '$lib/stores/game.svelte';

    const players = $derived(Array.from(gameStore.state.players.values()));
    const playerCount = $derived(players.length);
</script>

<Card class="w-full">
    <div class="p-4">
        <h3 class="font-semibold mb-2">Connected Players ({playerCount})</h3>
        <ScrollArea class="h-32">
            <div class="space-y-2">
                {#each players as player}
                    <div class="flex items-center justify-between p-2 bg-muted rounded">
                        <span>{player.name}</span>
                        <span class="text-muted-foreground">{player.score}</span>
                    </div>
                {/each}
            </div>
        </ScrollArea>
    </div>
</Card>
