<script lang="ts">
    import { gameStore } from '../../stores/game';
    import { gameActions } from '../../stores/game-actions';
    import { Button } from '$lib/components/ui/button';
    import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
    import { GamePhase } from '../../types/game';

    // Reactive values from store
    $: phase = $gameStore.phase;
    $: players = Array.from($gameStore.players.values());
    $: playerCount = players.length;
    $: currentQuestion = $gameStore.currentQuestion;
    $: roundAnswers = players.filter(p => p.hasAnswered).length;

    function getPhaseAction(): { text: string; action: () => void } {
        switch (phase) {
            case GamePhase.Lobby:
                return {
                    text: playerCount < 2 ? 'Waiting for Players...' : 'Start Game',
                    action: () => gameActions.startGame()
                };
            case GamePhase.Question:
                return {
                    text: 'End Round',
                    action: () => gameActions.endRound()
                };
            case GamePhase.Score:
                return {
                    text: 'Start Next Round',
                    action: () => gameActions.startRound()
                };
            default:
                return {
                    text: 'Unknown Phase',
                    action: () => {}
                };
        }
    }

    $: phaseAction = getPhaseAction();
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

            {#if phase === GamePhase.Question}
                <div class="flex justify-between text-sm">
                    <span class="text-zinc-400">Answers</span>
                    <span class="font-medium">{roundAnswers}/{playerCount}</span>
                </div>
            {/if}
        </div>

        <!-- Phase-specific controls -->
        <div class="space-y-4 pt-4 border-t border-zinc-800">
            <!-- Main phase action button -->
            <Button
                class="w-full"
                disabled={phase === GamePhase.Lobby && playerCount < 2}
                on:click={phaseAction.action}
            >
                {phaseAction.text}
            </Button>

            <!-- Skip question button (only during question phase) -->
            {#if phase === GamePhase.Question}
                <Button
                    variant="outline"
                    class="w-full border-zinc-800"
                    on:click={() => gameActions.skipQuestion()}
                >
                    Skip Question
                </Button>
            {/if}

            <!-- End game button (not in game over phase) -->
            {#if phase !== GamePhase.GameOver}
                <Button
                    variant="destructive"
                    class="w-full"
                    on:click={() => gameActions.endGame()}
                >
                    End Game
                </Button>
            {/if}

            <!-- Close lobby button (always available) -->
            <Button
                variant="destructive"
                class="w-full"
                on:click={() => gameActions.closeGame()}
            >
                Close Lobby
            </Button>
        </div>
    </CardContent>
</Card>
