use crate::StorageConfig;
use crate::db::{DbError, QuestionDatabase, QuestionSet, StoredData};
use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

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
    pub const COUNT: usize = 13;

    pub fn all() -> &'static [Color] {
        use Color::{
            Black, Blue, Brown, Gold, Gray, Green, Orange, Pink, Purple, Red, Silver, White, Yellow,
        };
        &[
            Red, Green, Blue, Yellow, Purple, Gold, Silver, Pink, Black, White, Brown, Orange, Gray,
        ]
    }

    /// Canonical dense index for array-backed color data. Do not assume Color::all() order matches.
    pub fn idx(self) -> usize {
        match self {
            Color::Red => 0,
            Color::Green => 1,
            Color::Blue => 2,
            Color::Yellow => 3,
            Color::Purple => 4,
            Color::Gold => 5,
            Color::Silver => 6,
            Color::Pink => 7,
            Color::Black => 8,
            Color::White => 9,
            Color::Brown => 10,
            Color::Orange => 11,
            Color::Gray => 12,
        }
    }
}

pub fn baseline_weights() -> [f64; Color::COUNT] {
    debug_assert_eq!(
        Color::COUNT,
        Color::all().len(),
        "Color::COUNT must match Color::all().len()"
    );
    [0.15; Color::COUNT]
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
    pub option: Arc<str>,
    pub is_correct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameQuestion {
    pub id: u16,
    pub question_type: QuestionType,
    pub question_text: Option<Arc<str>>,
    pub title: Arc<str>,
    pub artist: Option<Arc<str>>,
    pub youtube_id: Arc<str>,
    pub options: Vec<GameQuestionOption>,
}

impl GameQuestion {
    pub fn get_correct_options(&self) -> Vec<&GameQuestionOption> {
        self.options.iter().filter(|opt| opt.is_correct).collect()
    }

    pub fn get_question_type(&self) -> &'static str {
        match self.question_type {
            QuestionType::Color => "color",
            QuestionType::Character => "character",
            QuestionType::Text => "text",
            QuestionType::Year => "year",
        }
    }

    pub fn get_correct_answer(&self) -> Vec<Arc<str>> {
        self.get_correct_options()
            .iter()
            .map(|opt| opt.option.clone())
            .collect()
    }

    pub fn generate_round_alternatives(
        &self,
        color_weights: &[f64; Color::COUNT],
    ) -> Vec<Arc<str>> {
        match self.question_type {
            QuestionType::Color => self
                .generate_color_alternatives(color_weights)
                .into_iter()
                .map(Arc::from)
                .collect(),
            QuestionType::Character | QuestionType::Text => {
                let mut alternatives: Vec<Arc<str>> =
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
                        .into_iter()
                        .map(Arc::from)
                        .collect()
                } else {
                    vec![]
                }
            }
        }
    }

    fn generate_color_alternatives(
        &self,
        color_weights: &[f64; Color::COUNT],
    ) -> Vec<&'static str> {
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
            .map(|color| (color, color_weights[color.idx()]))
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
        round_colors
            .into_iter()
            .map(|c| match c {
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
            })
            .collect()
    }

    fn generate_year_alternatives(&self, correct_year: i32) -> Vec<&'static str> {
        use std::collections::HashMap;
        use std::sync::OnceLock;

        static YEAR_CACHE: OnceLock<HashMap<i32, &'static str>> = OnceLock::new();

        let cache = YEAR_CACHE.get_or_init(|| {
            let mut map = HashMap::new();
            // Pre-populate common years (1950-2030)
            for year in 1950..=2030 {
                let year_str: &'static str = Box::leak(year.to_string().into_boxed_str());
                map.insert(year, year_str);
            }
            map
        });

        let mut alternatives = [
            correct_year - 2,
            correct_year - 1,
            correct_year,
            correct_year + 1,
            correct_year + 2,
        ];
        fastrand::shuffle(&mut alternatives);

        alternatives
            .iter()
            .map(|&y| {
                cache.get(&y).copied().unwrap_or_else(|| {
                    // Fallback for years outside our cache range
                    let year_str: &'static str = Box::leak(y.to_string().into_boxed_str());
                    year_str
                })
            })
            .collect()
    }
}

/// Immutable snapshot of question data and derived weights. Always obtain via
/// `QuestionStore::snapshot()` to keep questions/sets/weights in sync.
pub struct QuestionSnapshot {
    pub questions: Arc<Vec<GameQuestion>>,
    pub sets: Arc<Vec<QuestionSet>>,
    pub color_weights: [f64; Color::COUNT],
}

fn calculate_color_weights_global(questions: &[GameQuestion]) -> [f64; Color::COUNT] {
    let mut color_counts = [0usize; Color::COUNT];
    let mut total_correct_color_answers = 0usize;

    for question in questions {
        if question.question_type == QuestionType::Color {
            for option in question.get_correct_options() {
                if let Ok(color) = option.option.parse::<Color>() {
                    color_counts[color.idx()] += 1;
                    total_correct_color_answers += 1;
                }
            }
        }
    }

    if total_correct_color_answers == 0 {
        return baseline_weights();
    }

    let mut weights = [0.0; Color::COUNT];
    for color in Color::all() {
        let count = color_counts[color.idx()] as f64;
        let base_proportion = count / total_correct_color_answers as f64;
        weights[color.idx()] = base_proportion.sqrt() + 0.15;
    }
    weights
}

fn build_snapshot(
    game_questions: Vec<GameQuestion>,
    sets: Vec<QuestionSet>,
) -> Result<QuestionSnapshot, QuestionError> {
    if game_questions.is_empty() {
        return Err(QuestionError::NoQuestions);
    }

    let color_weights = calculate_color_weights_global(&game_questions);
    Ok(QuestionSnapshot {
        questions: Arc::new(game_questions),
        sets: Arc::new(sets),
        color_weights,
    })
}

pub struct QuestionStore {
    snapshot: ArcSwap<QuestionSnapshot>,
    db: QuestionDatabase,
}

impl QuestionStore {
    pub async fn new(config: &StorageConfig) -> Result<Self, QuestionError> {
        let db = QuestionDatabase::new(config).map_err(QuestionError::DbError)?;

        let (game_questions, sets) = db.load_questions().await.map_err(QuestionError::DbError)?;
        let snapshot = build_snapshot(game_questions, sets)?;

        Ok(Self {
            snapshot: ArcSwap::from_pointee(snapshot),
            db,
        })
    }

    pub async fn reload(&self) -> Result<(), QuestionError> {
        let (game_questions, sets) = self
            .db
            .load_questions()
            .await
            .map_err(QuestionError::DbError)?;

        let snapshot = build_snapshot(game_questions, sets)?;
        self.snapshot.store(Arc::new(snapshot));
        Ok(())
    }

    pub fn snapshot(&self) -> Arc<QuestionSnapshot> {
        self.snapshot.load_full()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_weights_use_correct_answer_denominator() {
        let color_question = GameQuestion {
            id: 1,
            question_type: QuestionType::Color,
            question_text: None,
            title: Arc::from("Color"),
            artist: None,
            youtube_id: Arc::from("id"),
            options: vec![
                GameQuestionOption {
                    option: Arc::from("Red"),
                    is_correct: true,
                },
                GameQuestionOption {
                    option: Arc::from("Blue"),
                    is_correct: true,
                },
            ],
        };

        let mut questions = vec![color_question];
        for idx in 0..9 {
            questions.push(GameQuestion {
                id: idx + 2,
                question_type: QuestionType::Text,
                question_text: None,
                title: Arc::from("Other"),
                artist: None,
                youtube_id: Arc::from("id"),
                options: vec![GameQuestionOption {
                    option: Arc::from(format!("Opt{idx}")),
                    is_correct: true,
                }],
            });
        }

        let weights = calculate_color_weights_global(&questions);
        let expected = (0.5_f64).sqrt() + 0.15;
        assert!(
            (weights[Color::Red.idx()] - expected).abs() < 1e-12,
            "expected red weight close to {expected}, got {}",
            weights[Color::Red.idx()]
        );
        assert!(
            (weights[Color::Blue.idx()] - expected).abs() < 1e-12,
            "expected blue weight close to {expected}, got {}",
            weights[Color::Blue.idx()]
        );
        for color in Color::all()
            .iter()
            .copied()
            .filter(|c| *c != Color::Red && *c != Color::Blue)
        {
            assert!(
                (weights[color.idx()] - 0.15).abs() < 1e-12,
                "expected baseline weight 0.15 for {color:?}, got {}",
                weights[color.idx()]
            );
        }
    }

    #[test]
    fn color_weights_zero_correct_answers_use_baseline() {
        let questions = vec![GameQuestion {
            id: 1,
            question_type: QuestionType::Text,
            question_text: None,
            title: Arc::from("Other"),
            artist: None,
            youtube_id: Arc::from("id"),
            options: vec![GameQuestionOption {
                option: Arc::from("Only"),
                is_correct: true,
            }],
        }];

        let weights = calculate_color_weights_global(&questions);
        for (idx, &weight) in weights.iter().enumerate() {
            assert!(
                (weight - 0.15).abs() < 1e-12,
                "expected baseline weight for color index {idx}, got {weight}"
            );
        }
    }
}
