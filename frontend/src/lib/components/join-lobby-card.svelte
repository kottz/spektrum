<script lang="ts">
	import { Button } from '#lib/components/ui/button/index.ts';
	import { Card, CardContent, CardHeader, CardTitle } from '#lib/components/ui/card/index.ts';
	import { Input } from '#lib/components/ui/input/index.ts';
	import { gameActions } from '#lib/stores/game-actions.ts';
	import { notifications } from '#lib/stores/notification-store.svelte.ts';
	import { warn } from '#lib/utils/logger.ts';

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

			await gameActions.joinLobby(lobbyCode.trim(), playerName.trim());
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : String(error);
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
