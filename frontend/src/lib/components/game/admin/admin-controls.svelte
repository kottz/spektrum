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

<div class="border-t bg-background">
    <div class="space-y-3 p-4">
        <!-- Top Row -->
        <div class="grid grid-cols-2 gap-3">
            {#if !isGameOver}
                <Button
                    variant={isGameRunning ? 'destructive' : 'default'}
                    class="w-full"
                    onclick={() => (isGameRunning ? gameActions.endGame() : gameActions.startGame())}
                >
                    {isGameRunning ? 'End Game' : 'Start Game'}
                </Button>
            {:else}
                <!-- Placeholder to keep grid layout consistent -->
                <div></div>
            {/if}
            {#if isGameRunning}
                <Button
                    class="w-full"
                    disabled={outOfQuestions}
                    onclick={() => (isInQuestion ? gameActions.endRound() : gameActions.startRound())}
                >
                    {isInQuestion ? 'End Round' : 'Start Round'}
                </Button>
            {:else}
                <div></div>
            {/if}
        </div>
        <!-- Bottom Row -->
        <div class="grid grid-cols-2 gap-3">
            <Button
                variant="destructive"
                class="w-full"
                onclick={() => (isGameOver ? gameActions.leaveGame() : gameActions.closeGame())}
            >
                {isGameOver ? 'Leave Lobby' : 'Close Lobby'}
            </Button>
            {#if isGameRunning}
                <Button
                    variant="outline"
                    class="w-full"
                    disabled={isInQuestion || !isGameRunning || outOfQuestions}
                    on:click={() => gameActions.skipQuestion()}
                >
                    Skip Question
                </Button>
            {:else}
                <div></div>
            {/if}
        </div>
    </div>
</div>
