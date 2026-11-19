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
		class="flex-1"
		disabled={outOfQuestions}
		onclick={() => (isInQuestion ? gameActions.endRound() : gameActions.startRound())}
	>
		{isInQuestion ? 'End Round' : 'Start Round'}
	</Button>
{:else}
	<div></div>
{/if}
