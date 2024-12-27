// src/lib/components/game/game-over.svelte
<script lang="ts">
    import { gameStore } from '../../stores/game';
    import { gameActions } from '../../stores/game-actions';
    import { Button } from '$lib/components/ui/button';
    import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
    import { Progress } from '$lib/components/ui/progress';

    // Get sorted final scores
    $: scoreboard = $gameStore.scoreboard;
    $: winner = scoreboard[0];
    $: maxScore = winner?.score ?? 0;
    $: isAdmin = $gameStore.isAdmin;
    $: currentPlayer = $gameStore.currentPlayer;
    $: playerPosition = scoreboard.findIndex(p => p.name === currentPlayer?.name) + 1;

    function getPositionEmoji(position: number): string {
        switch (position) {
            case 1: return "👑";
            case 2: return "🥈";
            case 3: return "🥉";
            default: return `${position}.`;
        }
    }

    function getScoreProgress(score: number): number {
        return maxScore > 0 ? (score / maxScore) * 100 : 0;
    }
</script>

<div class="container mx-auto p-6 space-y-6">
    <!-- Header with game over announcement -->
    <div class="flex items-center justify-between">
        <div class="flex items-center gap-4">
            <span class="text-xl">🎵</span>
            <h1 class="text-2xl font-bold">Music Quiz</h1>
        </div>
        <div class="text-lg font-medium text-zinc-400">
            Game Over
        </div>
    </div>

    <!-- Winner announcement card -->
    <Card class="border-zinc-800 bg-zinc-900/50">
        <CardHeader>
            <CardTitle class="text-center">
                {#if winner}
                    {#if winner.name === currentPlayer?.name}
                        🎉 You Won! 🎉
                    {:else}
                        Winner: {winner.name}
                    {/if}
                {/if}
            </CardTitle>
        </CardHeader>
        <CardContent>
            {#if currentPlayer}
                <div class="text-center text-zinc-400 mb-6">
                    You finished {playerPosition}#{playerPosition === 1 ? '!' : ''}
                </div>
            {/if}
        </CardContent>
    </Card>

    <!-- Final scoreboard card -->
    <Card class="border-zinc-800 bg-zinc-900/50">
        <CardHeader>
            <CardTitle>Final Scores</CardTitle>
        </CardHeader>
        <CardContent class="space-y-6">
            <!-- Player scores -->
            <div class="space-y-4">
                {#each scoreboard as player, index}
                    <div class="space-y-2">
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <span class="w-8 text-zinc-400">
                                    {getPositionEmoji(index + 1)}
                                </span>
                                <span class="font-medium">
                                    {player.name}
                                    {#if player.name === currentPlayer?.name}
                                        <span class="text-zinc-400">(You)</span>
                                    {/if}
                                </span>
                            </div>
                            <span class="font-bold">
                                {player.score}
                            </span>
                        </div>
                        <Progress 
                            value={getScoreProgress(player.score)}
                            class="h-2 bg-zinc-800"
                            indicatorClass={player.name === currentPlayer?.name 
                                ? "bg-blue-500" 
                                : undefined}
                        />
                    </div>
                {/each}
            </div>

            <!-- Action buttons -->
            <div class="pt-4 border-t border-zinc-800 space-y-4">
                {#if isAdmin}
                    <Button 
                        class="w-full"
                        on:click={() => gameActions.startGame()}
                    >
                        Play Again
                    </Button>
                    <Button 
                        variant="destructive"
                        class="w-full"
                        on:click={() => gameActions.closeGame()}
                    >
                        Close Lobby
                    </Button>
                {:else}
                    <div class="text-center text-zinc-400 mb-4">
                        Waiting for admin to start a new game...
                    </div>
                    <Button 
                        variant="outline"
                        class="w-full border-zinc-800"
                        on:click={() => gameActions.leaveGame()}
                    >
                        Leave Lobby
                    </Button>
                {/if}
            </div>
        </CardContent>
    </Card>
</div>
