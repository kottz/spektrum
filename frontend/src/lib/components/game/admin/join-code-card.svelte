<script lang="ts">
	import { Card } from '$lib/components/ui/card';
	import { gameStore } from '$lib/stores/game.svelte';
	import { Copy, Link } from 'lucide-svelte';
	import { Button } from '$lib/components/ui/button';
	import { notifications } from '$lib/stores/notification-store';
	const joinCode = $derived(gameStore.state.joinCode);

	async function copyCodeToClipboard() {
		if (!joinCode) return;
		try {
			await navigator.clipboard.writeText(joinCode);
			notifications.add('Join code copied to clipboard', 'success');
		} catch {
			notifications.add('Failed to copy join code', 'destructive');
		}
	}

	async function copyLinkToClipboard() {
		if (!joinCode) return;
		try {
			const joinLink = `${window.location.origin}/join/${joinCode}`;
			await navigator.clipboard.writeText(joinLink);
			notifications.add('Join link copied to clipboard', 'success');
		} catch {
			notifications.add('Failed to copy join link', 'destructive');
		}
	}
</script>

<Card class="bg-card">
	<div class="flex flex-col items-center justify-center p-2">
		<div class="mb-2 flex w-full items-center">
			<div class="flex flex-1 justify-center">
				<h2 class="text-xl font-semibold">Join Code</h2>
			</div>
			{#if joinCode}
				<Button
					variant="ghost"
					size="icon"
					class="h-8 w-8 flex-none"
					on:click={copyLinkToClipboard}
					title="Copy join link"
				>
					<Link class="h-4 w-4" />
					<span class="sr-only">Copy join link</span>
				</Button>
			{/if}
		</div>
		<div class="flex w-full items-center">
			<div class="flex flex-1 justify-center">
				<div class="font-mono text-3xl font-bold">{joinCode ?? ''}</div>
			</div>
			{#if joinCode}
				<Button
					variant="ghost"
					size="icon"
					class="h-8 w-8 flex-none"
					on:click={copyCodeToClipboard}
					title="Copy join code"
				>
					<Copy class="h-4 w-4" />
					<span class="sr-only">Copy join code</span>
				</Button>
			{/if}
		</div>
	</div>
</Card>
