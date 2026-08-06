import type { GameState, GameUpdate } from '$lib/types/game';

export type StreamStateSnapshot = Extract<GameUpdate, { type: 'StateDelta' }>;

export function createStreamStateSnapshot(gameState: GameState): StreamStateSnapshot {
	return {
		type: 'StateDelta',
		phase: gameState.phase,
		scoreboard: Array.from(gameState.players, ([name, player]) => [name, player.score]),
		round_scores: Array.from(gameState.players, ([name, player]) => [name, player.roundScore || 0]),
		answered_player_names: gameState.currentAnswers.map(({ name }) => name),
		question_time_remaining_ms: gameState.questionTimeRemainingMs,
		question_type: gameState.currentQuestion?.type,
		question_text: gameState.currentQuestion?.text,
		alternatives: gameState.currentQuestion?.alternatives
	};
}
