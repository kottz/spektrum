<script lang="ts">
    import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
    import Scoreboard from './scoreboard.svelte';
    import { gameStore } from '../../stores/game';

    $: players = Array.from($gameStore.players.values())
        .sort((a, b) => b.score - a.score);
    $: winner = players[0];
    $: currentPlayer = $gameStore.playerName ? 
        $gameStore.players.get($gameStore.playerName) : undefined;
</script>

<div class="space-y-6">
    <!-- Winner announcement -->
    <Card class="border-zinc-800 bg-zinc-900/50">
        <CardHeader>
            <CardTitle class="text-center">Game Over!</CardTitle>
        </CardHeader>
        <CardContent>
            <div class="text-center space-y-2">
                {#if winner}
                    <div class="text-2xl font-bold">
                        {#if winner.name === $gameStore.playerName}
                            🎉 You Won! 🎉
                        {:else}
                            Winner: {winner.name}
                        {/if}
                    </div>
                    <div class="text-zinc-400">
                        Final Score: {winner.score} points
                    </div>
                {/if}
            </div>
        </CardContent>
    </Card>

    <!-- Final Scoreboard -->
    <Card class="border-zinc-800 bg-zinc-900/50">
        <CardHeader>
            <CardTitle>Final Scores</CardTitle>
        </CardHeader>
        <CardContent>
            <Scoreboard />
        </CardContent>
    </Card>
</div>
