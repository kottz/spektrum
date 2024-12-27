// src/lib/types/game.ts

// Core game enums and types that mirror the backend
export enum GamePhase {
	Lobby = 'lobby',
	Score = 'score',
	Question = 'question',
	GameOver = 'gameover'
}

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

export interface PlayerAnswer {
    name: string;
    correct: boolean;
    timestamp: number;
}

// Player state
export interface PlayerState {
	name: string;
	score: number;
	hasAnswered: boolean;
	answer: string | null;
}

// Game state managed by frontend
export interface GameState {
	phase: GamePhase;
	lobbyId?: string;
	joinCode?: string;
	playerId?: string;
	playerName?: string;
	isAdmin: boolean;
	roundDuration: number;
	players: Map<string, PlayerState>;
	currentQuestion?: {
		type: string;
		alternatives: string[];
	};
	error?: string;
	upcomingQuestions?: Array<{
		type: 'character' | 'color';
		song: string;
		artist?: string;
		spotify_url?: string;
		youtube_id: string;
		correct_character?: string;
		colors?: string[];
	}>;
	currentAnswers: PlayerAnswer[];
}

// Game question type
export interface GameQuestion {
	type: string;
	song_name: string;
	artist: string;
	youtube_id: string;
	alternatives: string[];
	correct_answer: string;
}

// Messages from server to client
export type ServerMessage =
	| {
		type: 'Joined';
		player_id: string;
		lobby_id: string;
		name: string;
		round_duration: number;
		current_players: [string, number][];  // Tuple of [name, score]
	}
	| {
		type: 'PlayerLeft';
		name: string;
	}
	| {
		type: 'PlayerAnswered';
		name: string;
		correct: boolean;
		new_score: number;
	}
	| {
		type: 'StateChanged';
		phase: GamePhase;
		question_type: string;
		alternatives: string[];
		scoreboard: [string, number][];
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
		type: 'AdminInfo';
		current_question: GameQuestion;
	}
	| {
		type: 'AdminNextQuestions';
		upcoming_questions: GameQuestion[];
	}
	| {
		type: 'Error';
		code: ErrorCode;
		message: string;
	};

// Messages from client to server
export type ClientMessage =
	| {
		type: 'JoinLobby';
		join_code: string;
		name: string;
		admin_id?: string;  // Only used when joining as admin
	}
	| {
		type: 'Leave';
		lobby_id: string;
	}
	| {
		type: 'Answer';
		lobby_id: string;
		answer: string;
	}
	| {
		type: 'AdminAction';
		lobby_id: string;
		action: AdminAction;
	};

// Admin actions that can be sent to server
export type AdminAction =
	| { type: 'StartGame' }
	| { type: 'StartRound'; specified_alternatives?: string[] }
	| { type: 'EndRound' }
	| { type: 'SkipQuestion' }
	| { type: 'EndGame'; reason: string }
	| { type: 'CloseGame'; reason: string };

// Name validation error types
export type NameValidationError =
	| 'TooShort'
	| 'TooLong'
	| 'InvalidCharacters'
	| 'AlreadyTaken';

// Helper function to get user-friendly error messages
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
