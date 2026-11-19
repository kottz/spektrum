import type { GamePhase } from '#lib/generated/GamePhase.ts';
import type { GameQuestion } from '#lib/generated/GameQuestion.ts';
import type { QuestionType } from '#lib/generated/QuestionType.ts';

/* Client-side state; the wire types are generated from the server in #lib/generated. */

/**
 * Information for a single answer in the current round.
 */
export interface PlayerAnswer {
	name: string;
	score: number;
	timestamp: number;
}

/**
 * Represents a single player’s state on the client side.
 */
export interface PlayerState {
	name: string;
	score: number;
	roundScore: number;
	hasAnswered: boolean;
	consecutiveMisses: number;
}

/**
 * Frontend store for the overall game state.
 */
export interface GameState {
	phase: GamePhase;
	playerId?: string;
	playerName?: string;
	joinCode?: string;
	sessionToken?: string;
	isAdmin: boolean;
	roundDuration: number;
	players: Map<string, PlayerState>;
	currentQuestion?: {
		type: QuestionType;
		text?: string;
		alternatives: string[];
	};
	upcomingQuestions?: GameQuestion[];
	currentAnswers: PlayerAnswer[];
	questionTimeRemainingMs?: number;
	lobbyLocked: boolean;
}
