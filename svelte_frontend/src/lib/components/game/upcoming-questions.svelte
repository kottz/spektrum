<script lang="ts">
	import { gameStore } from '../../stores/game';
	$: upcomingQuestions = $gameStore.upcomingQuestions || [];
</script>

<div class="space-y-2">
	<div class="flex justify-between text-sm">
		<span class="text-muted-foreground">Upcoming Questions</span>
		<span class="font-medium">{upcomingQuestions.length}</span>
	</div>
	<div class="space-y-1">
		{#each upcomingQuestions as question, i}
			<div class="space-y-1 rounded bg-muted p-2">
				<div class="flex justify-between text-sm">
					<span class="flex-1 truncate font-medium">{question.song}</span>
					<span class="ml-2 text-muted-foreground">#{i + 1}</span>
				</div>
				{#if question.artist}
					<div class="truncate text-sm text-muted-foreground">
						{question.artist}
					</div>
				{/if}
				<div class="flex items-center gap-2 text-xs text-muted-foreground/70">
					<span class="rounded bg-muted-foreground/10 px-1.5 py-0.5">
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
			<div class="text-sm text-muted-foreground text-center p-2">No upcoming questions</div>
		{/each}
	</div>
</div>
