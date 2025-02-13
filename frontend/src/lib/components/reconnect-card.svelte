<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import type { SessionInfo } from '$lib/stores/game.svelte';

	const props = $props<{
		session: SessionInfo & { last_update: string };
		onReconnect: () => void;
		onNewLobby: () => void;
	}>();

	// Function to format the time ago string
	function getTimeAgoString(lastUpdateStr: string): string {
		const lastUpdate = new Date(lastUpdateStr);
		const now = new Date();
		const diffSeconds = Math.floor((now.getTime() - lastUpdate.getTime()) / 1000);

		if (diffSeconds < 60) {
			return `${diffSeconds} seconds ago`;
		}

		const diffMinutes = Math.floor(diffSeconds / 60);
		if (diffMinutes < 60) {
			return `${diffMinutes} minute${diffMinutes === 1 ? '' : 's'} ago`;
		}

		const diffHours = Math.floor(diffMinutes / 60);
		if (diffHours < 24) {
			return `${diffHours} hour${diffHours === 1 ? '' : 's'} ago`;
		}

		const diffDays = Math.floor(diffHours / 24);
		return `${diffDays} day${diffDays === 1 ? '' : 's'} ago`;
	}

	let timeAgoString = $derived(getTimeAgoString(props.session.last_update));
</script>

<Card>
	<CardHeader>
		<CardTitle>Rejoin Previous Lobby</CardTitle>
	</CardHeader>
	<CardContent class="space-y-4">
		<div class="text-sm text-muted-foreground">
			Last lobby update: {timeAgoString}
		</div>
		<div class="space-y-2">
			<Button class="w-full" on:click={props.onReconnect}>Reconnect to Lobby</Button>
			<Button variant="outline" class="w-full" on:click={props.onNewLobby}>
				Join Different Lobby
			</Button>
		</div>
	</CardContent>
</Card>
