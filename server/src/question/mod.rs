pub mod character;
pub mod color;

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Core trait that all question types must implement
pub trait Question: Debug + Send + Sync {
    /// Get the unique identifier for this question
    fn get_id(&self) -> u32;

    /// Get the correct answer for this question
    fn get_correct_answer(&self) -> Vec<String>;

    /// Get the specific alternatives for this question instance
    fn get_all_possible_alternatives(&self) -> Vec<String>;

    /// Generate alternatives for this round, including the correct answer
    fn generate_round_alternatives(&self) -> Vec<String>;

    /// Get the Spotify URI for the song if available
    fn get_spotify_uri(&self) -> String;

    /// Get the YouTube ID for the song if available
    fn get_youtube_id(&self) -> String;
}

/// Enum representing all possible question types in the game
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GameQuestion {
    #[serde(rename = "color")]
    Color(color::ColorQuestion),
    #[serde(rename = "character")]
    Character(character::CharacterQuestion),
}

impl GameQuestion {
    /// Get the specific question type for this instance
    pub fn get_question_type(&self) -> &str {
        match self {
            GameQuestion::Color(_) => "color",
            GameQuestion::Character(_) => "character",
        }
    }
}

impl Question for GameQuestion {
    fn get_id(&self) -> u32 {
        match self {
            GameQuestion::Color(q) => q.get_id(),
            GameQuestion::Character(q) => q.get_id(),
        }
    }

    fn get_correct_answer(&self) -> Vec<String> {
        match self {
            GameQuestion::Color(q) => q.get_correct_answer(),
            GameQuestion::Character(q) => q.get_correct_answer(),
        }
    }

    fn get_all_possible_alternatives(&self) -> Vec<String> {
        match self {
            GameQuestion::Color(q) => q.get_all_possible_alternatives(),
            GameQuestion::Character(q) => q.get_all_possible_alternatives(),
        }
    }

    fn generate_round_alternatives(&self) -> Vec<String> {
        match self {
            GameQuestion::Color(q) => q.generate_round_alternatives(),
            GameQuestion::Character(q) => q.generate_round_alternatives(),
        }
    }

    fn get_spotify_uri(&self) -> String {
        match self {
            GameQuestion::Color(q) => q.get_spotify_uri(),
            GameQuestion::Character(q) => q.get_spotify_uri(),
        }
    }

    fn get_youtube_id(&self) -> String {
        match self {
            GameQuestion::Color(q) => q.get_youtube_id(),
            GameQuestion::Character(q) => q.get_youtube_id(),
        }
    }
}

/// Shared error type for question loading and processing
#[derive(Debug, thiserror::Error)]
pub enum QuestionError {
    #[error("CSV error: {0}")]
    CsvError(#[from] csv::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid question format: {0}")]
    InvalidFormat(String),
}

/// Result type alias for question operations
pub type QuestionResult<T> = Result<T, QuestionError>;

/// Common metadata shared by all question types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionMetadata {
    pub id: u32,
    pub spotify_uri: Option<String>,
    pub youtube_id: Option<String>,
}

/// Function to load questions from a CSV file
pub fn load_questions_from_csv(filepath: &str) -> QuestionResult<Vec<GameQuestion>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(filepath)?;

    let mut questions = Vec::new();
    let mut color_questions = Vec::new();

    // Read headers to determine question type
    let headers = reader.headers()?;
    let question_type = determine_question_type(headers)?;

    for result in reader.records() {
        let record = result?;
        let question = match question_type {
            QuestionType::Color => {
                let color_q = color::load_from_record(&record)?;
                color_questions.push(color_q.clone());
                GameQuestion::Color(color_q)
            }
            QuestionType::Character => {
                GameQuestion::Character(character::load_from_record(&record)?)
            }
        };
        questions.push(question);
    }

    // Initialize color weights if we loaded any color questions
    if !color_questions.is_empty() {
        color::initialize_color_weights(&color_questions)?;
    }

    Ok(questions)
}

#[derive(Debug, Clone, Copy)]
enum QuestionType {
    Color,
    Character,
}

fn determine_question_type(headers: &csv::StringRecord) -> QuestionResult<QuestionType> {
    let headers: Vec<_> = headers.iter().collect();
    match headers.as_slice() {
        ["id", "title", "artist", "color", "spotify_uri", "youtube_id"] => Ok(QuestionType::Color),
        ["id", "difficulty", "song", "correct_character", "other_characters", "spotify_uri", "youtube_id"] => {
            Ok(QuestionType::Character)
        }
        _ => Err(QuestionError::InvalidFormat("Unknown CSV format".into())),
    }
}
