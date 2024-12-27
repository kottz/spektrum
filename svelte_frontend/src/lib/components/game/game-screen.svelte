<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import { Card, CardContent, CardHeader, CardTitle } from "$lib/components/ui/card";
  import { Progress } from "$lib/components/ui/progress";

  // Props
  export let playerName: string;
  export let lobbyCode: string;

  // Simple state placeholders - we'll handle proper state management later
  let phase: "question" | "score" = "question";
  let timeRemaining = 60;
  let selectedColor: number | null = null;

  const colors = [
    "bg-emerald-500",
    "bg-purple-500",
    "bg-amber-500",
    "bg-blue-500",
    "bg-red-500",
    "bg-brown-500",
  ];
</script>

<div class="container mx-auto flex min-h-screen flex-col gap-6 p-6">
  <header class="flex items-center justify-between">
    <div class="flex items-center gap-3">
      <!-- Replace Music4 icon with simple text/emoji -->
      <span class="text-xl">🎵</span>
      <h1 class="text-2xl font-bold">Music Quiz</h1>
    </div>
    <div class="flex items-center gap-4">
      <div class="flex items-center gap-2">
        <!-- Replace Timer icon with simple text/emoji -->
        <span class="text-zinc-400">⏱️</span>
        <span class="text-lg font-medium">{timeRemaining}s</span>
      </div>
      <Button variant="outline" class="border-zinc-800 hover:bg-zinc-800">
        Leave Lobby
      </Button>
    </div>
  </header>

  <Card class="border-zinc-800 bg-zinc-900/50">
    <CardHeader class="flex flex-row items-center justify-between">
      <CardTitle>Game Phase: {phase}</CardTitle>
      <div class="text-sm text-zinc-400">Player: {playerName}</div>
    </CardHeader>
    <CardContent class="grid gap-6">
      {#if phase === "question"}
        <div class="text-center text-2xl font-bold">What color represents this song?</div>
        <Progress value={33} class="h-2 w-full bg-zinc-800" />
        <div class="grid grid-cols-3 gap-4 max-w-2xl mx-auto w-full">
          {#each colors as color, i}
            <button
              on:click={() => selectedColor === null && (selectedColor = i)}
              class="w-24 h-24 rounded-lg {color} transition-transform hover:scale-105 focus:outline-none
                {selectedColor !== null && selectedColor !== i ? 'opacity-50' : ''}
                {selectedColor === i ? 'ring-4 ring-white ring-opacity-60' : ''}
                {selectedColor === null ? 'hover:ring-2 hover:ring-white hover:ring-opacity-50' : ''}"
              disabled={selectedColor !== null}
            />
          {/each}
        </div>
      {:else}
        <div class="space-y-6">
          <div class="text-center text-2xl font-bold">Scoreboard</div>
          <div class="space-y-4">
            {#each [{name: playerName, score: 10210, correct: true}, {name: "Player 2", score: 8150, correct: false}] as player}
              <div class="space-y-2">
                <div class="flex items-center justify-between">
                  <span>{player.name}</span>
                  <span>{player.score} points</span>
                </div>
                <Progress
                  value={75}
                  class="h-2 {player.correct ? 'bg-emerald-500/20' : 'bg-red-500/20'}"
                  indicatorClass={player.correct ? "bg-emerald-500" : "bg-red-500"}
                />
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </CardContent>
  </Card>
</div>
