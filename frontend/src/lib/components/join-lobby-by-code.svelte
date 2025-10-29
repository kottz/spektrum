<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent } from '$lib/components/ui/card';
	import { Input } from '$lib/components/ui/input';
	import { gameActions } from '$lib/stores/game-actions';
	import { notifications } from '$lib/stores/notification-store';
	import { warn } from '$lib/utils/logger';
	import { gameStore } from '$lib/stores/game.svelte';
	import { PUBLIC_SPEKTRUM_SERVER_URL } from '$env/static/public';
	import { goto } from '$app/navigation';

	// Accept the join code as a prop
	const { initialJoinCode } = $props<{ initialJoinCode: string }>();

	// State for player name and joining status
	let playerName = $state('');
	let isJoining = $state(false);
	let hasAttemptedSubmit = $state(false);

	// Use the prop directly for validation and submission
	const lobbyCode = initialJoinCode;

	const NAME_VALIDATION_REGEX = /^[\p{L}\p{N}\s._-]+$/u;
	// Basic validation for the passed code (optional, API should handle robustly)
	const LOBBY_CODE_REGEX = /^\d+$/;
	const isValidLobbyCode = LOBBY_CODE_REGEX.test(lobbyCode);

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

		// Optional: Basic client-side check on the provided code
		if (!isValidLobbyCode) {
			notifications.add('Invalid lobby code format in URL.', 'destructive');
			warn('Attempted join with invalid code format from URL:', lobbyCode);
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
					// Use the prop directly
					join_code: lobbyCode.trim(),
					name: playerName.trim()
				})
			});

			if (!response.ok) {
				const errorData = await response.json();
				const errorMessage = errorData.details || errorData.error || 'Failed to join lobby';
				throw new Error(errorMessage);
			}

			gameStore.setPlayerName(playerName.trim());
			const data = await response.json();
			await gameActions.joinGame(data.player_id);
			goto('/');
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
	<CardContent class="grid gap-4">
		<div class="text-center">
			<p class="text-muted-foreground text-sm">You are joining lobby:</p>
			<p class="text-2xl font-semibold tracking-wider">{lobbyCode}</p>
		</div>
		<div class="mt-2">
			<Input
				name="playerName"
				placeholder="Enter your name"
				bind:value={playerName}
				on:input={handleNameInput}
				maxlength="16"
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

		<Button on:click={handleJoinGame} disabled={isJoining || !playerName}>
			{isJoining ? 'Joining...' : 'Join Game'}
		</Button>
	</CardContent>
</Card>
