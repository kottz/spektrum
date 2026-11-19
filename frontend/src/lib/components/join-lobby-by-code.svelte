<script lang="ts">
	import { Button } from '#lib/components/ui/button/index.ts';
	import { Card, CardContent } from '#lib/components/ui/card/index.ts';
	import { Input } from '#lib/components/ui/input/index.ts';
	import { gameActions } from '#lib/stores/game-actions.ts';
	import { notifications } from '#lib/stores/notification-store.svelte.ts';
	import { warn } from '#lib/utils/logger.ts';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';

	// Accept the join code as a prop
	const { initialJoinCode }: { initialJoinCode: string } = $props();

	// State for player name and joining status
	let playerName = $state('');
	let isJoining = $state(false);
	let hasAttemptedSubmit = $state(false);

	const NAME_VALIDATION_REGEX = /^[\p{L}\p{N}\s._-]+$/u;
	// Basic validation for the passed code (optional, API should handle robustly)
	const LOBBY_CODE_REGEX = /^\d+$/;
	const isValidLobbyCode = $derived(LOBBY_CODE_REGEX.test(initialJoinCode));

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
			warn('Attempted join with invalid code format from URL:', initialJoinCode);
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

			await gameActions.joinLobby(initialJoinCode.trim(), playerName.trim());
			goto(resolve('/'));
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
	<CardContent class="grid gap-4">
		<div class="text-center">
			<p class="text-muted-foreground text-sm">You are joining lobby:</p>
			<p class="text-2xl font-semibold tracking-wider">{initialJoinCode}</p>
		</div>
		<div class="mt-2">
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

		<Button onclick={handleJoinGame} disabled={isJoining || !playerName}>
			{isJoining ? 'Joining...' : 'Join Game'}
		</Button>
	</CardContent>
</Card>
