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
    $: roundAnswers = players.filter(p => p.hasAnswered).length;

    // Determine primary action based on game phase
    function getPrimaryAction(): { text: string; action: () => void; disabled: boolean } {
        switch (phase) {
            case GamePhase.Lobby:
                return {
                    text: 'Start Game',
                    action: () => gameActions.startGame(),
                    disabled: false
                };
            case GamePhase.Score:
                return {
                    text: 'Start Round',
                    action: () => gameActions.startRound(),
                    disabled: false
                };
            case GamePhase.Question:
                return {
                    text: 'End Round',
                    action: () => gameActions.endRound(),
                    disabled: false
                };
            default:
                return {
                    text: 'Waiting...',
                    action: () => {},
                    disabled: true
                };
        }
    }

    $: primaryAction = getPrimaryAction();
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

        <!-- Game flow controls -->
        <div class="space-y-4 pt-4 border-t border-zinc-800">
            <!-- Primary action button (Start Game/Start Round/End Round) -->
            <Button
                class="w-full"
                disabled={primaryAction.disabled}
                on:click={primaryAction.action}
            >
                {primaryAction.text}
            </Button>

            <!-- End game button -->
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
