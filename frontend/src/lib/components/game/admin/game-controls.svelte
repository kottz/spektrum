<script lang="ts">
    import { Button } from '$lib/components/ui/button';
    import { gameStore } from '$lib/stores/game.svelte';
    import { gameActions } from '$lib/stores/game-actions';

    const phase = $derived(gameStore.state.phase?.toLowerCase() || 'lobby');
    const isGameRunning = $derived(phase !== 'lobby' && phase !== 'gameover');
    const isInQuestion = $derived(phase === 'question');
    const isGameOver = $derived(phase === 'gameover');
    const outOfQuestions = $derived(gameStore.state.upcomingQuestions?.length === 0);
</script>

<div class="fixed bottom-0 left-0 right-0 bg-background border-t">
    <div class="container max-w-md mx-auto p-4 space-y-3">
        <!-- Primary Actions -->
        <div class="grid grid-cols-2 gap-3">
            <Button
                variant="destructive"
                class="w-full"
                onclick={() => (isGameOver ? gameActions.leaveGame() : gameActions.closeGame())}
            >
                {isGameOver ? 'Leave Lobby' : 'Close Lobby'}
            </Button>
            
            {#if !isGameOver}
                <Button
                    variant={isGameRunning ? 'destructive' : 'default'}
                    class="w-full"
                    onclick={() => (isGameRunning ? gameActions.endGame() : gameActions.startGame())}
                >
                    {isGameRunning ? 'End Game' : 'Start Game'}
                </Button>
            {/if}
        </div>

        <!-- Secondary Actions -->
        {#if isGameRunning}
            <div class="grid grid-cols-2 gap-3">
                <Button
                    class="w-full"
                    disabled={outOfQuestions}
                    onclick={() => (isInQuestion ? gameActions.endRound() : gameActions.startRound())}
                >
                    {isInQuestion ? 'End Round' : 'Start Round'}
                </Button>
                <Button
                    variant="outline"
                    class="w-full"
                    disabled={isInQuestion || outOfQuestions}
                    onclick={gameActions.skipQuestion}
                >
                    Skip Question
                </Button>
            </div>
        {/if}
    </div>
</div>
