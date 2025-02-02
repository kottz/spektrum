<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { gameStore } from '$lib/stores/game.svelte';
	import { gameActions } from '$lib/stores/game-actions';

	const phase = $derived(gameStore.state.phase?.toLowerCase() || 'lobby');
	const isLobby = $derived(phase === 'lobby');
	const isGameOver = $derived(phase === 'gameover');
</script>

<Button
	variant="destructive"
	class="w-full"
	disabled={isLobby}
	on:click={() => (isGameOver ? gameActions.leaveGame() : gameActions.endGame())}
>
	{isGameOver ? 'Leave Game' : 'End Game'}
</Button>
