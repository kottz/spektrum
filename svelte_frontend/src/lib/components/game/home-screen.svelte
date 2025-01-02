<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { gameActions } from '../../stores/game-actions';
	import { gameStore } from '../../stores/game';
	import { get } from 'svelte/store';

	export let playerName = "";
	export let lobbyCode = "";

	let isJoining = false;
	let isCreating = false;
	let joinError = '';
	let hasStoredCredentials = false; 

	onMount(() => {
		// Check if we have stored credentials
		const storedLobbyId = localStorage.getItem('lobbyId');
		const storedPlayerId = localStorage.getItem('playerId');
		if (storedLobbyId && storedPlayerId) {
			hasStoredCredentials = true;
		}
	});

	const handleCreateLobby = async () => {
		if (isCreating) return;
		
		try {
			isCreating = true;
			await gameActions.createGame(playerName || 'Admin');
		} catch (error) {
			console.error("Error creating lobby:", error);
			alert('Failed to create lobby. Please try again.');
		} finally {
			isCreating = false;
		}
	};

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
			console.error("Error joining game:", error);
			joinError = 'Failed to join game. Please check your code and try again.';
		} finally {
			isJoining = false;
		}
	};

	const handleReconnect = () => {
		gameActions.reconnectGame();
	};
</script>

<div class="container flex min-h-screen flex-col items-center justify-center gap-8 py-8">
	<div class="flex items-center gap-3">
		<span class="text-2xl">🎵</span>
		<h1 class="text-3xl font-bold">Music Quiz</h1>
	</div>
	<div class="grid w-full max-w-lg gap-6">
		<!-- Create Lobby Card -->
		<Card>
			<CardHeader>
				<CardTitle>Create New Lobby</CardTitle>
			</CardHeader>
			<CardContent>
				<Input
					name="playerName"
					placeholder="Your name (optional, defaults to Admin)"
					bind:value={playerName}
					disabled={isCreating}
					class="mb-4"
				/>
				<Button
					size="lg"
					class="w-full"
					on:click={handleCreateLobby}
					disabled={isCreating}
				>
					{isCreating ? 'Creating...' : 'Create Lobby'}
				</Button>
			</CardContent>
		</Card>

		<!-- Join Lobby Card -->
		<Card>
			<CardHeader>
				<CardTitle>Join Game</CardTitle>
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

		<!-- Optional Reconnect Button if credentials exist in localStorage -->
		{#if hasStoredCredentials}
			<Card>
				<CardHeader>
					<CardTitle>Reconnect to Existing Game</CardTitle>
				</CardHeader>
				<CardContent>
					<Button
						size="lg"
						class="w-full"
						on:click={handleReconnect}
					>
						Try Reconnect
					</Button>
				</CardContent>
			</Card>
		{/if}
	</div>
</div>
