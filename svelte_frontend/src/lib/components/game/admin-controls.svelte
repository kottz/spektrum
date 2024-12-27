<script lang="ts">
    import { gameStore } from '../../stores/game';
    import { gameActions } from '../../stores/game-actions';
    import { Button } from '$lib/components/ui/button';
    import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';

    // Reactive values from store
    $: phase = $gameStore.phase?.toLowerCase() || 'lobby';
    $: players = Array.from($gameStore.players.values());
    $: playerCount = players.length;
    $: roundAnswers = players.filter(p => p.hasAnswered).length;
    
    // Game state checks
    $: isGameRunning = phase !== 'lobby' && phase !== 'gameover';
    $: isInQuestion = phase === 'question';
    $: isGameOver = phase === 'gameover';
</script>

<Card class="border-zinc-800 bg-zinc-900/50">
    <CardHeader>
        <CardTitle>Admin Controls</CardTitle>
    </CardHeader>
    <CardContent class="space-y-6">
        <!-- Current game status -->
        <div class="space-y-2">
            <div class="flex justify-between text-sm">
                <span class="text-zinc-400">Current Phase</span>
                <span class="font-medium">{phase}</span>
            </div>
            
            <!-- Players list -->
            <div class="space-y-2">
                <div class="flex justify-between text-sm">
                    <span class="text-zinc-400">Players ({playerCount})</span>
                </div>
                <div class="space-y-1">
                    {#each players as player}
                        <div class="flex items-center justify-between p-2 rounded bg-zinc-800/50 text-sm">
                            <span>{player.name}</span>
                            <span class="text-zinc-400">{player.score}</span>
                        </div>
                    {/each}
                </div>
            </div>

            {#if isInQuestion}
                <div class="flex justify-between text-sm">
                    <span class="text-zinc-400">Answers</span>
                    <span class="font-medium">{roundAnswers}/{playerCount}</span>
                </div>
            {/if}
        </div>

        <!-- Game flow controls -->
        <div class="space-y-4 pt-4 border-t border-zinc-800">
            {#if !isGameOver}
                <!-- Game control -->
                <Button
                    class="w-full"
                    variant={isGameRunning ? "destructive" : "default"}
                    on:click={() => isGameRunning ? gameActions.endGame() : gameActions.startGame()}
                >
                    {isGameRunning ? 'End Game' : 'Start Game'}
                </Button>

                <!-- Round control - only shown when game is running -->
                {#if isGameRunning}
                    <Button
                        class="w-full"
                        on:click={() => isInQuestion ? gameActions.endRound() : gameActions.startRound()}
                    >
                        {isInQuestion ? 'End Round' : 'Start Round'}
                    </Button>
                {/if}

                <!-- Close game button -->
                <Button
                    variant="destructive"
                    class="w-full"
                    on:click={() => gameActions.closeGame()}
                >
                    Close Lobby
                </Button>
            {:else}
                <!-- Leave button - only button shown when game is closed -->
                <Button
                    variant="default"
                    class="w-full"
                    on:click={() => gameActions.leaveGame()}
                >
                    Leave Lobby
                </Button>
            {/if}
        </div>
    </CardContent>
</Card>
