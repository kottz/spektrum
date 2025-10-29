<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { gameStore } from '$lib/stores/game.svelte';
	import { gameActions } from '$lib/stores/game-actions';

	const phase = $derived(gameStore.state.phase?.toLowerCase() || 'lobby');
	const isLobby = $derived(phase === 'lobby');
	const isGameOver = $derived(phase === 'gameover');
</script>

<Button
	variant="default"
	class="w-full"
	disabled={!(isLobby || isGameOver)}
	onclick={() => gameActions.startGame()}
>
	{isGameOver ? 'Restart Game' : 'Start Game'}
</Button>
