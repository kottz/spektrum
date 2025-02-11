<script lang="ts">
	// Import UI components and stores.
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import NotificationList from '$lib/components/NotificationList.svelte';
	import SetSelector from '$lib/components/set-selector.svelte';
	import LightSwitch from '$lib/components/ui/light-switch.svelte';
	import JoinLobbyCard from '$lib/components/join-lobby-card.svelte';
	import { gameActions } from '$lib/stores/game-actions';
	import { gameStore } from '$lib/stores/game.svelte';
	import type { SessionInfo } from '$lib/stores/game.svelte';

	let showSetSelector = $state(false);
	let storedSessions = $state<SessionInfo[]>([]);

	// On component initialization, check for saved sessions.
	//$effect(() => {
	//	storedSessions = gameStore.checkSessions();
	//});

	// Function to reconnect using a saved session.
	function reconnectToSession(session: SessionInfo) {
		gameStore.setPlayerId(session.playerId);
		gameStore.setPlayerName(session.playerName);
		gameActions.reconnectGame(session.playerId);
	}
</script>

<!-- Notifications -->
<NotificationList />

<!-- Main container -->
<div class="container flex min-h-screen flex-col items-center justify-center gap-8 py-8">
	<div class="flex items-center gap-3">
		<span class="text-2xl">🎵</span>
		<h1 class="text-3xl font-bold">Music Quiz</h1>
		<LightSwitch />
	</div>

	<div class="grid w-full max-w-lg gap-6">
		<!-- Create Lobby / Set Selection Card -->
		<Card>
			<CardHeader class="mb-2">
				<div class="flex items-center gap-4">
					{#if showSetSelector}
						<Button variant="outline" size="sm" on:click={() => (showSetSelector = false)}>
							Back
						</Button>
					{/if}
					<CardTitle>{showSetSelector ? 'Select Question Set' : 'Create New Lobby'}</CardTitle>
				</div>
			</CardHeader>
			<CardContent class={showSetSelector ? 'h-[400px] p-0' : 'p-4'}>
				{#if showSetSelector}
					<SetSelector />
				{:else}
					<Button size="lg" class="w-full" on:click={() => (showSetSelector = true)}>
						Create Lobby
					</Button>
				{/if}
			</CardContent>
		</Card>

		{#if !showSetSelector}
			<!-- Use the new JoinLobbyCard component for joining an existing lobby -->
			<JoinLobbyCard />
		{/if}
	</div>
</div>
