// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import type { PlayerState } from '../types/game';

vi.mock('$app/env', () => ({ browser: true }));
vi.mock('$app/env/public', () => ({
	PUBLIC_SPEKTRUM_SERVER_URL: 'http://test',
	PUBLIC_SPEKTRUM_WS_SERVER_URL: 'ws://test'
}));
vi.mock('#lib/utils/logger.ts', () => ({ info: () => {}, warn: () => {} }));

const { applyPlayerDelta } = await import('./player-delta');
const { gameStore } = await import('./game.svelte');

function player(name: string, fields: Partial<PlayerState> = {}): [string, PlayerState] {
	return [
		name,
		{ name, score: 0, roundScore: 0, hasAnswered: false, consecutiveMisses: 0, ...fields }
	];
}

describe('applyPlayerDelta', () => {
	it('keeps round scores and misses that the delta does not mention', () => {
		const before = new Map([player('Ann', { score: 100, roundScore: 40, consecutiveMisses: 2 })]);
		const after = applyPlayerDelta(before, {
			type: 'StateDelta',
			scoreboard: [['Ann', 100]]
		});
		expect(after.get('Ann')).toMatchObject({ roundScore: 40, consecutiveMisses: 2 });
	});

	it('treats a scoreboard as the complete player list', () => {
		const before = new Map([player('Ann'), player('Ghost')]);
		const after = applyPlayerDelta(before, {
			type: 'StateDelta',
			scoreboard: [['Ann', 5]]
		});
		expect([...after.keys()]).toEqual(['Ann']);
	});

	it('updates answered flags without mutating the previous players', () => {
		const ann = player('Ann');
		const before = new Map([ann]);
		const after = applyPlayerDelta(before, {
			type: 'StateDelta',
			answered_player_names: ['Ann']
		});
		expect(after.get('Ann')?.hasAnswered).toBe(true);
		expect(ann[1].hasAnswered).toBe(false);
	});

	it('returns the same map when the delta carries no player data', () => {
		const before = new Map([player('Ann')]);
		expect(applyPlayerDelta(before, { type: 'StateDelta', lobby_locked: true })).toBe(before);
	});
});

describe('GameOver', () => {
	it('applies the final scores, which include points from an unfinished round', () => {
		gameStore.processServerMessage({
			type: 'StateDelta',
			phase: 'question',
			scoreboard: [['Ann', 100]]
		});
		gameStore.processServerMessage({
			type: 'GameOver',
			final_scores: [['Ann', 4100]],
			reason: 'Game ended by admin'
		});
		expect(gameStore.state.phase).toBe('gameover');
		expect(gameStore.state.players.get('Ann')?.score).toBe(4100);
		expect(gameStore.state.currentQuestion).toBeUndefined();
	});
});
