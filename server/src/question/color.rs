use super::{Question, QuestionError, QuestionResult};
use csv::StringRecord;
use lazy_static::lazy_static;
use rand::Rng;
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

// Static storage for color weights
lazy_static! {
    static ref COLOR_WEIGHTS: Mutex<ColorWeightStore> = Mutex::new(ColorWeightStore::new());
}

// Structure to store color weights
struct ColorWeightStore {
    weights: HashMap<Color, f64>,
    total_songs: usize,
}

impl ColorWeightStore {
    fn new() -> Self {
        Self {
            weights: HashMap::new(),
            total_songs: 0,
        }
    }

    fn update_from_questions(&mut self, questions: &[ColorQuestion]) {
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
            let base_proportion = count as f64 / self.total_songs as f64;
            let weight = base_proportion.sqrt() + 0.15; // Apply square root transformation
            self.weights.insert(color, weight);
        }
    }

    fn get_weight(&self, color: Color) -> f64 {
        self.weights.get(&color).copied().unwrap_or(0.15)
    }
}

// Public function to initialize weights
pub fn initialize_color_weights(questions: &[ColorQuestion]) {
    let mut store = COLOR_WEIGHTS.lock().unwrap();
    store.update_from_questions(questions);
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

    fn from_str(s: &str) -> QuestionResult<Self> {
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
            _ => Err(QuestionError::InvalidFormat(format!(
                "Invalid color: {}",
                s
            ))),
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
    fn generate_weighted_alternatives(&self) -> Vec<Color> {
        let mut rng = thread_rng();
        let mut round_colors = self.colors.clone();
        let weights = COLOR_WEIGHTS.lock().unwrap();

        // If we already have 6 or more colors, shuffle and return
        if round_colors.len() >= 6 {
            round_colors.shuffle(&mut rng);
            return round_colors;
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
        round_colors
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
            .into_iter()
            .map(|c| c.to_string())
            .collect()
    }

    fn get_spotify_uri(&self) -> String {
        self.spotify_uri.clone()
    }

    fn get_youtube_id(&self) -> String {
        self.youtube_id.clone()
    }
}

// Modified load function that also initializes weights
pub fn load_from_csv(filepath: &str) -> QuestionResult<Vec<ColorQuestion>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(filepath)?;

    let mut questions = Vec::new();

    for result in reader.records() {
        let record = result?;
        questions.push(load_from_record(&record)?);
    }

    // Initialize weights after loading all questions
    initialize_color_weights(&questions);

    Ok(questions)
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
        .collect::<Result<Vec<_>, _>>()?;

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

    #[test]
    fn test_color_from_str() {
        assert!(matches!(Color::from_str("RED"), Ok(Color::Red)));
        assert!(matches!(Color::from_str(" blue "), Ok(Color::Blue)));
        assert!(matches!(Color::from_str("GREY"), Ok(Color::Gray)));
        assert!(Color::from_str("InvalidColor").is_err());
    }

    #[test]
    fn test_generate_alternatives() {
        let question = ColorQuestion::new(
            1,
            "Test Song".into(),
            "Test Artist".into(),
            vec![Color::Blue],
            "spotify:uri".into(),
            "youtube123".into(),
        );

        let color_stats = HashMap::from([
            (
                Color::Blue,
                ColorStats {
                    frequency: 0.5,
                    song_count: 5,
                },
            ),
            (
                Color::Red,
                ColorStats {
                    frequency: 0.3,
                    song_count: 3,
                },
            ),
        ]);

        let alternatives = question.generate_weighted_alternatives(&color_stats);

        assert!(alternatives.len() == 6);
        assert!(alternatives.contains(&Color::Blue));
        assert!(alternatives.iter().all(|c| Color::all().contains(c)));
    }
}
