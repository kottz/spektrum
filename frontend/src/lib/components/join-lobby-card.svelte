<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Input } from '$lib/components/ui/input';
	import { gameActions } from '$lib/stores/game-actions';
	import { notifications } from '$lib/stores/notification-store';
	import { warn } from '$lib/utils/logger';
	import { gameStore } from '$lib/stores/game.svelte';
	import { PUBLIC_SPEKTRUM_SERVER_URL } from '$env/static/public';

	let lobbyCode = $state('');
	let playerName = $state('');
	let isJoining = $state(false);
	let hasAttemptedSubmit = $state(false);

	const NAME_VALIDATION_REGEX = /^[\p{L}\p{N}\s._-]+$/u;
	const LOBBY_CODE_REGEX = /^\d+$/;

	const isValidLobbyCode = $derived(LOBBY_CODE_REGEX.test(lobbyCode));
	const hasNameValidationError = $derived(
		playerName.length > 0 && (playerName.length > 16 || !NAME_VALIDATION_REGEX.test(playerName))
	);
	const isNameTooShort = $derived(playerName.length > 0 && playerName.length < 2);

	function handleNameInput(e: Event) {
		if ((e.target as HTMLInputElement).value.length === 0) {
			hasAttemptedSubmit = false;
		}
	}

	async function handleJoinGame() {
		hasAttemptedSubmit = true;

		if (!isValidLobbyCode) {
			notifications.add('Lobby code must contain only numbers', 'destructive');
			return;
		}

		if (isNameTooShort) {
			notifications.add('Name must be at least 2 characters', 'destructive');
			return;
		}

		if (hasNameValidationError) {
			notifications.add('Invalid name format', 'destructive');
			return;
		}

		try {
			isJoining = true;

			const response = await fetch(`${PUBLIC_SPEKTRUM_SERVER_URL}/api/join-lobby`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					join_code: lobbyCode.trim(),
					name: playerName.trim()
				}),
				cache: 'no-store'
			});

			if (!response.ok) {
				const errorData = await response.json();
				const errorMessage = errorData.details || errorData.error || 'Failed to join lobby';
				throw new Error(errorMessage);
			}

			gameStore.setPlayerName(playerName.trim());
			const data = await response.json();
			gameStore.setJoinCode(data.join_code);
			gameStore.setSessionToken(data.session_token);
			gameStore.setPlayerId(data.player_id);
			await gameActions.joinGame(data.session_token);
		} catch (error) {
			let errorMessage = 'Failed to join lobby';
			if (error instanceof Error) {
				errorMessage = error.message;
			} else {
				errorMessage = String(error);
			}
			warn('Error joining game:', error);
			notifications.add(errorMessage, 'destructive');
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
		<div>
			<Input
				name="lobbyCode"
				type="text"
				inputmode="numeric"
				pattern="\d*"
				placeholder="Enter lobby code"
				bind:value={lobbyCode}
				disabled={isJoining}
				class={!isValidLobbyCode && lobbyCode ? 'border-red-500' : ''}
			/>
			{#if !isValidLobbyCode && lobbyCode}
				<p class="mt-1 text-sm text-red-500">Lobby code must contain only numbers</p>
			{/if}
		</div>

		<div>
			<Input
				name="playerName"
				placeholder="Enter your name"
				bind:value={playerName}
				oninput={handleNameInput}
				maxlength={16}
				disabled={isJoining}
				class={hasNameValidationError || (isNameTooShort && hasAttemptedSubmit)
					? 'border-red-500'
					: ''}
			/>
			{#if hasNameValidationError}
				<p class="mt-1 text-sm text-red-500">
					Name can only contain letters, numbers, spaces, and the symbols: _ - .
				</p>
			{:else if isNameTooShort && hasAttemptedSubmit}
				<p class="mt-1 text-sm text-red-500">Name must be at least 2 characters</p>
			{/if}
		</div>

		<Button onclick={handleJoinGame} disabled={isJoining || !lobbyCode || !playerName}>
			{isJoining ? 'Joining...' : 'Join Game'}
		</Button>
	</CardContent>
</Card>
