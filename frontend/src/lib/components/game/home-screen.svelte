<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { gameActions } from '$lib/stores/game-actions';
	import { gameStore } from '$lib/stores/game.svelte';
	import type { SessionInfo } from '$lib/stores/game.svelte';
	import { notifications } from '$lib/stores/notification-store';
	import NotificationList from '$lib/components/NotificationList.svelte';
	import SetSelector from '$lib/components/set-selector.svelte';
	import { warn } from '$lib/utils/logger';

	// Make props bindable
	let { playerName = $bindable(''), lobbyCode = $bindable('') } = $props();

	let isJoining = $state(false);
	let showSetSelector = $state(false);
	let storedSessions = $state([]);

	$effect(() => {
		storedSessions = gameStore.checkSessions();
	});

	async function handleJoinGame() {
		if (!lobbyCode || !playerName) {
			notifications.add('Please enter both lobby code and player name', 'destructive');
			return;
		}

		try {
			isJoining = true;
			await gameActions.joinGame(lobbyCode, playerName);
		} catch (error) {
			warn('Error joining game:', error);
			notifications.add('Failed to join game', 'destructive');
		} finally {
			isJoining = false;
		}
	}

	function reconnectToSession(session: SessionInfo) {
		gameStore.setLobbyId(session.lobbyId);
		gameStore.setPlayerId(session.playerId);
		gameStore.setPlayerName(session.playerName);
		gameActions.reconnectGame();
	}
</script>

<NotificationList />
<div class="container flex min-h-screen flex-col items-center justify-center gap-8 py-8">
	<div class="flex items-center gap-3">
		<span class="text-2xl">🎵</span>
		<h1 class="text-3xl font-bold">Music Quiz</h1>
	</div>

	<div class="grid w-full max-w-lg gap-6">
		<Card>
			<CardHeader>
				<CardTitle>{showSetSelector ? 'Select Question Set' : 'Create New Lobby'}</CardTitle>
			</CardHeader>
			<CardContent>
				{#if showSetSelector}
					<SetSelector on:back={() => (showSetSelector = false)} />
				{:else}
					<Button size="lg" class="w-full" on:click={() => (showSetSelector = true)}>
						Create Lobby
					</Button>
				{/if}
			</CardContent>
		</Card>

		{#if storedSessions.length > 0}
			<Card>
				<CardHeader>
					<CardTitle>Saved Sessions</CardTitle>
				</CardHeader>
				<CardContent class="grid gap-2">
					{#each storedSessions as session}
						<Button size="lg" class="w-full" on:click={() => reconnectToSession(session)}>
							Reconnect: {session.playerName} ({new Date(session.createdAt).toLocaleString()})
						</Button>
					{/each}
				</CardContent>
			</Card>
		{/if}

		<Card>
			<CardHeader>
				<CardTitle>Join Existing Lobby</CardTitle>
			</CardHeader>
			<CardContent class="grid gap-4">
				<Input
					name="lobbyCode"
					placeholder="Enter lobby code"
					bind:value={lobbyCode}
					disabled={isJoining}
				/>
				<Input
					name="playerName"
					placeholder="Enter your name"
					bind:value={playerName}
					disabled={isJoining}
				/>
				<Button
					size="lg"
					class="w-full"
					on:click={handleJoinGame}
					disabled={isJoining || !lobbyCode || !playerName}
				>
					{isJoining ? 'Joining...' : 'Join Game'}
				</Button>
			</CardContent>
		</Card>
	</div>
</div>
