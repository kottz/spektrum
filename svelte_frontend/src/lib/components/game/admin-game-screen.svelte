<script lang="ts">
  import { gameStore } from '../../stores/game';
  import AdminControls from './admin-controls.svelte';
  import Scoreboard from './scoreboard.svelte';
  import { Card } from '$lib/components/ui/card';
  import { GamePhase } from '../../types/game';

  $: joinCode = $gameStore.joinCode;
  $: phase = $gameStore.phase;
  $: showScoreboard = phase === GamePhase.Score || phase === GamePhase.GameOver;
</script>

<div class="container mx-auto p-6">
  <div class="grid grid-cols-[1fr,300px] gap-6">
    <!-- Main content -->
    <div class="space-y-6">
      <!-- Join code -->
      {#if joinCode}
        <Card class="border-zinc-800 bg-zinc-900/50">
          <div class="p-4 flex items-center justify-between">
            <div class="text-zinc-400">Join Code:</div>
            <div class="font-mono text-lg font-bold">{joinCode}</div>
          </div>
        </Card>
      {/if}

      <!-- Scoreboard (shown in score phase and game over) -->
      {#if showScoreboard}
        <Card class="border-zinc-800 bg-zinc-900/50">
          <div class="p-6">
            <Scoreboard />
          </div>
        </Card>
      {/if}
    </div>

    <!-- Admin controls sidebar -->
    <div class="space-y-6">
      <AdminControls />

      {#if $gameStore.error}
        <Card class="border-red-500/20 bg-red-500/10">
          <div class="p-4 text-red-500">
            {$gameStore.error}
          </div>
        </Card>
      {/if}
    </div>
  </div>
</div>
