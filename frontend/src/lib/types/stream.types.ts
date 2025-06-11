export interface BasePublicGameState {
	phase: { type: string; data?: Record<string, unknown> };
}

export interface StreamEvent {
	id: string;
	type: string;
	timestamp: number;
	data: Record<string, unknown>;
	duration?: number;
}

export interface DisplayConfig {
	showPlayerNames: boolean;
	showScores: boolean;
	showAnswerProgress: boolean;
	showTimer: boolean;
	animationSpeed: number;
}

export const DEFAULT_DISPLAY_CONFIG: DisplayConfig = {
	showPlayerNames: true,
	showScores: true,
	showAnswerProgress: true,
	showTimer: true,
	animationSpeed: 1.0
};
