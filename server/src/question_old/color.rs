use super::{Question, QuestionError, QuestionResult};
use csv::StringRecord;
use lazy_static::lazy_static;
use rand::Rng;
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ColorError {
    #[error("Failed to acquire weights lock: {0}")]
    LockError(String),

    #[error("Invalid color format: {0}")]
    InvalidColor(String),
}

impl From<ColorError> for QuestionError {
    fn from(err: ColorError) -> Self {
        QuestionError::InvalidFormat(err.to_string())
    }
}

type ColorResult<T> = Result<T, ColorError>;

// Static storage for color weights
lazy_static! {
    static ref COLOR_WEIGHTS: Mutex<ColorWeightStore> = Mutex::new(ColorWeightStore::new());
}

// Structure to store color weights
#[derive(Default)]
struct ColorWeightStore {
    weights: HashMap<Color, f64>,
    total_songs: usize,
}

impl ColorWeightStore {
    fn new() -> Self {
        Self::default()
    }

    fn update_from_questions(&mut self, questions: &[ColorQuestion]) -> ColorResult<()> {
        let mut color_counts: HashMap<Color, usize> = HashMap::new();
        self.total_songs = questions.len();

        // Count occurrences of each color
        for question in questions {
            for &color in &question.colors {
                *color_counts.entry(color).or_insert(0) += 1;
            }
        }

        // Calculate weights for each color
        for &color in Color::all() {
            let count = color_counts.get(&color).copied().unwrap_or(0);
            let base_proportion = if self.total_songs > 0 {
                count as f64 / self.total_songs as f64
            } else {
                0.0
            };
            let weight = base_proportion.sqrt() + 0.15; // Apply square root transformation
            self.weights.insert(color, weight);
        }

        Ok(())
    }

    fn get_weight(&self, color: Color) -> f64 {
        self.weights.get(&color).copied().unwrap_or(0.15)
    }
}

// Public function to initialize weights
pub fn initialize_color_weights(questions: &[ColorQuestion]) -> ColorResult<()> {
    COLOR_WEIGHTS
        .lock()
        .map_err(|e| ColorError::LockError(e.to_string()))?
        .update_from_questions(questions)
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

    fn from_str(s: &str) -> Result<Self, ColorError> {
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
            _ => Err(ColorError::InvalidColor(format!("Invalid color: {}", s))),
        }
    }

    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ColorQuestion {
    pub id: u32,
    pub song: String,
    pub artist: String,
    pub colors: Vec<Color>,
    pub spotify_uri: String,
    pub youtube_id: String,
}

impl ColorQuestion {
    pub fn new(
        id: u32,
        song: String,
        artist: String,
        colors: Vec<Color>,
        spotify_uri: String,
        youtube_id: String,
    ) -> Self {
        Self {
            id,
            song,
            artist,
            colors,
            spotify_uri,
            youtube_id,
        }
    }
}

impl ColorQuestion {
    fn generate_weighted_alternatives(&self) -> ColorResult<Vec<Color>> {
        let mut rng = thread_rng();
        let mut round_colors = self.colors.clone();

        let weights = COLOR_WEIGHTS
            .lock()
            .map_err(|e| ColorError::LockError(e.to_string()))?;

        // If we already have 6 or more colors, shuffle and return
        if round_colors.len() >= 6 {
            round_colors.shuffle(&mut rng);
            return Ok(round_colors);
        }

        // Get available colors (excluding ones we already have)
        let mut available_colors: Vec<(Color, f64)> = Color::all()
            .iter()
            .copied()
            .filter(|color| !round_colors.contains(color))
            .map(|color| (color, weights.get_weight(color)))
            .collect();

        // Select additional colors based on weights until we have 6
        while round_colors.len() < 6 && !available_colors.is_empty() {
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
        Ok(round_colors)
    }
}

impl Question for ColorQuestion {
    fn get_id(&self) -> u32 {
        self.id
    }

    fn get_correct_answer(&self) -> Vec<String> {
        self.colors.iter().map(|color| color.to_string()).collect()
    }

    fn get_all_possible_alternatives(&self) -> Vec<String> {
        Color::all().iter().map(|color| color.to_string()).collect()
    }

    fn generate_round_alternatives(&self) -> Vec<String> {
        self.generate_weighted_alternatives()
            .map(|colors| colors.into_iter().map(|c| c.to_string()).collect())
            .unwrap_or_else(|_| self.get_all_possible_alternatives())
    }

    fn get_spotify_uri(&self) -> String {
        self.spotify_uri.clone()
    }

    fn get_youtube_id(&self) -> String {
        self.youtube_id.clone()
    }
}

pub fn load_from_record(record: &StringRecord) -> QuestionResult<ColorQuestion> {
    if record.len() < 6 {
        return Err(QuestionError::InvalidFormat(
            "Record does not have enough fields".into(),
        ));
    }

    let colors = record[3]
        .split(';')
        .map(Color::from_str)
        .collect::<Result<Vec<_>, ColorError>>()?;

    if colors.is_empty() {
        return Err(QuestionError::InvalidFormat("No valid colors found".into()));
    }

    Ok(ColorQuestion {
        id: record[0]
            .parse()
            .map_err(|_| QuestionError::InvalidFormat("Invalid ID".into()))?,
        song: record[1].trim().to_string(),
        artist: record[2].trim().to_string(),
        colors,
        spotify_uri: record[4].trim().to_string(),
        youtube_id: record[5].trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use csv::StringRecord;

    fn create_test_questions() -> Vec<ColorQuestion> {
        vec![
            ColorQuestion {
                id: 1,
                song: "Test Song 1".to_string(),
                artist: "Test Artist 1".to_string(),
                colors: vec![Color::Red, Color::Blue],
                spotify_uri: "spotify:1".to_string(),
                youtube_id: "yt1".to_string(),
            },
            ColorQuestion {
                id: 2,
                song: "Test Song 2".to_string(),
                artist: "Test Artist 2".to_string(),
                colors: vec![Color::Green, Color::Yellow],
                spotify_uri: "spotify:2".to_string(),
                youtube_id: "yt2".to_string(),
            },
        ]
    }

    #[test]
    fn test_color_from_str() {
        // Test valid colors
        assert!(matches!(Color::from_str("red"), Ok(Color::Red)));
        assert!(matches!(Color::from_str("BLUE"), Ok(Color::Blue)));
        assert!(matches!(Color::from_str(" Green "), Ok(Color::Green)));
        assert!(matches!(Color::from_str("grey"), Ok(Color::Gray)));
        assert!(matches!(Color::from_str("GRAY"), Ok(Color::Gray)));

        // Test invalid colors
        assert!(matches!(
            Color::from_str("invalid"),
            Err(ColorError::InvalidColor(_))
        ));
        assert!(matches!(
            Color::from_str(""),
            Err(ColorError::InvalidColor(_))
        ));
    }

    #[test]
    fn test_color_weight_initialization() {
        let questions = create_test_questions();
        let result = initialize_color_weights(&questions);
        assert!(result.is_ok());

        // Test weight access
        if let Ok(weights) = COLOR_WEIGHTS.lock() {
            assert!(weights.get_weight(Color::Red) > 0.0);
            assert!(weights.get_weight(Color::Blue) > 0.0);
            assert!(weights.get_weight(Color::Green) > 0.0);
            // Colors not in questions should have minimum weight
            assert_eq!(weights.get_weight(Color::Purple), 0.15);
        }
    }

    #[test]
    fn test_load_from_record() {
        let record = StringRecord::from(vec![
            "1",
            "Test Song",
            "Test Artist",
            "Red;Blue",
            "spotify:test",
            "yt123",
        ]);

        let result = load_from_record(&record);
        assert!(result.is_ok());

        if let Ok(question) = result {
            assert_eq!(question.id, 1);
            assert_eq!(question.song, "Test Song");
            assert_eq!(question.artist, "Test Artist");
            assert_eq!(question.colors.len(), 2);
            assert!(question.colors.contains(&Color::Red));
            assert!(question.colors.contains(&Color::Blue));
            assert_eq!(question.spotify_uri, "spotify:test");
            assert_eq!(question.youtube_id, "yt123");
        }
    }

    #[test]
    fn test_load_from_record_errors() {
        // Test record with too few fields
        let short_record = StringRecord::from(vec!["1", "Test Song", "Test Artist"]);
        assert!(matches!(
            load_from_record(&short_record),
            Err(QuestionError::InvalidFormat(_))
        ));

        // Test invalid color format
        let invalid_colors = StringRecord::from(vec![
            "1",
            "Test Song",
            "Test Artist",
            "Invalid;Colors",
            "spotify:test",
            "yt123",
        ]);
        assert!(matches!(
            load_from_record(&invalid_colors),
            Err(QuestionError::InvalidFormat(_))
        ));

        // Test invalid ID
        let invalid_id = StringRecord::from(vec![
            "not_a_number",
            "Test Song",
            "Test Artist",
            "Red;Blue",
            "spotify:test",
            "yt123",
        ]);
        assert!(matches!(
            load_from_record(&invalid_id),
            Err(QuestionError::InvalidFormat(_))
        ));

        // Test empty colors
        let empty_colors = StringRecord::from(vec![
            "1",
            "Test Song",
            "Test Artist",
            "",
            "spotify:test",
            "yt123",
        ]);
        assert!(matches!(
            load_from_record(&empty_colors),
            Err(QuestionError::InvalidFormat(_))
        ));
    }

    #[test]
    fn test_generate_weighted_alternatives() {
        let question = ColorQuestion {
            id: 1,
            song: "Test".to_string(),
            artist: "Test".to_string(),
            colors: vec![Color::Red, Color::Blue],
            spotify_uri: "test".to_string(),
            youtube_id: "test".to_string(),
        };

        // Initialize weights first
        let _ = initialize_color_weights(&vec![question.clone()]);

        // Test alternative generation
        let result = question.generate_weighted_alternatives();
        assert!(result.is_ok());
        if let Ok(alternatives) = result {
            assert!(alternatives.len() >= 2);
            assert!(alternatives.len() <= 6);
            assert!(alternatives.contains(&Color::Red));
            assert!(alternatives.contains(&Color::Blue));
        }
    }

    #[test]
    fn test_question_trait_implementation() {
        let question = ColorQuestion {
            id: 1,
            song: "Test".to_string(),
            artist: "Test".to_string(),
            colors: vec![Color::Red, Color::Blue],
            spotify_uri: "test".to_string(),
            youtube_id: "test".to_string(),
        };

        assert_eq!(question.get_id(), 1);
        assert_eq!(question.get_spotify_uri(), "test");
        assert_eq!(question.get_youtube_id(), "test");

        let correct_answers = question.get_correct_answer();
        assert_eq!(correct_answers.len(), 2);
        assert!(correct_answers.contains(&"Red".to_string()));
        assert!(correct_answers.contains(&"Blue".to_string()));

        let alternatives = question.get_all_possible_alternatives();
        assert_eq!(alternatives.len(), Color::all().len());
    }

    #[test]
    fn test_multiple_color_combinations() {
        let questions = vec![
            ColorQuestion {
                id: 1,
                song: "Test 1".to_string(),
                artist: "Artist".to_string(),
                colors: vec![
                    Color::Red,
                    Color::Blue,
                    Color::Green,
                    Color::Yellow,
                    Color::Purple,
                    Color::Gold,
                ],
                spotify_uri: "test".to_string(),
                youtube_id: "test".to_string(),
            },
            ColorQuestion {
                id: 2,
                song: "Test 2".to_string(),
                artist: "Artist".to_string(),
                colors: vec![Color::Silver],
                spotify_uri: "test".to_string(),
                youtube_id: "test".to_string(),
            },
        ];

        let _ = initialize_color_weights(&questions);

        // Test with many colors
        let question = &questions[0];
        let result = question.generate_weighted_alternatives();
        assert!(result.is_ok());
        if let Ok(alternatives) = result {
            assert_eq!(alternatives.len(), 6);
            // Check all original colors are included
            for color in &question.colors {
                assert!(alternatives.contains(color));
            }
        }

        // Test with single color
        let question = &questions[1];
        let result = question.generate_weighted_alternatives();
        assert!(result.is_ok());
        if let Ok(alternatives) = result {
            assert!(alternatives.len() >= 2);
            assert!(alternatives.contains(&Color::Silver));
        }
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let questions = create_test_questions();
        // Handle initialization error
        if let Err(e) = initialize_color_weights(&questions) {
            panic!("Failed to initialize color weights: {}", e);
        }

        let handles: Vec<_> = (0..10)
            .map(|_| {
                thread::spawn(|| {
                    let question = ColorQuestion {
                        id: 1,
                        song: "Test".to_string(),
                        artist: "Test".to_string(),
                        colors: vec![Color::Red, Color::Blue],
                        spotify_uri: "test".to_string(),
                        youtube_id: "test".to_string(),
                    };
                    question.generate_weighted_alternatives()
                })
            })
            .collect();

        for handle in handles {
            match handle.join() {
                Ok(result) => {
                    assert!(
                        result.is_ok(),
                        "Thread generated alternatives failed: {:?}",
                        result.err()
                    );
                }
                Err(e) => {
                    panic!("Thread panicked: {:?}", e);
                }
            }
        }
    }

    #[test]
    fn test_weight_store_empty() {
        let store = ColorWeightStore::new();
        assert_eq!(store.total_songs, 0);
        assert_eq!(store.get_weight(Color::Red), 0.15); // Default weight
    }
}
