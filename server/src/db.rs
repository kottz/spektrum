use crate::question::{Color, GameQuestion, GameQuestionOption, QuestionType, COLOR_WEIGHTS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
struct StoredData {
    media: Vec<Media>,
    questions: Vec<Question>,
    options: Vec<QuestionOption>,
    sets: Vec<QuestionSet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionSet {
    pub id: i64,
    pub name: String,
    pub question_ids: Vec<i64>,
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

    fn read_stored_data(&self) -> Result<StoredData, DbError> {
        if Path::new(&self.file_path).exists() {
            let content = fs::read_to_string(&self.file_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(StoredData {
                media: Vec::new(),
                questions: Vec::new(),
                options: Vec::new(),
                sets: Vec::new(),
            })
        }
    }

    pub fn load_questions(&self) -> Result<(Vec<GameQuestion>, Vec<QuestionSet>), DbError> {
        let stored_data = self.read_stored_data()?;

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

    pub fn add_question(&self, new_question: QuestionInput) -> Result<(), DbError> {
        let mut stored_data = self.read_stored_data()?;

        let media_id = stored_data.media.iter().map(|m| m.id).max().unwrap_or(0) + 1;

        let question_id = stored_data
            .questions
            .iter()
            .map(|q| q.id)
            .max()
            .unwrap_or(0)
            + 1;

        let option_id = stored_data.options.iter().map(|o| o.id).max().unwrap_or(0) + 1;

        let media = Media {
            id: media_id,
            title: new_question.title,
            artist: new_question.artist,
            release_year: new_question.release_year,
            spotify_uri: new_question.spotify_uri,
            youtube_id: new_question.youtube_id,
        };

        let question = Question {
            id: question_id,
            media_id,
            question_type: new_question.question_type,
            question_text: new_question.question_text,
            image_url: new_question.image_url,
            is_active: true,
        };

        // Generate IDs for options
        let options: Vec<QuestionOption> = new_question
            .options
            .into_iter()
            .enumerate()
            .map(|(i, opt)| QuestionOption {
                id: option_id + i as i64,
                question_id,
                option_text: opt.option_text,
                is_correct: opt.is_correct,
            })
            .collect();

        stored_data.media.push(media);
        stored_data.questions.push(question);
        stored_data.options.extend(options);

        let json = serde_json::to_string_pretty(&stored_data)?;
        fs::write(&self.file_path, json)?;

        Ok(())
    }

    pub fn update_question(
        &self,
        question_id: i64,
        update_data: QuestionInput,
    ) -> Result<(), DbError> {
        let mut stored_data = self.read_stored_data()?;

        // Update question
        if let Some(question) = stored_data
            .questions
            .iter_mut()
            .find(|q| q.id == question_id)
        {
            question.question_type = update_data.question_type;
            question.question_text = update_data.question_text;
            question.image_url = update_data.image_url;
        }

        // Update media
        if let Some(media) = stored_data
            .media
            .iter_mut()
            .find(|m| m.id == update_data.media_id)
        {
            media.title = update_data.title;
            media.artist = update_data.artist;
            media.release_year = update_data.release_year;
            media.spotify_uri = update_data.spotify_uri;
            media.youtube_id = update_data.youtube_id;
        }

        // Update options
        stored_data.options.retain(|o| o.question_id != question_id);
        stored_data.options.extend(update_data.options);

        let json = serde_json::to_string_pretty(&stored_data)?;
        fs::write(&self.file_path, json)?;

        Ok(())
    }

    pub fn remove_question(&self, question_id: i64) -> Result<(), DbError> {
        let mut stored_data = self.read_stored_data()?;

        if let Some(question) = stored_data
            .questions
            .iter_mut()
            .find(|q| q.id == question_id)
        {
            question.is_active = false;

            let json = serde_json::to_string_pretty(&stored_data)?;
            fs::write(&self.file_path, json)?;

            Ok(())
        } else {
            Err(DbError::NoQuestions)
        }
    }
}

// Helper struct for adding/updating questions
#[derive(Debug)]
pub struct QuestionInput {
    pub question_id: i64,
    pub media_id: i64,
    pub question_type: QuestionType,
    pub question_text: Option<String>,
    pub image_url: Option<String>,
    pub title: String,
    pub artist: String,
    pub release_year: Option<i32>,
    pub spotify_uri: Option<String>,
    pub youtube_id: String,
    pub options: Vec<QuestionOption>,
}
