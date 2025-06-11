import type { StreamEvent } from '$lib/types/stream.types';
import type { SpektrumStreamEvent, PlayerState, GamePhase } from '$lib/types/game';

/**
 * Manages a queue of stream events with automatic expiration
 */
export class StreamEventManager {
	private events: StreamEvent[] = [];
	private maxEvents: number;

	constructor(maxEvents: number = 20) {
		this.maxEvents = maxEvents;
	}

	addEvent(
		type: string,
		data: Record<string, unknown>,
		duration: number = 5000,
		id?: string
	): void {
		const event: StreamEvent = {
			id: id || this.generateId(),
			type,
			timestamp: Date.now(),
			data,
			duration
		};

		this.events.push(event);

		// Remove oldest events if we exceed max
		if (this.events.length > this.maxEvents) {
			this.events = this.events.slice(-this.maxEvents);
		}
	}

	getEvents(): StreamEvent[] {
		return [...this.events];
	}

	clearEvents(): void {
		this.events = [];
	}

	removeExpiredEvents(): void {
		const now = Date.now();
		this.events = this.events.filter((event) => {
			if (!event.duration) return true;
			return now - event.timestamp < event.duration;
		});
	}

	private generateId(): string {
		return `event_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
	}
}

/**
 * Helper function to create common stream events
 */
export const createStreamEvent = {
	playerAnswered: (playerName: string, duration: number = 3000): SpektrumStreamEvent => ({
		id: `player_answered_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
		type: 'PLAYER_ANSWERED_STREAM',
		timestamp: Date.now(),
		duration,
		data: { playerName }
	}),

	newQuestion: (
		questionText?: string,
		alternativesCount: number = 0,
		duration: number = 4000
	): SpektrumStreamEvent => ({
		id: `new_question_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
		type: 'NEW_QUESTION_STREAM',
		timestamp: Date.now(),
		duration,
		data: { questionText, alternativesCount }
	}),

	phaseChange: (
		newPhase: GamePhase,
		previousPhase: GamePhase,
		duration: number = 2000
	): SpektrumStreamEvent => ({
		id: `phase_change_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
		type: 'PHASE_CHANGE_STREAM',
		timestamp: Date.now(),
		duration,
		data: { newPhase, previousPhase }
	})
};

/**
 * Filter game state to create a public version safe for broadcasting
 */
export function createPublicStateFilter() {
	// This could contain utility functions for filtering sensitive data
	// For now, it's a placeholder for future enhancements
	return {
		filterPlayers: (players: Map<string, PlayerState>) => {
			return Array.from(players.values()).map((p) => ({
				name: p.name,
				score: p.score,
				hasAnsweredPublic: p.hasAnswered
			}));
		},

		filterQuestion: (question: { type: string; text?: string; alternatives: string[] }) => {
			if (!question) return undefined;

			return {
				type: question.type,
				text: question.text,
				alternatives: question.alternatives
			};
		},

		filterAnswers: (
			answers: Array<{ name: string; score: number }>,
			revealCorrectness: boolean = false
		) => {
			return answers.map((ans) => ({
				name: ans.name,
				score: ans.score,
				...(revealCorrectness && { isCorrect: ans.score > 0 })
			}));
		}
	};
}
