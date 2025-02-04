<script lang="ts">
	import { Card } from '$lib/components/ui/card';
	import { gameStore } from '$lib/stores/game.svelte';

	const joinCode = $derived(gameStore.state.joinCode);
</script>

<Card class="bg-card">
	<div class="flex flex-col items-center justify-center p-2">
		<h2 class="mb-2 text-xl font-semibold">Join Code</h2>
		<div class="font-mono text-3xl font-bold">{joinCode}</div>
	</div>
</Card>
