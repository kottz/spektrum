export enum QuestionType {
    Color = 'color',
    Character = 'character',
    Text = 'text',
    Year = 'year'
}

export interface Media {
    id: number;
    title: string;
    artist: string;
    release_year: number | null;
    spotify_uri: string | null;
    youtube_id: string;
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
    questions: Question[];
    options: QuestionOption[];
    sets: QuestionSet[];
}
