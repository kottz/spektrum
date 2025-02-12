<script lang="ts">
	import { Card } from '$lib/components/ui/card';
	import { gameStore } from '$lib/stores/game.svelte';
	import { Copy } from 'lucide-svelte';
	import { Button } from '$lib/components/ui/button';
	import { notifications } from '$lib/stores/notification-store';

	const joinCode = $derived(gameStore.state.joinCode);

	async function copyToClipboard() {
		if (!joinCode) return;

		try {
			await navigator.clipboard.writeText(joinCode);
			notifications.add('Join code copied to clipboard', 'success');
		} catch (err) {
			notifications.add('Failed to copy join code', 'destructive');
		}
	}
</script>

<Card class="bg-card">
	<div class="flex flex-col items-center justify-center p-2">
		<h2 class="mb-2 text-xl font-semibold">Join Code</h2>
		<div class="flex items-center gap-2">
			<div class="font-mono text-3xl font-bold">{joinCode ?? ''}</div>
			{#if joinCode}
				<Button
					variant="ghost"
					size="icon"
					class="h-8 w-8"
					on:click={copyToClipboard}
					title="Copy join code"
				>
					<Copy class="h-4 w-4" />
					<span class="sr-only">Copy join code</span>
				</Button>
			{/if}
		</div>
	</div>
</Card>
