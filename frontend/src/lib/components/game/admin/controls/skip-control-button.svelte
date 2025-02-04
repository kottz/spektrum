<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { gameStore } from '$lib/stores/game.svelte';
	import { gameActions } from '$lib/stores/game-actions';

	const phase = $derived(gameStore.state.phase?.toLowerCase() || 'lobby');
	const isGameRunning = $derived(phase !== 'lobby' && phase !== 'gameover');
	const isInQuestion = $derived(phase === 'question');
	const outOfQuestions = $derived(gameStore.state.upcomingQuestions?.length === 0);
</script>

{#if isGameRunning}
	<Button
		variant="outline"
		class="w-full"
		disabled={isInQuestion || !isGameRunning || outOfQuestions}
		on:click={() => gameActions.skipQuestion()}
	>
		Skip Question
	</Button>
{:else}
	<div></div>
{/if}
