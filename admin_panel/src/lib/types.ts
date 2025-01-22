export enum QuestionType {
	Color = 'color',
	Character = 'character',
	Text = 'text',
	Year = 'year'
}

export enum Color {
	Red = 'Red',
	Green = 'Green',
	Blue = 'Blue',
	Yellow = 'Yellow',
	Purple = 'Purple',
	Gold = 'Gold',
	Silver = 'Silver',
	Pink = 'Pink',
	Black = 'Black',
	White = 'White',
	Brown = 'Brown',
	Orange = 'Orange',
	Gray = 'Gray'
}

export interface Media {
	id: number;
	title: string;
	artist: string;
	release_year: number | null;
	spotify_uri: string | null;
	youtube_id: string;
}

export interface Character {
	id: number;
	name: string;
	image_url: string;
}

export interface Question {
	id: number;
	media_id: number;
	question_type: QuestionType;
	question_text: string | null;
	image_url: string | null;
	is_active: boolean;
}

export interface QuestionOption {
	id: number;
	question_id: number;
	option_text: string;
	is_correct: boolean;
}

export interface QuestionSet {
	id: number;
	name: string;
	question_ids: number[];
}

export interface StoredData {
	media: Media[];
	characters: Character[];
	questions: Question[];
	options: QuestionOption[];
	sets: QuestionSet[];
}
