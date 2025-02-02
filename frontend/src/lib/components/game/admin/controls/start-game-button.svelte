<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { gameStore } from '$lib/stores/game.svelte';
	import { gameActions } from '$lib/stores/game-actions';

	const phase = $derived(gameStore.state.phase?.toLowerCase() || 'lobby');
	const isLobby = $derived(phase === 'lobby');
</script>

<Button
	variant="default"
	class="w-full"
	disabled={!isLobby}
	on:click={() => gameActions.startGame()}
>
	Start Game
</Button>
