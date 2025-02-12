<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	const upcomingQuestions = $derived(gameStore.state.upcomingQuestions || []);
</script>

<div class="space-y-2">
	<div class="space-y-1">
		{#each upcomingQuestions as question, i}
			<div class="space-y-1 rounded bg-secondary p-2">
				<div class="flex justify-between text-sm">
					<span class="flex-1 truncate font-medium">{question.title}</span>
					<span class="ml-2 text-muted-foreground">#{i + 1}</span>
				</div>
				<div class="truncate text-sm text-muted-foreground">
					{question.artist}
				</div>
				<div class="flex items-center gap-2 text-xs text-muted-foreground/70">
					<span class="rounded bg-muted-foreground/10 px-1.5 py-0.5">
						{question.question_type}
					</span>
					{#if question.question_type === 'character'}
						<span class="truncate">
							{question.options.find((opt) => opt.is_correct)?.option || 'Unknown'}
						</span>
					{:else if question.question_type === 'color'}
						<span class="truncate">
							{question.options
								.filter((opt) => opt.is_correct)
								.map((opt) => opt.option)
								.join(', ')}
						</span>
					{/if}
				</div>
			</div>
		{:else}
			<div class="text-sm text-muted-foreground text-center p-2">No upcoming questions</div>
		{/each}
	</div>
</div>
