// src/lib/components/game/lobby-view.svelte
<script lang="ts">
    import { gameStore } from '../../stores/game';
    import { gameActions } from '../../stores/game-actions';
    import { Button } from '$lib/components/ui/button';
    import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';

    // Reactive values from store
    $: players = Array.from($gameStore.players.values());
    $: playerCount = players.length;
    $: isAdmin = $gameStore.isAdmin;
    $: joinCode = $gameStore.joinCode;

    // Handle start game (admin only)
    function handleStartGame() {
        gameActions.startGame();
    }

    // Handle leave game
    function handleLeave() {
        gameActions.leaveGame();
    }
</script>

<div class="container mx-auto p-6 space-y-6">
    <!-- Header with join code and player count -->
    <div class="flex items-center justify-between">
        <div class="flex items-center gap-4">
            <span class="text-xl">🎵</span>
            <h1 class="text-2xl font-bold">Music Quiz</h1>
        </div>
        {#if joinCode}
            <div class="text-lg">
                Join Code: <span class="font-mono font-bold">{joinCode}</span>
            </div>
        {/if}
    </div>

    <!-- Main content -->
    <Card class="border-zinc-800 bg-zinc-900/50">
        <CardHeader>
            <CardTitle>Lobby</CardTitle>
        </CardHeader>
        <CardContent class="space-y-6">
            <!-- Players list -->
            <div class="space-y-4">
                <h3 class="text-lg font-semibold">
                    Players ({playerCount})
                </h3>
                <div class="grid gap-2">
                    {#each players as player (player.name)}
                        <div class="flex items-center justify-between p-3 rounded-lg bg-zinc-800/50">
                            <span class="font-medium">
                                {player.name}
                                {#if player.name === $gameStore.playerName}
                                    <span class="text-zinc-400">(You)</span>
                                {/if}
                            </span>
                            {#if isAdmin && player.name !== 'Admin'}
                                <!-- TODO: Add kick player functionality -->
                                <Button 
                                    variant="ghost" 
                                    size="sm"
                                    class="text-red-500 hover:text-red-400"
                                >
                                    Kick
                                </Button>
                            {/if}
                        </div>
                    {/each}
                </div>
            </div>

            <!-- Admin controls or waiting message -->
            <div class="pt-4 border-t border-zinc-800">
                {#if isAdmin}
                    <div class="space-y-4">
                        <Button 
                            class="w-full"
                            disabled={playerCount < 2}
                            on:click={handleStartGame}
                        >
                            {playerCount < 2 ? 'Waiting for Players...' : 'Start Game'}
                        </Button>
                        <Button 
                            variant="destructive"
                            class="w-full"
                            on:click={() => gameActions.closeGame()}
                        >
                            Close Lobby
                        </Button>
                    </div>
                {:else}
                    <div class="text-center text-zinc-400">
                        Waiting for admin to start the game...
                    </div>
                {/if}
            </div>

            <!-- Leave button -->
            <div class="pt-4">
                <Button 
                    variant="outline"
                    class="w-full border-zinc-800"
                    on:click={handleLeave}
                >
                    Leave Lobby
                </Button>
            </div>
        </CardContent>
    </Card>
</div>
