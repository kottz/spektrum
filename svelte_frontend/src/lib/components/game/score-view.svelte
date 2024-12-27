<script lang="ts">
    import { gameStore } from '../../stores/game';
    import { gameActions } from '../../stores/game-actions';
    import { Button } from '$lib/components/ui/button';
    import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
    import { Progress } from '$lib/components/ui/progress';

    // Get sorted players list for scoreboard
    $: scoreboard = $gameStore.scoreboard;
    $: maxScore = scoreboard[0]?.score ?? 0;
    
    // Get current player info
    $: currentPlayer = $gameStore.currentPlayer;
    $: isAdmin = $gameStore.isAdmin;

    // Calculate positions and score changes
    function getScoreProgress(score: number): number {
        return maxScore > 0 ? (score / maxScore) * 100 : 0;
    }

    function getPositionLabel(index: number): string {
        if (index === 0) return "👑";
        if (index === 1) return "🥈";
        if (index === 2) return "🥉";
        return `${index + 1}.`;
    }
</script>

<div class="container mx-auto p-6 space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
        <div class="flex items-center gap-4">
            <span class="text-xl">🎵</span>
            <h1 class="text-2xl font-bold">Music Quiz</h1>
        </div>
        <div class="text-lg font-medium">
            Round Results
        </div>
    </div>

    <!-- Scoreboard card -->
    <Card class="border-zinc-800 bg-zinc-900/50">
        <CardHeader>
            <CardTitle>Scoreboard</CardTitle>
        </CardHeader>
        <CardContent class="space-y-6">
            <!-- Player scores -->
            <div class="space-y-4">
                {#each scoreboard as player, index}
                    <div class="space-y-2">
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <span class="w-8 text-zinc-400">
                                    {getPositionLabel(index)}
                                </span>
                                <span class="font-medium">
                                    {player.name}
                                    {#if player.name === $gameStore.playerName}
                                        <span class="text-zinc-400">(You)</span>
                                    {/if}
                                </span>
                            </div>
                            <div class="flex items-center gap-2">
                                <span class="font-bold">
                                    {player.score}
                                </span>
                                {#if player.hasAnswered}
                                    <span class={player.answer === $gameStore.currentQuestion?.correct_answer 
                                        ? "text-emerald-500" 
                                        : "text-red-500"}>
                                        {player.answer === $gameStore.currentQuestion?.correct_answer ? "✓" : "✗"}
                                    </span>
                                {/if}
                            </div>
                        </div>
                        <Progress 
                            value={getScoreProgress(player.score)}
                            class="h-2 bg-zinc-800"
                            indicatorClass={player.name === $gameStore.playerName 
                                ? "bg-blue-500" 
                                : undefined}
                        />
                    </div>
                {/each}
            </div>

            <!-- Admin controls or waiting message -->
            <div class="pt-4 border-t border-zinc-800">
                {#if isAdmin}
                    <div class="space-y-4">
                        <Button 
                            class="w-full"
                            on:click={() => gameActions.startRound()}
                        >
                            Next Round
                        </Button>
                        <Button 
                            variant="destructive"
                            class="w-full"
                            on:click={() => gameActions.endGame()}
                        >
                            End Game
                        </Button>
                    </div>
                {:else}
                    <div class="text-center text-zinc-400">
                        Waiting for next round...
                    </div>
                {/if}
            </div>

            <!-- Correct answer display -->
            {#if $gameStore.currentQuestion?.correct_answer}
                <div class="pt-4 border-t border-zinc-800">
                    <div class="text-center">
                        <span class="text-zinc-400">Correct answer was: </span>
                        <span class="font-bold">
                            {$gameStore.currentQuestion.correct_answer}
                        </span>
                    </div>
                </div>
            {/if}
        </CardContent>
    </Card>
</div>
