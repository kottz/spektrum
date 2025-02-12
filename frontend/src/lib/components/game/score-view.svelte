<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import { gameActions } from '../../stores/game-actions';
	import { Card, CardContent } from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import Scoreboard from './scoreboard.svelte';
	const isAdmin = $derived(gameStore.state.isAdmin);
</script>

<div class="container mx-auto flex h-full min-h-0 max-w-2xl flex-col p-6">
	<Card class="min-h-0 flex-1">
		<CardContent class="flex h-full min-h-0 flex-col p-6">
			<div class="min-h-0 flex-1 overflow-auto">
				<Scoreboard />
			</div>
		</CardContent>
	</Card>

	<div class="mt-6 flex-none">
		{#if isAdmin}
			<Button class="w-full" on:click={() => gameActions.startRound()}>Start Next Round</Button>
		{:else}
			<div class="text-center text-muted-foreground">Waiting for next round...</div>
		{/if}
	</div>
</div>
