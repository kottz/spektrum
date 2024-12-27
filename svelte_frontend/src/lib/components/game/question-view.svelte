// src/lib/components/game/question-view.svelte
<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { gameStore } from '../../stores/game';
    import { gameActions } from '../../stores/game-actions';
    import { Button } from '$lib/components/ui/button';
    import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
    import { Progress } from '$lib/components/ui/progress';

    // Timer state
    let timeLeft = $gameStore.roundDuration;
    let timerInterval: ReturnType<typeof setInterval>;

    // Get current player's state
    $: currentPlayer = $gameStore.currentPlayer;
    $: hasAnswered = currentPlayer?.hasAnswered ?? false;
    $: selectedAnswer = currentPlayer?.answer;

    // Get question information
    $: question = $gameStore.currentQuestion;
    $: alternatives = question?.alternatives ?? [];

    // Get player answer statistics
    $: players = Array.from($gameStore.players.values());
    $: answeredCount = $gameStore.answeredCount;
    $: totalPlayers = players.length;
    $: answeredPercentage = (answeredCount / totalPlayers) * 100;

    // Start timer when component mounts
    onMount(() => {
        timeLeft = $gameStore.roundDuration;
        timerInterval = setInterval(() => {
            timeLeft = Math.max(0, timeLeft - 1);
            if (timeLeft === 0) {
                clearInterval(timerInterval);
            }
        }, 1000);
    });

    // Cleanup timer when component unmounts
    onDestroy(() => {
        if (timerInterval) clearInterval(timerInterval);
    });

    function handleAnswer(answer: string) {
        if (!hasAnswered) {
            gameActions.submitAnswer(answer);
        }
    }

    // Function to get style for answer button
    function getAnswerButtonStyle(answer: string): string {
        if (!hasAnswered) return "bg-zinc-800/50 hover:bg-zinc-700/50";
        if (answer === selectedAnswer) return "bg-blue-500/50 border-blue-500";
        return "bg-zinc-800/50 opacity-50";
    }
</script>

<div class="container mx-auto p-6 space-y-6">
    <!-- Header with timer -->
    <div class="flex items-center justify-between">
        <div class="flex items-center gap-4">
            <span class="text-xl">🎵</span>
            <h1 class="text-2xl font-bold">Music Quiz</h1>
        </div>
        <div class="flex items-center gap-2">
            <span class="text-zinc-400">⏱️</span>
            <span class="text-lg font-medium">
                {timeLeft}s
            </span>
        </div>
    </div>

    <!-- Main question card -->
    <Card class="border-zinc-800 bg-zinc-900/50">
        <CardHeader>
            <CardTitle>Choose your answer</CardTitle>
        </CardHeader>
        <CardContent class="space-y-6">
            <!-- Answer grid -->
            <div class="grid grid-cols-2 gap-4">
                {#each alternatives as answer}
                    <button
                        class="aspect-square rounded-lg border-2 border-transparent transition-all duration-200 {getAnswerButtonStyle(answer)}"
                        disabled={hasAnswered}
                        on:click={() => handleAnswer(answer)}
                    >
                        <div class="h-full w-full flex items-center justify-center text-lg font-medium">
                            {answer}
                        </div>
                    </button>
                {/each}
            </div>

            <!-- Answer progress -->
            <div class="space-y-2">
                <div class="flex justify-between text-sm text-zinc-400">
                    <span>Answers</span>
                    <span>{answeredCount}/{totalPlayers}</span>
                </div>
                <Progress 
                    value={answeredPercentage} 
                    class="h-2 bg-zinc-800"
                />
            </div>

            <!-- Player answers list -->
            <div class="flex flex-wrap gap-2">
                {#each players as player}
                    {#if player.hasAnswered}
                        <div class="px-2 py-1 rounded bg-zinc-800 text-sm">
                            {player.name}
                            {#if player.name === $gameStore.playerName}
                                <span class="text-zinc-400">(You)</span>
                            {/if}
                        </div>
                    {/if}
                {/each}
            </div>

            <!-- Admin controls -->
            {#if $gameStore.isAdmin}
                <div class="pt-4 border-t border-zinc-800">
                    <Button
                        variant="destructive"
                        class="w-full"
                        on:click={() => gameActions.endRound()}
                    >
                        End Round
                    </Button>
                </div>
            {/if}
        </CardContent>
    </Card>
</div>
