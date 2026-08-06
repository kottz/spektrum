import { describe, expect, it } from 'vitest';
import { GamePhase, type GameState } from '$lib/types/game';
import { createStreamStateSnapshot } from './stream-state';

describe('createStreamStateSnapshot', () => {
	it('captures the complete current state for a stream opened mid-question', () => {
		const gameState: GameState = {
			phase: GamePhase.Question,
			isAdmin: true,
			joinCode: '123456',
			roundDuration: 60,
			lobbyLocked: false,
			players: new Map([
				[
					'Ada',
					{
						name: 'Ada',
						score: 120,
						roundScore: 20,
						hasAnswered: true,
						consecutiveMisses: 0,
						answer: null
					}
				]
			]),
			currentAnswers: [{ name: 'Ada', score: 20, timestamp: 1 }],
			questionTimeRemainingMs: 12_500,
			currentQuestion: {
				type: 'text',
				text: 'Which song is this?',
				alternatives: ['One', 'Two']
			}
		};

		expect(createStreamStateSnapshot(gameState)).toEqual({
			type: 'StateDelta',
			phase: GamePhase.Question,
			scoreboard: [['Ada', 120]],
			round_scores: [['Ada', 20]],
			answered_player_names: ['Ada'],
			question_time_remaining_ms: 12_500,
			question_type: 'text',
			question_text: 'Which song is this?',
			alternatives: ['One', 'Two']
		});
	});
});
