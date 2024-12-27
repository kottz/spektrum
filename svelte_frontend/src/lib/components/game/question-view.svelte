<script lang="ts">
    import { gameStore } from '../../stores/game';
    import { gameActions } from '../../stores/game-actions';
    import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';

    // Get current game state
    $: alternatives = $gameStore.currentQuestion?.alternatives || [];
    $: currentPlayer = $gameStore.playerName ? 
        $gameStore.players.get($gameStore.playerName) : undefined;
    $: hasAnswered = currentPlayer?.hasAnswered || false;
    $: timeRemaining = $gameStore.roundDuration;

    function handleAnswer(answer: string) {
        if (!hasAnswered) {
            gameActions.submitAnswer(answer);
        }
    }
</script>

<div class="container mx-auto max-w-2xl p-6">
    <Card class="border-zinc-800 bg-zinc-900/50">
        <CardHeader>
            <CardTitle class="flex justify-between items-center">
                <span>Choose your answer</span>
                <span class="text-zinc-400">{timeRemaining}s</span>
            </CardTitle>
        </CardHeader>
        <CardContent>
            <div class="grid grid-cols-2 gap-4">
                {#each alternatives as alternative}
                    <button
                        class="aspect-square rounded-lg border-2 border-transparent p-4 
                               {hasAnswered ? 
                                   (currentPlayer?.answer === alternative ? 
                                       'bg-blue-500/20 border-blue-500' : 
                                       'bg-zinc-800/50 opacity-50') : 
                                   'bg-zinc-800/50 hover:bg-zinc-700/50'}"
                        disabled={hasAnswered}
                        on:click={() => handleAnswer(alternative)}
                    >
                        <div class="h-full w-full flex items-center justify-center text-lg font-medium">
                            {alternative}
                        </div>
                    </button>
                {/each}
            </div>
        </CardContent>
    </Card>
</div>
