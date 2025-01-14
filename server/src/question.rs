use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

use crate::db::{QuestionDatabase, QuestionSet};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

    pub fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

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
        let mut rng = rand::thread_rng();
        let correct_colors: Vec<Color> = self
            .get_correct_options()
            .iter()
            .filter_map(|opt| opt.option.parse::<Color>().ok())
            .collect();

        let mut round_colors = correct_colors.clone();

        if round_colors.len() >= 6 {
            round_colors.shuffle(&mut rng);
            return round_colors.iter().map(|c| c.to_string()).collect();
        }

        let mut available_colors: Vec<Color> = Color::all()
            .iter()
            .copied()
            .filter(|color| !round_colors.contains(color))
            .collect();

        available_colors.shuffle(&mut rng);
        round_colors.extend(available_colors.iter().take(6 - round_colors.len()));
        round_colors.shuffle(&mut rng);

        round_colors.iter().map(|c| c.to_string()).collect()
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

pub struct QuestionManager {
    questions: RwLock<Arc<Vec<GameQuestion>>>,
    sets: RwLock<Arc<Vec<QuestionSet>>>,
    db: QuestionDatabase,
}

impl QuestionManager {
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
