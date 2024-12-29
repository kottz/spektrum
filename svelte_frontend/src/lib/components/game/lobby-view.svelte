<script lang="ts">
    import { gameStore } from '../../stores/game';
    import { gameActions } from '../../stores/game-actions';
    import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
    import { Button } from '$lib/components/ui/button';

    $: players = Array.from($gameStore.players.values());
    
    function handleLeaveGame() {
        gameActions.leaveGame();
    }
</script>

<div class="container mx-auto max-w-2xl p-6 space-y-6">
    <Card>
        <CardHeader>
            <CardTitle>Waiting for Game to Start</CardTitle>
        </CardHeader>
        <CardContent class="space-y-6">
            <!-- Connected players list -->
            <div class="space-y-2">
                <h3 class="text-sm text-muted-foreground">Connected Players ({players.length})</h3>
                <div class="space-y-1">
                    {#each players as player}
                        <div class="flex items-center justify-between p-2 rounded bg-muted">
                            <span class="font-medium">
                                {player.name}
                                {#if player.name === $gameStore.playerName}
                                    <span class="text-muted-foreground">(You)</span>
                                {/if}
                            </span>
                        </div>
                    {/each}
                </div>
            </div>
            <div class="text-center text-muted-foreground">
                Waiting for admin to start the game...
            </div>
            <!-- Leave button -->
            <Button 
                variant="outline" 
                class="w-full"
                on:click={handleLeaveGame}
            >
                Leave Game
            </Button>
        </CardContent>
    </Card>
</div>
