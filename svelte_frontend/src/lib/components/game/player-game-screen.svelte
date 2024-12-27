<script lang="ts">
    import { gameStore } from "../../stores/game";
    import QuestionView from "./question-view.svelte";
    import ScoreView from "./score-view.svelte";
    import LobbyView from "./lobby-view.svelte";
    import { GamePhase } from "../../types/game";
    import { Card } from "$lib/components/ui/card";

    $: phase = $gameStore.phase;
    $: joinCode = $gameStore.joinCode;
</script>

<div class="container mx-auto p-6 space-y-6">
    <!-- Join code display -->
    {#if joinCode}
        <Card class="border-zinc-800 bg-zinc-900/50">
            <div class="p-4 flex items-center justify-between">
                <div class="text-zinc-400">Join Code:</div>
                <div class="font-mono text-lg font-bold">{joinCode}</div>
            </div>
        </Card>
    {/if}

    <!-- Game content based on phase -->
    {#if phase === GamePhase.Lobby}
        <LobbyView />
    {:else if phase === GamePhase.Question}
        <QuestionView />
    {:else if phase === GamePhase.Score}
        <ScoreView />
    {/if}
</div>
