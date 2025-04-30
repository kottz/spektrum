use crate::db::{DbError, StoredData};
use crate::db::{QuestionDatabase, QuestionSet};
use crate::StorageConfig;
use dashmap::DashMap;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

lazy_static! {
    pub(crate) static ref COLOR_WEIGHTS: DashMap<Color, f64> = DashMap::new();
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
        use Color::{
            Black, Blue, Brown, Gold, Gray, Green, Orange, Pink, Purple, Red, Silver, White, Yellow,
        };
        &[
            Red, Green, Blue, Yellow, Purple, Gold, Silver, Pink, Black, White, Brown, Orange, Gray,
        ]
    }

    fn get_weight(&self) -> f64 {
        COLOR_WEIGHTS.get(self).map(|v| *v).unwrap_or(0.15)
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            Color::Red => "Red",
            Color::Green => "Green",
            Color::Blue => "Blue",
            Color::Yellow => "Yellow",
            Color::Purple => "Purple",
            Color::Gold => "Gold",
            Color::Silver => "Silver",
            Color::Pink => "Pink",
            Color::Black => "Black",
            Color::White => "White",
            Color::Brown => "Brown",
            Color::Orange => "Orange",
            Color::Gray => "Gray",
        };
        write!(f, "{}", s)
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

    #[error("Database error: {0}")]
    DbError(#[from] crate::db::DbError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameQuestionOption {
    pub option: String,
    pub is_correct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameQuestion {
    pub id: u16,
    pub question_type: QuestionType,
    pub question_text: Option<String>,
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
        match self.question_type {
            QuestionType::Color => self.generate_color_alternatives(),
            QuestionType::Character | QuestionType::Text => {
                let mut alternatives: Vec<String> =
                    self.options.iter().map(|opt| opt.option.clone()).collect();
                fastrand::shuffle(&mut alternatives);
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
                let idx = fastrand::usize(..available_colors.len());
                let (color, _) = available_colors.remove(idx);
                round_colors.push(color);
                continue;
            }

            let mut selection = fastrand::f64() * total_weight;
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

        fastrand::shuffle(&mut round_colors);
        round_colors.into_iter().map(|c| c.to_string()).collect()
    }

    fn generate_year_alternatives(&self, correct_year: i32) -> Vec<String> {
        let mut alternatives = [
            correct_year - 2,
            correct_year - 1,
            correct_year,
            correct_year + 1,
            correct_year + 2,
        ];
        fastrand::shuffle(&mut alternatives);
        alternatives.iter().map(|y| y.to_string()).collect()
    }
}

pub struct QuestionStore {
    pub questions: RwLock<Arc<Vec<GameQuestion>>>,
    pub sets: RwLock<Arc<Vec<QuestionSet>>>,
    db: QuestionDatabase,
}

impl QuestionStore {
    pub async fn new(config: &StorageConfig) -> Result<Self, QuestionError> {
        let db = QuestionDatabase::new(config).map_err(QuestionError::DbError)?;

        let store = Self {
            questions: RwLock::new(Arc::new(Vec::new())),
            sets: RwLock::new(Arc::new(Vec::new())),
            db,
        };
        store.load_questions().await?;
        Ok(store)
    }

    pub async fn load_questions(&self) -> Result<(), QuestionError> {
        let (game_questions, sets) = self
            .db
            .load_questions()
            .await
            .map_err(QuestionError::DbError)?;

        if game_questions.is_empty() {
            return Err(QuestionError::NoQuestions);
        }
        *self.questions.write().await = Arc::new(game_questions);
        *self.sets.write().await = Arc::new(sets);
        Ok(())
    }

    pub async fn get_stored_data(&self) -> Result<StoredData, DbError> {
        self.db.read_stored_data().await
    }

    pub async fn set_stored_data(&self, stored_data: StoredData) -> Result<(), DbError> {
        self.db.set_stored_data(stored_data).await
    }

    pub async fn backup_stored_data(&self) -> Result<(), DbError> {
        self.db.backup_stored_data().await
    }

    pub async fn store_character_image(
        &self,
        character_name: &str,
        data: &[u8],
    ) -> Result<String, DbError> {
        self.db.store_character_image(character_name, data).await
    }
}
