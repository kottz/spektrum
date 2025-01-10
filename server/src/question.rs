use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// This is the data model for the media table
// It represents a media item such as a song or a movie
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub release_year: Option<i32>,
    pub spotify_uri: Option<String>,
    pub youtube_id: String,
}

// The different types of question that are supported
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QuestionType {
    Color,
    Character,
    Text,
    Year,
}

// The question data model. A question has to have a media item associated with
// it and a type. is_active is used to enable/disable questions without removing them
// from the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: i64,
    pub media_id: i64,
    pub question_type: QuestionType,
    pub question_text: Option<String>,
    pub image_url: Option<String>,
    pub is_active: bool,
}

// Actual question options for a question. Each question can have multiple options
// that are either correct or incorrect. All different types are stored as text.
// Color options are stored as "Blue", "Red", etc. Character options are stored
// as the character name. Text options are stored as the text itself.
// Year options are stored as the year in text format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    pub id: i64,
    pub question_id: i64,
    pub option_text: String,
    pub is_correct: bool,
}

// This represents a complete question with all its related data
// This is not stored in the database but is used to represent a question
// in the engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameQuestion {
    pub question: Question,
    pub media: Media,
    pub options: Vec<QuestionOption>,
}

#[derive(Error, Debug)]
pub enum QuestionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("No questions found")]
    NoQuestions,

    #[error("Question set not found: {0}")]
    QuestionSetNotFound(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Color {
    Red,
    Green,
    Blue,
    Yellow,
    Purple,
    Gold,
    Silver,
    Pink,
    Black,
    White,
    Brown,
    Orange,
    Gray,
}

impl Color {
    pub fn all() -> &'static [Color] {
        use Color::*;
        &[
            Red, Green, Blue, Yellow, Purple, Gold, Silver, Pink, Black, White, Brown, Orange, Gray,
        ]
    }

    pub fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

// GameQuestion implementation that handles all question types
impl GameQuestion {
    pub fn new(question: Question, media: Media, options: Vec<QuestionOption>) -> Self {
        Self {
            question,
            media,
            options,
        }
    }

    // Common functionality for getting metadata
    pub fn get_id(&self) -> i64 {
        self.question.id
    }

    pub fn get_spotify_uri(&self) -> Option<String> {
        self.media.spotify_uri.clone()
    }

    pub fn get_youtube_id(&self) -> String {
        self.media.youtube_id.clone()
    }

    pub fn get_correct_options(&self) -> Vec<&QuestionOption> {
        self.options.iter().filter(|opt| opt.is_correct).collect()
    }

    // Generate alternatives based on question type
    pub fn generate_round_alternatives(&self) -> Vec<String> {
        let mut rng = rand::thread_rng();

        match self.question.question_type {
            QuestionType::Color => self.generate_color_alternatives(),
            QuestionType::Character => {
                // For character questions, use provided options with shuffling
                let mut alternatives: Vec<String> = self
                    .options
                    .iter()
                    .map(|opt| opt.option_text.clone())
                    .collect();
                alternatives.shuffle(&mut rng);
                alternatives
            }
            QuestionType::Year => {
                // Generate year alternatives around the correct year
                if let Some(year) = self.media.release_year {
                    self.generate_year_alternatives(year)
                } else {
                    vec![]
                }
            }
            QuestionType::Text => {
                // For text questions, use all provided options
                let mut alternatives: Vec<String> = self
                    .options
                    .iter()
                    .map(|opt| opt.option_text.clone())
                    .collect();
                alternatives.shuffle(&mut rng);
                alternatives
            }
        }
    }

    // Helper method for generating color alternatives
    fn generate_color_alternatives(&self) -> Vec<String> {
        let mut rng = rand::thread_rng();
        let correct_colors: Vec<Color> = self
            .get_correct_options()
            .iter()
            .filter_map(|opt| {
                // Try to parse the color from option text
                match opt.option_text.parse::<Color>() {
                    Ok(color) => Some(color),
                    Err(_) => None,
                }
            })
            .collect();

        let mut round_colors = correct_colors.clone();

        // If we have enough colors, just shuffle and return
        if round_colors.len() >= 6 {
            round_colors.shuffle(&mut rng);
            return round_colors.iter().map(|c| c.to_string()).collect();
        }

        // Get available colors (excluding ones we already have)
        let mut available_colors: Vec<Color> = Color::all()
            .iter()
            .copied()
            .filter(|color| !round_colors.contains(color))
            .collect();

        available_colors.shuffle(&mut rng);

        // Add random colors until we have 6
        round_colors.extend(available_colors.iter().take(6 - round_colors.len()));
        round_colors.shuffle(&mut rng);

        round_colors.iter().map(|c| c.to_string()).collect()
    }

    // Helper method for generating year alternatives
    fn generate_year_alternatives(&self, correct_year: i32) -> Vec<String> {
        let mut alternatives = vec![
            correct_year - 2,
            correct_year - 1,
            correct_year,
            correct_year + 1,
            correct_year + 2,
        ];
        alternatives.shuffle(&mut rand::thread_rng());
        alternatives.iter().map(|y| y.to_string()).collect()
    }

    // Validate an answer based on question type
    pub fn validate_answer(&self, answer: &str) -> bool {
        match self.question.question_type {
            QuestionType::Year => {
                // For year questions, check if within acceptable range
                if let (Some(correct_year), Ok(answered_year)) =
                    (self.media.release_year, answer.parse::<i32>())
                {
                    (answered_year - correct_year).abs() <= 2 // Allow 2 years difference
                } else {
                    false
                }
            }
            QuestionType::Color | QuestionType::Character | QuestionType::Text => {
                // For other types, check if matches any correct option
                self.get_correct_options()
                    .iter()
                    .any(|opt| opt.option_text == answer)
            }
        }
    }
}

// Add implementation for Color parsing if needed
impl std::str::FromStr for Color {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_uppercase().as_str() {
            "RED" => Ok(Color::Red),
            "GREEN" => Ok(Color::Green),
            "BLUE" => Ok(Color::Blue),
            "YELLOW" => Ok(Color::Yellow),
            "PURPLE" => Ok(Color::Purple),
            "GOLD" => Ok(Color::Gold),
            "SILVER" => Ok(Color::Silver),
            "PINK" => Ok(Color::Pink),
            "BLACK" => Ok(Color::Black),
            "WHITE" => Ok(Color::White),
            "BROWN" => Ok(Color::Brown),
            "ORANGE" => Ok(Color::Orange),
            "GRAY" | "GREY" => Ok(Color::Gray),
            _ => Err(format!("Invalid color: {}", s)),
        }
    }
}
