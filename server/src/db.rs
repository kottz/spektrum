use crate::question::{Color, GameQuestion, GameQuestionOption, QuestionType, COLOR_WEIGHTS};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub(crate) enum DbError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("No questions found")]
    NoQuestions,
    #[error("Validation error: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Media {
    id: i64,
    title: String,
    artist: String,
    release_year: Option<i32>,
    spotify_uri: Option<String>,
    youtube_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Character {
    id: i64,
    name: String,
    image_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Question {
    id: i64,
    media_id: i64,
    question_type: QuestionType,
    question_text: Option<String>,
    image_url: Option<String>,
    is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QuestionOption {
    id: i64,
    question_id: i64,
    option_text: String,
    is_correct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionSet {
    pub id: i64,
    pub name: String,
    pub question_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredData {
    media: Vec<Media>,
    characters: Vec<Character>,
    questions: Vec<Question>,
    options: Vec<QuestionOption>,
    sets: Vec<QuestionSet>,
}

/// Validates the integrity of the stored data by checking:
/// - Duplicate IDs of any type (media, characters, questions, options, sets)
/// - Duplicate character names or image URLs
/// - Questions referencing non-existent media IDs
/// - Options referencing non-existent questions
/// - Options referencing non-existent character names (via option_text)
/// - Sets referencing non-existent questions
///
/// Returns Ok(()) if all validations pass, or a DbError::Validation with detailed error message.
pub fn validate_stored_data(data: &StoredData) -> Result<(), DbError> {
    let mut seen_media_ids = HashSet::new();
    for media in &data.media {
        if !seen_media_ids.insert(media.id) {
            return Err(DbError::Validation(format!(
                "Duplicate media ID: {}",
                media.id
            )));
        }
    }

    let mut seen_character_ids = HashSet::new();
    let mut seen_names = HashSet::new();
    let mut seen_urls = HashSet::new();
    for character in &data.characters {
        if !seen_character_ids.insert(character.id) {
            return Err(DbError::Validation(format!(
                "Duplicate character ID: {}",
                character.id
            )));
        }
        if !seen_names.insert(&character.name) {
            return Err(DbError::Validation(format!(
                "Duplicate character name: {}",
                character.name
            )));
        }
        if !seen_urls.insert(&character.image_url) {
            return Err(DbError::Validation(format!(
                "Duplicate character image URL: {}",
                character.image_url
            )));
        }
    }

    let mut seen_question_ids = HashSet::new();
    for question in &data.questions {
        if !seen_question_ids.insert(question.id) {
            return Err(DbError::Validation(format!(
                "Duplicate question ID: {}",
                question.id
            )));
        }
    }

    let mut seen_option_ids = HashSet::new();
    for option in &data.options {
        if !seen_option_ids.insert(option.id) {
            return Err(DbError::Validation(format!(
                "Duplicate option ID: {}",
                option.id
            )));
        }
    }

    let mut seen_set_ids = HashSet::new();
    for set in &data.sets {
        if !seen_set_ids.insert(set.id) {
            return Err(DbError::Validation(format!("Duplicate set ID: {}", set.id)));
        }
    }

    let character_names = seen_names;
    let media_ids = seen_media_ids;
    let question_ids = seen_question_ids;

    // Create a HashSet of valid color names
    let valid_colors: HashSet<String> = Color::all().iter().map(|c| c.to_string()).collect();

    for question in &data.questions {
        if !media_ids.contains(&question.media_id) {
            return Err(DbError::Validation(format!(
                "Question {} references non-existent media ID {}",
                question.id, question.media_id
            )));
        }
    }

    for option in &data.options {
        if !question_ids.contains(&option.question_id) {
            return Err(DbError::Validation(format!(
                "Option {} references non-existent question ID {}",
                option.id, option.question_id
            )));
        }

        // Get the question type for this option
        let question_type = data
            .questions
            .iter()
            .find(|q| q.id == option.question_id)
            .map(|q| &q.question_type)
            .ok_or_else(|| {
                DbError::Validation(format!("Question not found for option {}", option.id))
            })?;

        match question_type {
            QuestionType::Color => {
                if !valid_colors.contains(&option.option_text) {
                    return Err(DbError::Validation(format!(
                        "Option {} references invalid color name '{}'",
                        option.id, option.option_text
                    )));
                }
            }
            _ => {
                if !character_names.contains(&option.option_text) {
                    return Err(DbError::Validation(format!(
                        "Option {} references non-existent character name '{}'",
                        option.id, option.option_text
                    )));
                }
            }
        }
    }

    for set in &data.sets {
        for &qid in &set.question_ids {
            if !question_ids.contains(&qid) {
                return Err(DbError::Validation(format!(
                    "Set {} references non-existent question ID {}",
                    set.id, qid
                )));
            }
        }
    }

    Ok(())
}

pub struct QuestionDatabase {
    file_path: String,
}

impl QuestionDatabase {
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
        }
    }

    pub fn read_stored_data(&self) -> Result<StoredData, DbError> {
        if Path::new(&self.file_path).exists() {
            let content = fs::read_to_string(&self.file_path)?;
            let json_content = serde_json::from_str(&content)?;
            validate_stored_data(&json_content)?;
            Ok(json_content)
        } else {
            Ok(StoredData {
                media: Vec::new(),
                characters: Vec::new(),
                questions: Vec::new(),
                options: Vec::new(),
                sets: Vec::new(),
            })
        }
    }

    pub fn set_stored_data(&self, data: StoredData) -> Result<(), DbError> {
        let json = serde_json::to_string_pretty(&data)?;
        validate_stored_data(&data)?;
        let mut path = self.file_path.clone();
        path.push_str("_from_web.json");
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_questions(&self) -> Result<(Vec<GameQuestion>, Vec<QuestionSet>), DbError> {
        let stored_data = self.read_stored_data()?;
        validate_stored_data(&stored_data)?;

        let game_questions: Vec<GameQuestion> = stored_data
            .questions
            .iter()
            .filter(|q| q.is_active)
            .filter_map(|question| {
                let media = stored_data
                    .media
                    .iter()
                    .find(|m| m.id == question.media_id)?;

                let options = stored_data
                    .options
                    .iter()
                    .filter(|o| o.question_id == question.id)
                    .map(|opt| GameQuestionOption {
                        option: opt.option_text.clone(),
                        is_correct: opt.is_correct,
                    })
                    .collect();

                Some(GameQuestion {
                    id: question.id as u16,
                    question_type: question.question_type,
                    title: media.title.clone(),
                    artist: Some(media.artist.clone()),
                    youtube_id: media.youtube_id.clone(),
                    options,
                })
            })
            .collect();

        if game_questions.is_empty() {
            return Err(DbError::NoQuestions);
        }

        let weights = Self::calculate_color_weights(&game_questions);
        if let Ok(mut stored_weights) = COLOR_WEIGHTS.lock() {
            *stored_weights = weights;
        }

        info!("Loaded {} questions", game_questions.len());
        Ok((game_questions, stored_data.sets))
    }

    fn calculate_color_weights(questions: &[GameQuestion]) -> HashMap<Color, f64> {
        let mut color_counts: HashMap<Color, usize> = HashMap::new();
        let total_questions = questions.len();

        // Count color occurrences
        for question in questions {
            if question.question_type == QuestionType::Color {
                for option in question.get_correct_options() {
                    if let Ok(color) = option.option.parse::<Color>() {
                        *color_counts.entry(color).or_insert(0) += 1;
                    }
                }
            }
        }

        // Calculate weights using the same formula as before
        let mut weights = HashMap::new();
        for &color in Color::all() {
            let count = color_counts.get(&color).copied().unwrap_or(0);
            let base_proportion = if total_questions > 0 {
                count as f64 / total_questions as f64
            } else {
                0.0
            };
            let weight = base_proportion.sqrt() + 0.15; // Same formula as original
            weights.insert(color, weight);
        }

        weights
    }
}
