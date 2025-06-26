use crate::question::{Color, GameQuestion, GameQuestionOption, QuestionType, COLOR_WEIGHTS};
use crate::StorageConfig;
use aws_sdk_s3::config::{
    Credentials, Region, RequestChecksumCalculation, ResponseChecksumValidation,
};
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use chrono::Utc;
use flate2::Compression;
use flate2::{read::GzDecoder, write::GzEncoder};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum DbError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("No questions found")]
    NoQuestions,
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("S3 error: {0}")]
    S3(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Media {
    id: i64,
    title: Arc<str>,
    artist: Arc<str>,
    release_year: Option<i32>,
    spotify_uri: Option<Arc<str>>,
    youtube_id: Arc<str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Character {
    id: i64,
    name: Arc<str>,
    image_url: Arc<str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Question {
    id: i64,
    media_id: i64,
    question_type: QuestionType,
    question_text: Option<Arc<str>>,
    image_url: Option<Arc<str>>,
    is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QuestionOption {
    id: i64,
    question_id: i64,
    option_text: Arc<str>,
    is_correct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionSet {
    pub id: i64,
    pub name: Arc<str>,
    pub question_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoredData {
    media: Vec<Media>,
    characters: Vec<Character>,
    questions: Vec<Question>,
    options: Vec<QuestionOption>,
    sets: Vec<QuestionSet>,
}

impl StoredData {
    /// Validates the integrity of the stored data by checking:
    /// - Duplicate IDs of any type (media, characters, questions, options, sets)
    /// - Duplicate character names or image URLs
    /// - Questions referencing non-existent media IDs
    /// - Options referencing non-existent questions
    /// - Options referencing non-existent character names (via `option_text`)
    /// - Text questions/options having empty strings
    /// - Sets referencing non-existent questions
    ///
    /// Returns `Ok(())` if all validations pass, or a `DbError::Validation` with detailed error message.
    pub fn validate_stored_data(&self) -> Result<(), DbError> {
        let mut seen_media_ids = HashSet::new();
        for media in &self.media {
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
        for character in &self.characters {
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
        for question in &self.questions {
            if !seen_question_ids.insert(question.id) {
                return Err(DbError::Validation(format!(
                    "Duplicate question ID: {}",
                    question.id
                )));
            }
        }

        let mut seen_option_ids = HashSet::new();
        for option in &self.options {
            if !seen_option_ids.insert(option.id) {
                return Err(DbError::Validation(format!(
                    "Duplicate option ID: {}",
                    option.id
                )));
            }
        }

        let mut seen_set_ids = HashSet::new();
        for set in &self.sets {
            if !seen_set_ids.insert(set.id) {
                return Err(DbError::Validation(format!("Duplicate set ID: {}", set.id)));
            }
        }

        let character_names = seen_names;
        let media_ids = seen_media_ids;
        let question_ids = seen_question_ids;

        // Create a HashSet of valid color names
        let valid_colors: HashSet<String> = Color::all().iter().map(ToString::to_string).collect();

        for question in &self.questions {
            if !media_ids.contains(&question.media_id) {
                return Err(DbError::Validation(format!(
                    "Question {} references non-existent media ID {}",
                    question.id, question.media_id
                )));
            }

            match question.question_type {
                QuestionType::Text => {
                    if question.question_text.is_none() {
                        return Err(DbError::Validation(format!(
                            "Question {} of type Text has no question text",
                            question.id
                        )));
                    }
                }
                QuestionType::Year | QuestionType::Color | QuestionType::Character => {}
            }
        }

        for option in &self.options {
            if !question_ids.contains(&option.question_id) {
                return Err(DbError::Validation(format!(
                    "Option {} references non-existent question ID {}",
                    option.id, option.question_id
                )));
            }

            // Get the question type for this option
            let question_type = self
                .questions
                .iter()
                .find(|q| q.id == option.question_id)
                .map(|q| &q.question_type)
                .ok_or_else(|| {
                    DbError::Validation(format!("Question not found for option {}", option.id))
                })?;

            match question_type {
                QuestionType::Color => {
                    if !valid_colors.contains(option.option_text.as_ref()) {
                        return Err(DbError::Validation(format!(
                            "Option {} references invalid color name '{}'",
                            option.id, option.option_text
                        )));
                    }
                }
                QuestionType::Character => {
                    if !character_names.contains(&option.option_text) {
                        return Err(DbError::Validation(format!(
                            "Option {} references non-existent character name '{}'",
                            option.id, option.option_text
                        )));
                    }
                }
                QuestionType::Text => {
                    if option.option_text.is_empty() {
                        return Err(DbError::Validation(format!(
                            "Option {} of type Text has no option text",
                            option.id
                        )));
                    }
                }
                QuestionType::Year => {}
            }
        }

        for set in &self.sets {
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
}

pub enum Storage {
    Filesystem(FilesystemBackend),
    S3(S3Backend),
}

impl Storage {
    async fn read_file(&self, path: &str) -> Result<String, DbError> {
        match self {
            Self::Filesystem(fs) => fs.read_file(path).await,
            Self::S3(s3) => s3.read_file(path).await,
        }
    }

    async fn write_file(&self, path: &str, data: &[u8]) -> Result<(), DbError> {
        match self {
            Self::Filesystem(fs) => fs.write_file(path, data).await,
            Self::S3(s3) => s3.write_file(path, data).await,
        }
    }

    async fn create_backup(&self, content: &str, file_stem: &str) -> Result<(), DbError> {
        match self {
            Self::Filesystem(fs) => fs.create_backup(content, file_stem).await,
            Self::S3(s3) => s3.create_backup(content, file_stem).await,
        }
    }

    pub async fn store_character_image(
        &self,
        character_name: &str,
        data: &[u8],
    ) -> Result<String, DbError> {
        let filename = format!("{character_name}.avif");
        let path = format!("img/{filename}");
        self.write_file(&path, data).await?;
        Ok(format!("/img/{filename}"))
    }
}

// Filesystem implementation
pub struct FilesystemBackend {
    base_path: PathBuf,
    backup_dir: PathBuf,
}

impl FilesystemBackend {
    async fn read_file(&self, path: &str) -> Result<String, DbError> {
        let full_path = self.base_path.join(path);
        if full_path.exists() {
            tokio::fs::read_to_string(&full_path)
                .await
                .map_err(DbError::from)
        } else {
            Ok(String::new())
        }
    }

    async fn write_file(&self, path: &str, data: &[u8]) -> Result<(), DbError> {
        let full_path = self.base_path.join(path);

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(DbError::from)?;
        }

        tokio::fs::write(full_path, data)
            .await
            .map_err(DbError::from)
    }

    async fn create_backup(&self, content: &str, file_stem: &str) -> Result<(), DbError> {
        let backup_dir = self.backup_dir.clone();
        let file_stem = file_stem.to_string();
        let content = content.to_string();
        tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&backup_dir)?;
            let now = Utc::now();
            let timestamp = now.format("%y%m%d_%H%M%S").to_string();
            let filename = format!("{file_stem}_{timestamp}.json.gz");
            let full_path = backup_dir.join(filename);
            let file = std::fs::File::create(&full_path)?;
            let mut encoder = GzEncoder::new(file, Compression::default());
            encoder.write_all(content.as_bytes())?;
            encoder.finish()?;
            Ok(())
        })
        .await
        .map_err(|e| DbError::Io(std::io::Error::other(e)))?
    }
}

// S3 implementation
pub struct S3Backend {
    client: Client,
    bucket: String,
    prefix: String,
    question_folder: String,
}

impl S3Backend {
    async fn read_file(&self, path: &str) -> Result<String, DbError> {
        let mut key = format!("{}/{}", self.prefix, path);

        let is_json = std::path::Path::new(path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));

        // Read json question data from hidden folder
        if is_json {
            key = format!("{}/{}/{}.gz", self.prefix, self.question_folder, path);
        }

        info!("Reading from S3: {}", key);

        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(response) => {
                let bytes = response
                    .body
                    .collect()
                    .await
                    .map_err(|e| DbError::S3(format!("Failed to collect bytes: {e}")))?;

                // If it's a JSON file, decompress it
                if is_json {
                    let bytes = bytes.into_bytes();
                    let mut decoder = GzDecoder::new(&bytes[..]);
                    let mut decompressed = String::new();
                    decoder
                        .read_to_string(&mut decompressed)
                        .map_err(|e| DbError::S3(format!("Failed to decompress: {e}")))?;
                    Ok(decompressed)
                } else {
                    String::from_utf8(bytes.to_vec())
                        .map_err(|e| DbError::S3(format!("Invalid UTF-8: {e}")))
                }
            }
            Err(err) => {
                if let SdkError::ServiceError(service_err) = &err {
                    warn!("S3 error details: {:?}", err);
                    if service_err.err().is_no_such_key() {
                        return Ok(String::new());
                    }
                }
                Err(DbError::S3(err.to_string()))
            }
        }
    }

    async fn write_file(&self, path: &str, data: &[u8]) -> Result<(), DbError> {
        let mut key = format!("{}/{}", self.prefix, path);

        // Determine if we're writing a JSON file
        let is_json = std::path::Path::new(path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));

        // Store question data in hidden folder and append .gz for JSON files
        if is_json {
            key = format!("{}/{}/{}.gz", self.prefix, self.question_folder, path);
        }

        // Determine content type based on file extension
        let content_type = if std::path::Path::new(path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("avif"))
        {
            "image/avif"
        } else if std::path::Path::new(path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("webm"))
        {
            "video/webm"
        } else if is_json {
            "application/gzip" // Store JSON data as compressed
        } else {
            "application/octet-stream" // Default binary type
        };

        info!("Writing {} to S3: {}", content_type, key);

        let body = if is_json {
            // Compress JSON data
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder
                .write_all(data)
                .map_err(|e| DbError::S3(format!("Failed to compress: {e}")))?;
            encoder
                .finish()
                .map_err(|e| DbError::S3(format!("Failed to finish compression: {e}")))?
        } else {
            data.to_vec()
        };

        match self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(body))
            .content_type(content_type)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                warn!("S3 error details: {:?}", err);
                Err(DbError::S3(err.to_string()))
            }
        }
    }

    async fn create_backup(&self, content: &str, file_stem: &str) -> Result<(), DbError> {
        let now = Utc::now();
        let timestamp = now.format("%y%m%d_%H%M%S").to_string();
        let key = format!(
            "{}/{}/backup/{file_stem}_{timestamp}.json.gz",
            self.prefix, self.question_folder
        );
        info!("Create backup S3: {}", key);

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(content.as_bytes())?;
        let compressed = encoder.finish()?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(compressed))
            .content_type("application/gzip")
            .send()
            .await
            .map_err(|e| DbError::S3(e.to_string()))?;

        Ok(())
    }
}

pub struct QuestionDatabase {
    question_file: String,
    storage: Storage,
}

impl QuestionDatabase {
    pub fn new(config: &StorageConfig) -> Result<Self, DbError> {
        let (storage, file_path) = match config {
            StorageConfig::Filesystem {
                base_path,
                file_path,
            } => {
                let backup_dir = base_path.join("question_backup");
                (
                    Storage::Filesystem(FilesystemBackend {
                        base_path: base_path.clone(),
                        backup_dir,
                    }),
                    file_path.clone(),
                )
            }
            StorageConfig::S3 {
                bucket,
                region,
                prefix,
                question_folder,
                question_file: file_path,
                access_key_id,
                secret_access_key,
            } => {
                let config = aws_sdk_s3::Config::builder()
                    .region(Region::new(region.clone()))
                    .endpoint_url(format!("https://s3.{region}.backblazeb2.com"))
                    .force_path_style(true)
                    .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
                    .use_fips(false)
                    .use_dual_stack(false)
                    .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
                    .response_checksum_validation(ResponseChecksumValidation::WhenRequired)
                    .credentials_provider(Credentials::new(
                        access_key_id,
                        secret_access_key,
                        None,
                        None,
                        "backblaze-credentials",
                    ))
                    .build();
                let client = Client::from_conf(config);

                (
                    Storage::S3(S3Backend {
                        client,
                        bucket: bucket.clone(),
                        prefix: prefix.clone(),
                        question_folder: question_folder.clone(),
                    }),
                    file_path.clone(),
                )
            }
        };

        Ok(Self {
            question_file: file_path,
            storage,
        })
    }

    pub async fn read_stored_data(&self) -> Result<StoredData, DbError> {
        let content = self.storage.read_file(&self.question_file).await?;
        if content.is_empty() {
            return Ok(StoredData {
                media: Vec::new(),
                characters: Vec::new(),
                questions: Vec::new(),
                options: Vec::new(),
                sets: Vec::new(),
            });
        }

        let json_content: StoredData = serde_json::from_str(&content)?;
        json_content.validate_stored_data()?;
        Ok(json_content)
    }

    pub async fn set_stored_data(&self, data: StoredData) -> Result<(), DbError> {
        data.validate_stored_data()?;
        let json = serde_json::to_string(&data)?;
        self.storage
            .write_file(&self.question_file, json.as_bytes())
            .await
    }

    pub async fn store_character_image(
        &self,
        character_name: &str,
        data: &[u8],
    ) -> Result<String, DbError> {
        self.storage
            .store_character_image(character_name, data)
            .await
    }

    pub async fn backup_stored_data(&self) -> Result<(), DbError> {
        let stored_data = self.read_stored_data().await?;
        let json = serde_json::to_string(&stored_data)?;

        let file_stem = Path::new(&self.question_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                DbError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Could not extract filename stem",
                ))
            })?;

        self.storage.create_backup(&json, file_stem).await
    }

    pub async fn load_questions(&self) -> Result<(Vec<GameQuestion>, Vec<QuestionSet>), DbError> {
        let stored_data = self.read_stored_data().await?;
        stored_data.validate_stored_data()?;

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
                        option: Arc::from(&*opt.option_text),
                        is_correct: opt.is_correct,
                    })
                    .collect();

                Some(GameQuestion {
                    id: question.id.try_into().ok()?,
                    question_type: question.question_type,
                    question_text: question.question_text.as_ref().map(|s| Arc::from(&**s)),
                    title: Arc::from(&*media.title),
                    artist: Some(Arc::from(&*media.artist)),
                    youtube_id: Arc::from(&*media.youtube_id),
                    options,
                })
            })
            .collect();

        if game_questions.is_empty() {
            return Err(DbError::NoQuestions);
        }

        let weights = Self::calculate_color_weights(&game_questions);
        COLOR_WEIGHTS.clear();
        for (key, value) in weights {
            COLOR_WEIGHTS.insert(key, value);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_valid_data() {
        let data = StoredData {
            media: vec![Media {
                id: 1,
                title: Arc::from("Title"),
                artist: Arc::from("Artist"),
                release_year: None,
                spotify_uri: None,
                youtube_id: Arc::from("youtube_id"),
            }],
            characters: vec![Character {
                id: 1,
                name: Arc::from("CharacterName"),
                image_url: Arc::from("image_url"),
            }],
            questions: vec![Question {
                id: 1,
                media_id: 1,
                question_type: QuestionType::Character,
                question_text: None,
                image_url: None,
                is_active: true,
            }],
            options: vec![QuestionOption {
                id: 1,
                question_id: 1,
                option_text: Arc::from("CharacterName"),
                is_correct: true,
            }],
            sets: vec![QuestionSet {
                id: 1,
                name: Arc::from("Set Name"),
                question_ids: vec![1],
            }],
        };
        assert!(data.validate_stored_data().is_ok());
    }

    #[test]
    fn validate_duplicate_media_id() {
        let data = StoredData {
            media: vec![
                Media {
                    id: 1,
                    title: Arc::from("Title1"),
                    artist: Arc::from("Artist1"),
                    release_year: None,
                    spotify_uri: None,
                    youtube_id: Arc::from("youtube_id1"),
                },
                Media {
                    id: 1,
                    title: Arc::from("Title2"),
                    artist: Arc::from("Artist2"),
                    release_year: None,
                    spotify_uri: None,
                    youtube_id: Arc::from("youtube_id2"),
                },
            ],
            characters: vec![],
            questions: vec![],
            options: vec![],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_err());
        if let Err(DbError::Validation(msg)) = data.validate_stored_data() {
            assert_eq!(msg, "Duplicate media ID: 1");
        }
    }

    #[test]
    fn validate_duplicate_character_id() {
        let data = StoredData {
            media: vec![],
            characters: vec![
                Character {
                    id: 1,
                    name: Arc::from("Name1"),
                    image_url: Arc::from("url1"),
                },
                Character {
                    id: 1,
                    name: Arc::from("Name2"),
                    image_url: Arc::from("url2"),
                },
            ],
            questions: vec![],
            options: vec![],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_err());
        if let Err(DbError::Validation(msg)) = data.validate_stored_data() {
            assert_eq!(msg, "Duplicate character ID: 1");
        }
    }

    #[test]
    fn validate_duplicate_question_id() {
        let data = StoredData {
            media: vec![Media {
                id: 1,
                title: Arc::from("Title"),
                artist: Arc::from("Artist"),
                release_year: None,
                spotify_uri: None,
                youtube_id: Arc::from("youtube_id"),
            }],
            characters: vec![],
            questions: vec![
                Question {
                    id: 1,
                    media_id: 1,
                    question_type: QuestionType::Character,
                    question_text: None,
                    image_url: None,
                    is_active: true,
                },
                Question {
                    id: 1,
                    media_id: 1,
                    question_type: QuestionType::Character,
                    question_text: None,
                    image_url: None,
                    is_active: true,
                },
            ],
            options: vec![],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_err());
        if let Err(DbError::Validation(msg)) = data.validate_stored_data() {
            assert_eq!(msg, "Duplicate question ID: 1");
        }
    }

    #[test]
    fn validate_duplicate_option_id() {
        let data = StoredData {
            media: vec![Media {
                id: 1,
                title: Arc::from("Title"),
                artist: Arc::from("Artist"),
                release_year: None,
                spotify_uri: None,
                youtube_id: Arc::from("youtube_id"),
            }],
            characters: vec![Character {
                id: 1,
                name: Arc::from("CharacterName"),
                image_url: Arc::from("image_url"),
            }],
            questions: vec![Question {
                id: 1,
                media_id: 1,
                question_type: QuestionType::Character,
                question_text: None,
                image_url: None,
                is_active: true,
            }],
            options: vec![
                QuestionOption {
                    id: 1,
                    question_id: 1,
                    option_text: Arc::from("Option1"),
                    is_correct: true,
                },
                QuestionOption {
                    id: 1,
                    question_id: 1,
                    option_text: Arc::from("Option2"),
                    is_correct: false,
                },
            ],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_err());
        if let Err(DbError::Validation(msg)) = data.validate_stored_data() {
            assert_eq!(msg, "Duplicate option ID: 1");
        }
    }

    #[test]
    fn validate_duplicate_set_id() {
        let data = StoredData {
            media: vec![],
            characters: vec![],
            questions: vec![],
            options: vec![],
            sets: vec![
                QuestionSet {
                    id: 1,
                    name: Arc::from("Set1"),
                    question_ids: vec![],
                },
                QuestionSet {
                    id: 1,
                    name: Arc::from("Set2"),
                    question_ids: vec![],
                },
            ],
        };
        assert!(data.validate_stored_data().is_err());
        if let Err(DbError::Validation(msg)) = data.validate_stored_data() {
            assert_eq!(msg, "Duplicate set ID: 1");
        }
    }

    #[test]
    fn validate_duplicate_character_name() {
        let data = StoredData {
            media: vec![],
            characters: vec![
                Character {
                    id: 1,
                    name: Arc::from("CharacterName"),
                    image_url: Arc::from("url1"),
                },
                Character {
                    id: 2,
                    name: Arc::from("CharacterName"),
                    image_url: Arc::from("url2"),
                },
            ],
            questions: vec![],
            options: vec![],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_err());
        if let Err(DbError::Validation(msg)) = data.validate_stored_data() {
            assert_eq!(msg, "Duplicate character name: CharacterName");
        }
    }

    #[test]
    fn validate_duplicate_character_image_url() {
        let data = StoredData {
            media: vec![],
            characters: vec![
                Character {
                    id: 1,
                    name: Arc::from("Name1"),
                    image_url: Arc::from("image_url"),
                },
                Character {
                    id: 2,
                    name: Arc::from("Name2"),
                    image_url: Arc::from("image_url"),
                },
            ],
            questions: vec![],
            options: vec![],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_err());
        if let Err(DbError::Validation(msg)) = data.validate_stored_data() {
            assert_eq!(msg, "Duplicate character image URL: image_url");
        }
    }

    #[test]
    fn validate_question_non_existent_media_id() {
        let data = StoredData {
            media: vec![],
            characters: vec![],
            questions: vec![Question {
                id: 1,
                media_id: 1,
                question_type: QuestionType::Character,
                question_text: None,
                image_url: None,
                is_active: true,
            }],
            options: vec![],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_err());
        if let Err(DbError::Validation(msg)) = data.validate_stored_data() {
            assert_eq!(msg, "Question 1 references non-existent media ID 1");
        }
    }

    #[test]
    fn validate_option_non_existent_question_id() {
        let data = StoredData {
            media: vec![Media {
                id: 1,
                title: Arc::from("Title"),
                artist: Arc::from("Artist"),
                release_year: None,
                spotify_uri: None,
                youtube_id: Arc::from("youtube_id"),
            }],
            characters: vec![Character {
                id: 1,
                name: Arc::from("CharacterName"),
                image_url: Arc::from("image_url"),
            }],
            questions: vec![],
            options: vec![QuestionOption {
                id: 1,
                question_id: 1,
                option_text: Arc::from("CharacterName"),
                is_correct: true,
            }],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_err());
        if let Err(DbError::Validation(msg)) = data.validate_stored_data() {
            assert_eq!(msg, "Option 1 references non-existent question ID 1");
        }
    }

    #[test]
    fn validate_set_non_existent_question_id() {
        let data = StoredData {
            media: vec![],
            characters: vec![],
            questions: vec![],
            options: vec![],
            sets: vec![QuestionSet {
                id: 1,
                name: Arc::from("Set Name"),
                question_ids: vec![1],
            }],
        };
        assert!(data.validate_stored_data().is_err());
        if let Err(DbError::Validation(msg)) = data.validate_stored_data() {
            assert_eq!(msg, "Set 1 references non-existent question ID 1");
        }
    }

    #[test]
    fn validate_option_non_existent_character_name() {
        let data = StoredData {
            media: vec![Media {
                id: 1,
                title: Arc::from("Title"),
                artist: Arc::from("Artist"),
                release_year: None,
                spotify_uri: None,
                youtube_id: Arc::from("youtube_id"),
            }],
            characters: vec![Character {
                id: 1,
                name: Arc::from("CharacterName"),
                image_url: Arc::from("image_url"),
            }],
            questions: vec![Question {
                id: 1,
                media_id: 1,
                question_type: QuestionType::Character,
                question_text: None,
                image_url: None,
                is_active: true,
            }],
            options: vec![QuestionOption {
                id: 1,
                question_id: 1,
                option_text: Arc::from("NonExistentCharacter"),
                is_correct: true,
            }],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_err());
        if let Err(DbError::Validation(msg)) = data.validate_stored_data() {
            assert_eq!(
                msg,
                "Option 1 references non-existent character name 'NonExistentCharacter'"
            );
        }
    }

    #[test]
    fn validate_option_invalid_color_name() {
        let data = StoredData {
            media: vec![Media {
                id: 1,
                title: Arc::from("Title"),
                artist: Arc::from("Artist"),
                release_year: None,
                spotify_uri: None,
                youtube_id: Arc::from("youtube_id"),
            }],
            characters: vec![],
            questions: vec![Question {
                id: 1,
                media_id: 1,
                question_type: QuestionType::Color,
                question_text: None,
                image_url: None,
                is_active: true,
            }],
            options: vec![QuestionOption {
                id: 1,
                question_id: 1,
                option_text: Arc::from("invalid_color"),
                is_correct: true,
            }],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_err());
        if let Err(DbError::Validation(msg)) = data.validate_stored_data() {
            assert!(msg.starts_with("Option 1 references invalid color name"));
        }
    }

    #[test]
    fn validate_option_valid_color_name() {
        let data = StoredData {
            media: vec![Media {
                id: 1,
                title: Arc::from("Title"),
                artist: Arc::from("Artist"),
                release_year: None,
                spotify_uri: None,
                youtube_id: Arc::from("youtube_id"),
            }],
            characters: vec![],
            questions: vec![Question {
                id: 1,
                media_id: 1,
                question_type: QuestionType::Color,
                question_text: None,
                image_url: None,
                is_active: true,
            }],
            options: vec![QuestionOption {
                id: 1,
                question_id: 1,
                option_text: Arc::from("Red"),
                is_correct: true,
            }],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_ok());
    }

    #[test]
    fn validate_valid_color_question() {
        let data = StoredData {
            media: vec![Media {
                id: 1,
                title: Arc::from("Title"),
                artist: Arc::from("Artist"),
                release_year: None,
                spotify_uri: None,
                youtube_id: Arc::from("youtube_id"),
            }],
            characters: vec![],
            questions: vec![Question {
                id: 1,
                media_id: 1,
                question_type: QuestionType::Color,
                question_text: None,
                image_url: None,
                is_active: true,
            }],
            options: vec![GameQuestionOption {
                option: Arc::from("Red"),
                is_correct: true,
            }
            .into_stored(1)],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_ok());
    }

    #[test]
    fn validate_valid_character_question() {
        let data = StoredData {
            media: vec![Media {
                id: 1,
                title: Arc::from("Title"),
                artist: Arc::from("Artist"),
                release_year: None,
                spotify_uri: None,
                youtube_id: Arc::from("youtube_id"),
            }],
            characters: vec![Character {
                id: 1,
                name: Arc::from("CharacterName"),
                image_url: Arc::from("image_url"),
            }],
            questions: vec![Question {
                id: 1,
                media_id: 1,
                question_type: QuestionType::Character,
                question_text: None,
                image_url: None,
                is_active: true,
            }],
            options: vec![GameQuestionOption {
                option: Arc::from("CharacterName"),
                is_correct: true,
            }
            .into_stored(1)],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_ok());
    }

    #[test]
    fn validate_valid_multiple_options() {
        let data = StoredData {
            media: vec![Media {
                id: 1,
                title: Arc::from("Title"),
                artist: Arc::from("Artist"),
                release_year: None,
                spotify_uri: None,
                youtube_id: Arc::from("youtube_id"),
            }],
            characters: vec![
                Character {
                    id: 1,
                    name: Arc::from("CharacterName1"),
                    image_url: Arc::from("image_url1"),
                },
                Character {
                    id: 2,
                    name: Arc::from("CharacterName2"),
                    image_url: Arc::from("image_url2"),
                },
            ],
            questions: vec![Question {
                id: 1,
                media_id: 1,
                question_type: QuestionType::Character,
                question_text: None,
                image_url: None,
                is_active: true,
            }],
            options: vec![
                QuestionOption {
                    id: 1,
                    question_id: 1,
                    option_text: Arc::from("CharacterName1"),
                    is_correct: true,
                },
                QuestionOption {
                    id: 2,
                    question_id: 1,
                    option_text: Arc::from("CharacterName2"),
                    is_correct: false,
                },
            ],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_ok());
    }

    #[test]
    fn validate_valid_set_with_multiple_questions() {
        let data = StoredData {
            media: vec![Media {
                id: 1,
                title: Arc::from("Title"),
                artist: Arc::from("Artist"),
                release_year: None,
                spotify_uri: None,
                youtube_id: Arc::from("youtube_id"),
            }],
            characters: vec![Character {
                id: 1,
                name: Arc::from("CharacterName"),
                image_url: Arc::from("image_url"),
            }],
            questions: vec![
                Question {
                    id: 1,
                    media_id: 1,
                    question_type: QuestionType::Character,
                    question_text: None,
                    image_url: None,
                    is_active: true,
                },
                Question {
                    id: 2,
                    media_id: 1,
                    question_type: QuestionType::Character,
                    question_text: None,
                    image_url: None,
                    is_active: true,
                },
            ],
            options: vec![
                QuestionOption {
                    id: 1,
                    question_id: 1,
                    option_text: Arc::from("CharacterName"),
                    is_correct: true,
                },
                QuestionOption {
                    id: 2,
                    question_id: 2,
                    option_text: Arc::from("CharacterName"),
                    is_correct: true,
                },
            ],
            sets: vec![QuestionSet {
                id: 1,
                name: Arc::from("Set Name"),
                question_ids: vec![1, 2],
            }],
        };
        assert!(data.validate_stored_data().is_ok());
    }

    #[test]
    fn validate_empty_sets_question_ids_is_valid() {
        let data = StoredData {
            media: vec![],
            characters: vec![],
            questions: vec![],
            options: vec![],
            sets: vec![QuestionSet {
                id: 1,
                name: Arc::from("Set Name"),
                question_ids: vec![], // Empty question_ids
            }],
        };
        assert!(data.validate_stored_data().is_ok());
    }

    #[test]
    fn validate_long_character_name_and_image_url() {
        let long_string = "a".repeat(200); // Longer than typical names/urls, but still reasonable
        let data = StoredData {
            media: vec![],
            characters: vec![Character {
                id: 1,
                name: Arc::from(long_string.clone()),
                image_url: Arc::from(long_string),
            }],
            questions: vec![],
            options: vec![],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_ok());
    }

    #[test]
    fn validate_unicode_character_name() {
        let unicode_name = "キャラクタ名".to_string();
        let data = StoredData {
            media: vec![],
            characters: vec![Character {
                id: 1,
                name: Arc::from(unicode_name),
                image_url: Arc::from("url1"),
            }],
            questions: vec![],
            options: vec![],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_ok());
    }

    #[test]
    fn validate_special_chars_character_name_and_image_url() {
        let special_chars = r#"~!@#$%^&*()_+`-=[]\{}|;':",./<>?"#.to_string();
        let data = StoredData {
            media: vec![],
            characters: vec![Character {
                id: 1,
                name: Arc::from(special_chars.clone()),
                image_url: Arc::from(special_chars),
            }],
            questions: vec![],
            options: vec![],
            sets: vec![],
        };
        assert!(data.validate_stored_data().is_ok());
    }
    // Helper function to create a stored QuestionOption from GameQuestionOption for brevity
    impl GameQuestionOption {
        fn into_stored(self, question_id: i64) -> QuestionOption {
            QuestionOption {
                id: 0, // Dummy ID, not used in validation
                question_id,
                option_text: self.option.clone(),
                is_correct: self.is_correct,
            }
        }
    }
}
