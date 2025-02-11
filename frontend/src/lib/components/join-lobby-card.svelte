<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Input } from '$lib/components/ui/input';
	import { gameActions } from '$lib/stores/game-actions';
	import { notifications } from '$lib/stores/notification-store';
	import { warn } from '$lib/utils/logger';
	import { gameStore } from '$lib/stores/game.svelte';
	import { PUBLIC_SPEKTRUM_SERVER_URL } from '$env/static/public';

	// Local reactive variables.
	let lobbyCode: string = '';
	let playerName: string = '';
	let isJoining = false;

	async function handleJoinGame() {
		if (!lobbyCode || !playerName) {
			notifications.add('Please enter both lobby code and player name', 'destructive');
			return;
		}
		try {
			isJoining = true;
			// Call the join-lobby HTTP endpoint.
			const response = await fetch(`${PUBLIC_SPEKTRUM_SERVER_URL}/api/join-lobby`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ join_code: lobbyCode, name: playerName })
			});
			if (!response.ok) {
				throw new Error('Failed to join lobby');
			}
			gameStore.setPlayerName(playerName);
			const data = await response.json();
			// Then, initiate the websocket connection.
			await gameActions.joinGame(data.player_id);
		} catch (error) {
			warn('Error joining game:', error);
			notifications.add('Failed to join game', 'destructive');
		} finally {
			isJoining = false;
		}
	}
</script>

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
		<Button on:click={handleJoinGame} disabled={isJoining || !lobbyCode || !playerName}>
			{isJoining ? 'Joining...' : 'Join Game'}
		</Button>
	</CardContent>
</Card>
