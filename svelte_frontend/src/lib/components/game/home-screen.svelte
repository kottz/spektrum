<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { gameActions } from '../../stores/game-actions';
	import { gameStore } from '../../stores/game';
	import { notifications } from '../../stores/notification-store';
	import NotificationList from '$lib/components/NotificationList.svelte';

	/**
	 * The user can still enter a playerName for joining,
	 * but we don't allow custom admin names when creating a new lobby.
	 */
	export let playerName = '';
	export let lobbyCode = '';

	let isJoining = false;
	let isCreating = false;
	let joinError = '';

	// For listing multiple saved sessions (from localStorage)
	let storedSessions = [];

	onMount(async () => {
		// If we have a server URL from env or config:
		storedSessions = await gameStore.checkSessions();
	});

	/**
	 * Creates a new lobby as Admin, but no custom admin name is needed.
	 * The backend or the store logic will set us to "Admin" automatically.
	 */
	const handleCreateLobby = async () => {
		if (isCreating) return;
		try {
			isCreating = true;
			await gameActions.createGame();
			// We pass no playerName — the default 'Admin' is used in createGame()
		} catch (error) {
			console.error('Error creating lobby:', error);
			notifications.add('Failed to create lobby.', 'destructive');
		} finally {
			isCreating = false;
		}
	};

	/**
	 * Joins an existing lobby with a user-provided name and code.
	 */
	const handleJoinGame = async () => {
		if (!lobbyCode || !playerName) {
			alert('Please enter both lobby code and player name');
			return;
		}
		try {
			isJoining = true;
			joinError = '';
			await gameActions.joinGame(lobbyCode, playerName);
		} catch (error) {
			console.error('Error joining game:', error);
			notifications.add(`Failed to join game.`, 'destructive');
		} finally {
			isJoining = false;
		}
	};

	/**
	 * If you want to reconnect to the *active* credentials (lobby/player)
	 * from gameStore or localStorage. (Single-lobby approach.)
	 */
	const handleReconnect = () => {
		gameActions.reconnectGame();
	};

	/**
	 * Called when the user wants to reconnect to a *specific* saved session.
	 * (If you implement a multi-lobby approach, you might do something like
	 * `websocketStore.connect(...)`, or another custom flow.)
	 */
	function reconnectToSession(sess) {
		console.log('Reconnecting to session:', sess);

		// Step 1: Set store state to the chosen session
		gameStore.setLobbyId(sess.lobbyId);
		gameStore.setPlayerId(sess.playerId);
		gameStore.setPlayerName(sess.playerName);

		// Step 2: Call connect() with no arguments
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
		<!-- Create Lobby Card (No custom admin name field) -->
		<Card>
			<CardHeader>
				<CardTitle>Create New Lobby</CardTitle>
			</CardHeader>
			<CardContent>
				<!-- No text input for admin name -->
				<Button size="lg" class="w-full" on:click={handleCreateLobby} disabled={isCreating}>
					{isCreating ? 'Creating...' : 'Create Lobby'}
				</Button>
			</CardContent>
		</Card>

		{#if storedSessions.length > 0}
			<Card>
				<CardHeader>
					<CardTitle>Saved Sessions</CardTitle>
				</CardHeader>
				<CardContent class="grid gap-2">
					{#each storedSessions as sess}
						<Button size="lg" class="w-full" on:click={() => reconnectToSession(sess)}>
							Reconnect to Lobby: (Player: {sess.playerName}, code: {sess.joinCode})
						</Button>
					{/each}
				</CardContent>
			</Card>
		{/if}

		<!-- Join Lobby Card -->
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
				{#if joinError}
					<div class="text-sm text-destructive">
						{joinError}
					</div>
				{/if}
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
