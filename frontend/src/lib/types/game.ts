// src/lib/types/game.ts

/**
 * Core phases as defined by the backend.
 * Matches the enum GamePhase { Lobby, Score, Question, GameOver }.
 */
export enum GamePhase {
	Lobby = 'lobby',
	Score = 'score',
	Question = 'question',
	GameOver = 'gameover'
}

/**
 * Possible error codes the server can send for invalid actions or states.
 */
export enum ErrorCode {
	NotAuthorized = 'NotAuthorized',
	InvalidPhase = 'InvalidPhase',
	InvalidAction = 'InvalidAction',
	GameClosed = 'GameClosed',
	PlayerNotFound = 'PlayerNotFound',
	AlreadyAnswered = 'AlreadyAnswered',
	TimeExpired = 'TimeExpired',
	LobbyNotFound = 'LobbyNotFound',
	InvalidName = 'InvalidName'
}

/**
 * Information for a single answer in the current round.
 */
export interface PlayerAnswer {
	name: string;
	correct: boolean;
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
	answer: string | null;
}

/**
 * Frontend store for the overall game state.
 */
export interface GameState {
	phase: GamePhase;
	playerId?: string;
	playerName?: string;
	joinCode?: string;
	isAdmin: boolean;
	roundDuration: number;
	players: Map<string, PlayerState>;
	currentQuestion?: {
		type: string;
		alternatives: string[];
	};
	currentSong?: {
		songName: string;
		artist: string | undefined;
		youtubeId: string;
	};
	error?: string;
	upcomingQuestions?: GameQuestion[];
	currentAnswers: PlayerAnswer[];
}

/**
 * Represents a single quiz question.
 */
export interface GameQuestion {
	id: number;
	question_type: string;
	title: string;
	artist?: string;
	youtube_id: string;
	options: GameQuestionOption[];
}

export interface GameQuestionOption {
	option: string;
	is_correct: boolean;
}

/**
 * Extra information for the admin about the upcoming questions.
 */
export interface AdminExtraInfo {
	upcoming_questions: GameQuestion[];
}

/* ------------------------------------------------------------------
   SERVER -> CLIENT MESSAGES
------------------------------------------------------------------ */

export type GameUpdate =
	| {
			type: 'Connected';
			player_id: string;
			name: string;
			round_duration: number;
	  }
	| {
			type: 'StateDelta';
			phase?: GamePhase;
			question_type?: string;
			alternatives?: string[];
			scoreboard?: [string, number][];
			round_scores?: [string, number][];
			admin_extra?: AdminExtraInfo;
	  }
	| {
			type: 'PlayerLeft';
			name: string;
	  }
	| {
			type: 'Answered';
			name: string;
			correct: boolean;
	  }
	| {
			type: 'GameOver';
			final_scores: [string, number][];
			reason: string;
	  }
	| {
			type: 'GameClosed';
			reason: string;
	  }
	| {
			type: 'Error';
			message: string;
	  }
	| {
			type: 'AdminInfo';
			current_question: GameQuestion;
	  }
	| {
			type: 'AdminNextQuestions';
			upcoming_questions: GameQuestion[];
	  };

/* ------------------------------------------------------------------
   CLIENT -> SERVER MESSAGES
------------------------------------------------------------------ */

export type ClientMessage =
	| {
			type: 'Connect';
			player_id: string;
	  }
	| {
			type: 'Leave';
	  }
	| {
			type: 'Answer';
			answer: string;
	  }
	| {
			type: 'AdminAction';
			action: AdminAction;
	  };

/**
 * Administrative actions that can be performed in the lobby.
 */
export type AdminAction =
	| { type: 'StartGame' }
	| { type: 'StartRound' }
	| { type: 'EndRound' }
	| { type: 'SkipQuestion' }
	| { type: 'EndGame'; reason: string }
	| { type: 'CloseGame'; reason: string };

/**
 * Common name validation errors that might be returned by the server or client.
 */
export type NameValidationError = 'TooShort' | 'TooLong' | 'InvalidCharacters' | 'AlreadyTaken';

/**
 * Returns a user-friendly description for a name validation error.
 */
export function getNameValidationErrorMessage(error: NameValidationError): string {
	switch (error) {
		case 'TooShort':
			return 'Name must be at least 2 characters long.';
		case 'TooLong':
			return 'Name cannot be longer than 16 characters.';
		case 'InvalidCharacters':
			return 'Name can only contain letters, numbers, spaces, and the symbols: _ - .';
		case 'AlreadyTaken':
			return 'This name is already taken.';
	}
}
