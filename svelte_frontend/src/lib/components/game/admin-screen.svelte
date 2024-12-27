<script lang="ts">
    import { gameStore } from '../../stores/game';
    import AdminControls from './admin-controls.svelte';
    import QuestionView from './question-view.svelte';
    import ScoreView from './score-view.svelte';
    import GameOver from './game-over.svelte';
    import { GamePhase } from '../../types/game';
    import { Card, CardContent } from '$lib/components/ui/card';

    // Get current game phase and join code
    $: phase = $gameStore.phase;
    $: joinCode = $gameStore.joinCode;
</script>

<div class="container mx-auto p-6">
    <div class="grid grid-cols-[1fr,300px] gap-6">
        <!-- Main content area -->
        <div class="space-y-6">
            <!-- Join code display -->
            <Card class="border-zinc-800 bg-zinc-900/50">
                <CardContent class="p-4">
                    <div class="flex items-center justify-between">
                        <div class="text-zinc-400">Join Code:</div>
                        <div class="font-mono text-lg font-bold">{joinCode}</div>
                    </div>
                </CardContent>
            </Card>

            <!-- Game phase specific content -->
            <Card class="border-zinc-800 bg-zinc-900/50">
                <CardContent>
                    {#if phase === GamePhase.Lobby}
                        <div class="p-6 text-center text-zinc-400">
                            Waiting for players to join...
                        </div>
                    {:else if phase === GamePhase.Question}
                        <QuestionView />
                    {:else if phase === GamePhase.Score}
                        <ScoreView />
                    {:else if phase === GamePhase.GameOver}
                        <GameOver />
                    {/if}
                </CardContent>
            </Card>
        </div>

        <!-- Admin controls sidebar -->
        <div class="space-y-6">
            <AdminControls />

            <!-- Error display if any -->
            {#if $gameStore.error}
                <Card class="border-red-500/20 bg-red-500/10">
                    <CardContent class="p-4 text-red-500">
                        {$gameStore.error}
                    </CardContent>
                </Card>
            {/if}
        </div>
    </div>
</div>
