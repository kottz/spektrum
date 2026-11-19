import type { PlayerState } from '#lib/types/game.ts';
import type { GameUpdate } from '#lib/generated/GameUpdate.ts';

/**
 * Applies the player-related fields of a delta. Absent fields leave existing
 * values untouched; a scoreboard replaces the player list (dropping players the
 * server no longer has). Always returns new objects, never mutating `players`.
 */
export function applyPlayerDelta(
	players: Map<string, PlayerState>,
	delta: Extract<GameUpdate, { type: 'StateDelta' }>
): Map<string, PlayerState> {
	const roundScores = delta.round_scores && new Map(delta.round_scores);
	const misses = delta.consecutive_misses && new Map(delta.consecutive_misses);
	const answered = delta.answered_player_names && new Set(delta.answered_player_names);
	if (!delta.scoreboard && !roundScores && !misses && !answered) return players;

	const entries: [string, number | undefined][] = delta.scoreboard
		? delta.scoreboard
		: [...players.keys()].map((name) => [name, undefined]);
	return new Map(
		entries.map(([name, score]) => {
			const previous = players.get(name);
			return [
				name,
				{
					name,
					score: score ?? previous?.score ?? 0,
					roundScore: roundScores?.get(name) ?? previous?.roundScore ?? 0,
					consecutiveMisses: misses?.get(name) ?? previous?.consecutiveMisses ?? 0,
					hasAnswered: answered ? answered.has(name) : (previous?.hasAnswered ?? false)
				}
			];
		})
	);
}
