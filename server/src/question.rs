use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMedia {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub release_year: Option<i32>,
    pub spotify_uri: Option<String>,
    pub youtube_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuestionType {
    Color,
    Character,
    Text,
    Year,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageQuestion {
    pub id: i64,
    pub media_id: i64,
    pub question_type: QuestionType,
    pub question_text: Option<String>,
    pub image_url: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageQuestionOption {
    pub id: i64,
    pub question_id: i64,
    pub option_text: String,
    pub is_correct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredData {
    pub media: Vec<StorageMedia>,
    pub questions: Vec<StorageQuestion>,
    pub options: Vec<StorageQuestionOption>,
    pub sets: Vec<QuestionSet>,
}

// Runtime models used by the game engine
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

// Question sets are small enough to keep the same format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionSet {
    pub id: i64,
    pub name: String,
    pub question_ids: Vec<i64>,
}

impl GameQuestion {
    fn from_storage(
        question: &StorageQuestion,
        media: &StorageMedia,
        options: &[StorageQuestionOption],
    ) -> Self {
        GameQuestion {
            id: question.id as u16,
            question_type: question.question_type,
            title: media.title.clone(),
            artist: Some(media.artist.clone()),
            youtube_id: media.youtube_id.clone(),
            options: options
                .iter()
                .map(|opt| GameQuestionOption {
                    option: opt.option_text.clone(),
                    is_correct: opt.is_correct,
                })
                .collect(),
        }
    }
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

impl GameQuestion {
    pub fn get_id(&self) -> u16 {
        self.id
    }

    pub fn get_youtube_id(&self) -> &str {
        &self.youtube_id
    }

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
                // For our slimmer format, we'll derive year from the correct answer
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
                // For year questions, parse both the answer and the correct year
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

// Add implementation for Color parsing
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

// QuestionManager maintains the runtime state of questions.
// - Questions are stored in JSON format on disk
// - Only lightweight GameQuestions are kept in memory
// - When questions are updated, existing games continue with their version
// - New games will get the latest version of questions
pub struct QuestionManager {
    questions: RwLock<Arc<Vec<GameQuestion>>>,
    sets: RwLock<Arc<Vec<QuestionSet>>>,
    file_path: String,
}

impl QuestionManager {
    pub async fn new(file_path: &str) -> Result<Self, QuestionError> {
        let manager = Self {
            questions: RwLock::new(Arc::new(Vec::new())),
            sets: RwLock::new(Arc::new(Vec::new())),
            file_path: file_path.to_string(),
        };

        manager.load_questions().await?;
        Ok(manager)
    }

    pub async fn load_questions(&self) -> Result<(), QuestionError> {
        let stored_data: StoredData = if Path::new(&self.file_path).exists() {
            let content = fs::read_to_string(&self.file_path)?;
            serde_json::from_str(&content)?
        } else {
            StoredData {
                media: Vec::new(),
                questions: Vec::new(),
                options: Vec::new(),
                sets: Vec::new(),
            }
        };

        let game_questions: Vec<GameQuestion> = stored_data
            .questions
            .iter()
            .filter(|q| q.is_active)
            .filter_map(|question| {
                let media = stored_data
                    .media
                    .iter()
                    .find(|m| m.id == question.media_id)?;
                let options: Vec<StorageQuestionOption> = stored_data
                    .options
                    .iter()
                    .filter(|o| o.question_id == question.id)
                    .cloned()
                    .collect();

                Some(GameQuestion::from_storage(question, media, &options))
            })
            .collect();

        if game_questions.is_empty() {
            return Err(QuestionError::NoQuestions);
        }

        *self.questions.write().await = Arc::new(game_questions);
        *self.sets.write().await = Arc::new(stored_data.sets);

        Ok(())
    }

    async fn read_stored_data(&self) -> Result<StoredData, QuestionError> {
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

    // Game engine methods - work with GameQuestion
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

    // Admin methods - accept full storage format to preserve all metadata
    pub async fn add_question(
        &self,
        question: StorageQuestion,
        media: StorageMedia,
        options: Vec<StorageQuestionOption>,
    ) -> Result<(), QuestionError> {
        // Read current storage data
        let mut stored_data = self.read_stored_data().await?;

        // Update storage with full data
        stored_data.media.push(media.clone());
        stored_data.questions.push(question.clone());
        stored_data.options.extend(options.clone());

        // Save to file
        let json = serde_json::to_string_pretty(&stored_data)?;
        fs::write(&self.file_path, json)?;

        // Only update runtime questions if the question is active
        if question.is_active {
            // Update runtime questions with just the needed data
            let mut questions = self.questions.write().await;
            let mut new_questions = (*questions).to_vec();

            new_questions.push(GameQuestion::from_storage(&question, &media, &options));

            *questions = Arc::new(new_questions);
        }
        Ok(())
    }

    pub async fn update_question(
        &self,
        question: StorageQuestion,
        media: StorageMedia,
        options: Vec<StorageQuestionOption>,
    ) -> Result<(), QuestionError> {
        let mut stored_data = self.read_stored_data().await?;

        // Update storage data
        if let Some(idx) = stored_data
            .questions
            .iter()
            .position(|q| q.id == question.id)
        {
            stored_data.questions[idx] = question.clone();
        } else {
            return Err(QuestionError::NoQuestions);
        }

        if let Some(idx) = stored_data.media.iter().position(|m| m.id == media.id) {
            stored_data.media[idx] = media.clone();
        } else {
            stored_data.media.push(media.clone());
        }

        // Remove old options and add new ones
        stored_data.options.retain(|o| o.question_id != question.id);
        stored_data.options.extend(options.clone());

        // Save to file
        let json = serde_json::to_string_pretty(&stored_data)?;
        fs::write(&self.file_path, json)?;

        // Update runtime questions if active
        if question.is_active {
            let mut questions = self.questions.write().await;
            let mut new_questions = (*questions).to_vec();

            if let Some(idx) = new_questions
                .iter()
                .position(|q| i64::from(q.id) == question.id)
            {
                new_questions[idx] = GameQuestion::from_storage(&question, &media, &options);
            }

            *questions = Arc::new(new_questions);
        } else {
            // If question is now inactive, remove it from runtime
            let mut questions = self.questions.write().await;
            let mut new_questions = (*questions).to_vec();
            new_questions.retain(|q| i64::from(q.id) != question.id);
            *questions = Arc::new(new_questions);
        }

        Ok(())
    }

    pub async fn remove_question(&self, question_id: i64) -> Result<(), QuestionError> {
        let mut stored_data = self.read_stored_data().await?;

        // Find and update the question in storage
        if let Some(question) = stored_data
            .questions
            .iter_mut()
            .find(|q| q.id == question_id)
        {
            question.is_active = false;

            // Save to file
            let json = serde_json::to_string_pretty(&stored_data)?;
            fs::write(&self.file_path, json)?;

            // Remove from runtime questions
            let mut questions = self.questions.write().await;
            let mut new_questions = (*questions).to_vec();
            new_questions.retain(|q| i64::from(q.id) != question_id);
            *questions = Arc::new(new_questions);

            Ok(())
        } else {
            Err(QuestionError::NoQuestions)
        }
    }
}
