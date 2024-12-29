<script lang="ts">
    import { gameStore } from '../../stores/game';

    // Get sorted players and calculate highest score
    $: players = Array.from($gameStore.players.values())
        .sort((a, b) => b.score - a.score);
    $: maxScore = players[0]?.score || 0;

    // Calculate width percentage based on highest score
    function getScoreWidth(score: number): string {
        if (maxScore === 0) return '0%';
        return `${(score / maxScore) * 100}%`;
    }
</script>

<div class="space-y-4">
    <h2 class="text-2xl font-bold text-center mb-8">Leaderboard</h2>
    
    <div class="space-y-3">
        {#each players as player, i}
            <div class="relative">
                <!-- Background for full width context -->
                <div class="absolute inset-0 bg-muted rounded-lg" />
                
                <!-- Score bar -->
                <div 
                    class="absolute inset-0 bg-primary/20 rounded-lg transition-all duration-500 ease-out" 
                    style="width: {getScoreWidth(player.score)}"
                />
                
                <!-- Content -->
                <div class="relative px-4 py-3 flex justify-between items-center">
                    <div class="flex items-center gap-3">
                        <span class="text-muted-foreground">{i + 1}.</span>
                        <span class="font-medium">
                            {player.name}
                            {#if player.name === $gameStore.playerName}
                                <span class="text-muted-foreground">(You)</span>
                            {/if}
                        </span>
                    </div>
                    <span class="font-medium">{player.score} pts</span>
                </div>
            </div>
        {/each}
    </div>
</div>
