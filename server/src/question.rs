use crate::db::{DbError, StoredData};
use crate::db::{QuestionDatabase, QuestionSet};
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use thiserror::Error;
use tokio::sync::RwLock;

lazy_static! {
    pub(crate) static ref COLOR_WEIGHTS: Mutex<HashMap<Color, f64>> = Mutex::new(HashMap::new());
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuestionType {
    Color,
    Character,
    Text,
    Year,
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

    fn get_weight(&self) -> f64 {
        COLOR_WEIGHTS
            .lock()
            .map(|weights| weights.get(self).copied().unwrap_or(0.15))
            .unwrap_or(0.15) // Fallback weight if lock fails
    }

    pub fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

impl std::str::FromStr for Color {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "Red" => Ok(Color::Red),
            "Green" => Ok(Color::Green),
            "Blue" => Ok(Color::Blue),
            "Yellow" => Ok(Color::Yellow),
            "Purple" => Ok(Color::Purple),
            "Gold" => Ok(Color::Gold),
            "Silver" => Ok(Color::Silver),
            "Pink" => Ok(Color::Pink),
            "Black" => Ok(Color::Black),
            "White" => Ok(Color::White),
            "Brown" => Ok(Color::Brown),
            "Orange" => Ok(Color::Orange),
            "Gray" | "Grey" => Ok(Color::Gray),
            _ => Err(format!("Invalid color: {}", s)),
        }
    }
}

#[derive(Error, Debug)]
pub enum QuestionError {
    #[error("No questions found")]
    NoQuestions,

    #[error("Question set not found: {0}")]
    QuestionSetNotFound(i64),

    #[error("Database error: {0}")]
    DbError(#[from] crate::db::DbError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameQuestionOption {
    pub option: String,
    pub is_correct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameQuestion {
    pub id: u16,
    pub question_type: QuestionType,
    pub title: String,
    pub artist: Option<String>,
    pub youtube_id: String,
    pub options: Vec<GameQuestionOption>,
}

impl GameQuestion {
    pub fn get_correct_options(&self) -> Vec<&GameQuestionOption> {
        self.options.iter().filter(|opt| opt.is_correct).collect()
    }

    pub fn get_question_type(&self) -> &str {
        match self.question_type {
            QuestionType::Color => "color",
            QuestionType::Character => "character",
            QuestionType::Text => "text",
            QuestionType::Year => "year",
        }
    }

    pub fn get_correct_answer(&self) -> Vec<String> {
        self.get_correct_options()
            .iter()
            .map(|opt| opt.option.clone())
            .collect()
    }

    pub fn generate_round_alternatives(&self) -> Vec<String> {
        let mut rng = rand::thread_rng();

        match self.question_type {
            QuestionType::Color => self.generate_color_alternatives(),
            QuestionType::Character | QuestionType::Text => {
                let mut alternatives: Vec<String> =
                    self.options.iter().map(|opt| opt.option.clone()).collect();
                alternatives.shuffle(&mut rng);
                alternatives
            }
            QuestionType::Year => {
                if let Some(year) = self
                    .get_correct_answer()
                    .first()
                    .and_then(|y| y.parse().ok())
                {
                    self.generate_year_alternatives(year)
                } else {
                    vec![]
                }
            }
        }
    }

    fn generate_color_alternatives(&self) -> Vec<String> {
        const TARGET_SIZE: usize = 6;
        let mut rng = rand::thread_rng();

        // Get initial colors from correct options
        let mut round_colors: Vec<Color> = self
            .get_correct_options()
            .iter()
            .filter_map(|opt| opt.option.parse().ok())
            .collect();

        // Get available colors (excluding ones we already have)
        let mut available_colors: Vec<(Color, f64)> = Color::all()
            .iter()
            .copied()
            .filter(|color| !round_colors.contains(color))
            .map(|color| (color, color.get_weight()))
            .collect();

        // Select additional colors based on weights until we have TARGET_SIZE
        while round_colors.len() < TARGET_SIZE && !available_colors.is_empty() {
            let total_weight: f64 = available_colors.iter().map(|(_, w)| w).sum();

            if total_weight <= 0.0 {
                // Fallback to random selection if weights are invalid
                let idx = rng.gen_range(0..available_colors.len());
                let (color, _) = available_colors.remove(idx);
                round_colors.push(color);
                continue;
            }

            let mut selection = rng.gen_range(0.0..total_weight);
            let mut selected_idx = 0;

            for (idx, (_, weight)) in available_colors.iter().enumerate() {
                selection -= weight;
                if selection <= 0.0 {
                    selected_idx = idx;
                    break;
                }
            }

            let (color, _) = available_colors.remove(selected_idx);
            round_colors.push(color);
        }

        round_colors.shuffle(&mut rng);
        round_colors.into_iter().map(|c| c.to_string()).collect()
    }

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

    pub fn validate_answer(&self, answer: &str) -> bool {
        match self.question_type {
            QuestionType::Year => {
                if let (Some(correct_year), Ok(answered_year)) = (
                    self.get_correct_answer()
                        .first()
                        .and_then(|y| y.parse::<i32>().ok()),
                    answer.parse::<i32>(),
                ) {
                    correct_year == answered_year
                } else {
                    false
                }
            }
            QuestionType::Color | QuestionType::Character | QuestionType::Text => self
                .get_correct_options()
                .iter()
                .any(|opt| opt.option == answer),
        }
    }
}

pub struct QuestionStore {
    pub questions: RwLock<Arc<Vec<GameQuestion>>>,
    sets: RwLock<Arc<Vec<QuestionSet>>>,
    db: QuestionDatabase,
}

impl QuestionStore {
    pub async fn new(file_path: &str) -> Result<Self, QuestionError> {
        let db = QuestionDatabase::new(file_path);
        let manager = Self {
            questions: RwLock::new(Arc::new(Vec::new())),
            sets: RwLock::new(Arc::new(Vec::new())),
            db,
        };

        manager.load_questions().await?;
        Ok(manager)
    }

    pub async fn load_questions(&self) -> Result<(), QuestionError> {
        let (game_questions, sets) = self.db.load_questions()?;

        if game_questions.is_empty() {
            return Err(QuestionError::NoQuestions);
        }

        *self.questions.write().await = Arc::new(game_questions);
        *self.sets.write().await = Arc::new(sets);

        Ok(())
    }

    pub fn get_stored_data(&self) -> Result<StoredData, DbError> {
        self.db.read_stored_data()
    }

    pub async fn get_questions(&self) -> Result<Arc<Vec<GameQuestion>>, QuestionError> {
        let questions = self.questions.read().await;
        if Arc::strong_count(&questions) == 0 {
            return Err(QuestionError::NoQuestions);
        }
        Ok(Arc::clone(&questions))
    }

    pub async fn get_question_set(
        &self,
        set_id: i64,
    ) -> Result<Arc<Vec<GameQuestion>>, QuestionError> {
        let sets = self.sets.read().await;
        let questions = self.questions.read().await;

        let set = sets
            .iter()
            .find(|s| s.id == set_id)
            .ok_or(QuestionError::QuestionSetNotFound(set_id))?;

        let set_questions: Vec<GameQuestion> = questions
            .iter()
            .filter(|q| set.question_ids.contains(&i64::from(q.id)))
            .cloned()
            .collect();

        if set_questions.is_empty() {
            return Err(QuestionError::NoQuestions);
        }

        Ok(Arc::new(set_questions))
    }
}
