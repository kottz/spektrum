<script lang="ts">
	import { Button } from '#lib/components/ui/button/index.ts';
	import { gameStore } from '#lib/stores/game.svelte.ts';
	import { gameActions } from '#lib/stores/game-actions.ts';

	const phase = $derived(gameStore.state.phase?.toLowerCase() || 'lobby');
	const isGameRunning = $derived(phase !== 'lobby' && phase !== 'gameover');
	const isInQuestion = $derived(phase === 'question');
	const outOfQuestions = $derived(gameStore.state.upcomingQuestions?.length === 0);
</script>

{#if isGameRunning}
	<Button
		variant="outline"
		class="flex-1"
		disabled={isInQuestion || !isGameRunning || outOfQuestions}
		onclick={() => gameActions.skipQuestion()}
	>
		Skip Question
	</Button>
{:else}
	<div></div>
{/if}
