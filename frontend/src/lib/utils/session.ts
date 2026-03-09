import { gameStore, type ValidatedSession } from '$lib/stores/game.svelte';
import { gameActions } from '$lib/stores/game-actions';
import { info, warn } from '$lib/utils/logger';

export async function checkAndLoadSession(): Promise<ValidatedSession | null> {
	try {
		const session = await gameStore.checkSessions();
		info('Session check result:', session);
		return session;
	} catch (error) {
		warn('Failed to check sessions:', error);
		return null;
	}
}

export function reconnectToSession(session: ValidatedSession): void {
	if (!session.sessionToken) return;

	gameStore.setPlayerId(session.playerId);
	gameStore.setAdminTo(session.isAdmin);
	gameStore.setPlayerName(session.playerName);
	gameStore.setJoinCode(session.joinCode);
	gameStore.setSessionToken(session.sessionToken);
	gameActions.joinGame(session.sessionToken);
}
