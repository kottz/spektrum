<script lang="ts">
    import { gameStore } from '../../stores/game';

    $: upcomingQuestions = $gameStore.upcomingQuestions || [];
</script>

<div class="space-y-2">
    <div class="flex justify-between text-sm">
        <span class="text-zinc-400">Upcoming Questions</span>
        <span class="font-medium">{upcomingQuestions.length}</span>
    </div>
    <div class="space-y-1">
        {#each upcomingQuestions as question, i}
            <div class="p-2 rounded bg-zinc-800/50 space-y-1">
                <div class="flex justify-between text-sm">
                    <span class="font-medium truncate flex-1">{question.song}</span>
                    <span class="text-zinc-400 ml-2">#{i + 1}</span>
                </div>
                {#if question.artist}
                    <div class="text-sm text-zinc-400 truncate">
                        {question.artist}
                    </div>
                {/if}
                <div class="text-xs flex gap-2 items-center text-zinc-500">
                    <span class="px-1.5 py-0.5 rounded bg-zinc-800">
                        {question.type}
                    </span>
                    {#if question.type === 'character'}
                        <span class="truncate">
                            Answer: {question.correct_character}
                        </span>
                    {:else if question.type === 'color' && question.colors}
                        <span class="truncate">
                            Colors: {question.colors.join(', ')}
                        </span>
                    {/if}
                </div>
            </div>
        {:else}
            <div class="text-sm text-zinc-400 text-center p-2">
                No upcoming questions
            </div>
        {/each}
    </div>
</div>
