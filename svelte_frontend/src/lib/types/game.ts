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
	hasAnswered: boolean;
	answer: string | null;
}

/**
 * Frontend store for the overall game state.
 */
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
	currentSong?: {
		songName: string;
		artist: string;
		youtubeId: string;
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

/**
 * Represents a single quiz question.
 */
export interface GameQuestion {
	type: string;
	song_name: string;
	artist: string;
	youtube_id: string;
	alternatives: string[];
	correct_answer: string;
}

/* ------------------------------------------------------------------
   SERVER -> CLIENT MESSAGES
------------------------------------------------------------------ */

/**
 * Extended server messages reflecting the new server.rs protocol.
 */
export type ServerMessage =
	| {
		// Sent when the client has successfully joined a lobby
		type: 'JoinedLobby';
		player_id: string;
		lobby_id: string;
		name: string;
		round_duration: number;
		players: [string, number][]; // Tuple [playerName, score]
	}
	| {
		// Sent when a previously connected player is successfully reconnected
		type: 'ReconnectSuccess';
		game_state: {
			phase: string; // same as GamePhase, but might come as a string
			question_type: string;
			alternatives: string[];
			scoreboard: [string, number][]; // [playerName, score]
			current_song?: {
				song_name: string;
				artist: string;
				youtube_id: string;
			};
		};
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
		phase: string; // e.g. 'lobby', 'score', 'question', 'gameover'
		question_type: string;
		alternatives: string[];
		scoreboard: [string, number][]; // [playerName, score]
	}
	| {
		type: 'GameOver';
		scores: [string, number][]; // final scores
		reason: string;
	}
	| {
		type: 'GameClosed';
		reason: string;
	}
	| {
		type: 'AdminInfo';
		// Provides extra info about a question to the admin
		question: GameQuestion;
	}
	| {
		type: 'AdminNextQuestions';
		// Provides upcoming questions (could be used for preview, etc.)
		upcoming_questions: GameQuestion[];
	}
	| {
		type: 'Error';
		code: ErrorCode;
		message: string;
	};

/* ------------------------------------------------------------------
   CLIENT -> SERVER MESSAGES
------------------------------------------------------------------ */

export type ClientMessage =
	| {
		type: 'JoinLobby';
		join_code: string;
		name: string;
		admin_id?: string; // Only if joining as admin
	}
	| {
		type: 'Reconnect';
		lobby_id: string;
		player_id: string;
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

/**
 * Administrative actions that can be performed in the lobby.
 */
export type AdminAction =
	| { type: 'StartGame' }
	| { type: 'StartRound'; specified_alternatives?: string[] }
	| { type: 'EndRound' }
	| { type: 'SkipQuestion' }
	| { type: 'EndGame'; reason: string }
	| { type: 'CloseGame'; reason: string };

/**
 * Common name validation errors that might be returned by the server or client.
 */
export type NameValidationError =
	| 'TooShort'
	| 'TooLong'
	| 'InvalidCharacters'
	| 'AlreadyTaken';

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
