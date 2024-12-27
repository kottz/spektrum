<script lang="ts">
    import { gameStore } from '../../stores/game';
    import { gameActions } from '../../stores/game-actions';
    import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
    import { Button } from '$lib/components/ui/button';
    import Scoreboard from './scoreboard.svelte';

    $: isAdmin = $gameStore.isAdmin;
</script>

<div class="container mx-auto max-w-2xl p-6 space-y-6">
    <!-- Scoreboard -->
    <Card class="border-zinc-800 bg-zinc-900/50">
        <CardContent class="p-6">
            <Scoreboard />
        </CardContent>
    </Card>

    <!-- Admin controls or waiting message -->
    {#if isAdmin}
        <Button
            class="w-full"
            on:click={() => gameActions.startRound()}
        >
            Start Next Round
        </Button>
    {:else}
        <div class="text-center text-zinc-400">
            Waiting for next round...
        </div>
    {/if}
</div>
