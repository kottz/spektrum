use super::{Question, QuestionError, QuestionResult};
use csv::StringRecord;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Difficulty {
    Easy,
    Medium,
    Challenging,
    VeryChallenging,
    Expert,
    UltraHard,
}

impl Difficulty {
    fn from_str(s: &str) -> QuestionResult<Self> {
        match s.trim().to_lowercase().as_str() {
            "easy" => Ok(Difficulty::Easy),
            "medium" => Ok(Difficulty::Medium),
            "challenging" => Ok(Difficulty::Challenging),
            "very challenging" => Ok(Difficulty::VeryChallenging),
            "expert" => Ok(Difficulty::Expert),
            "ultra hard" => Ok(Difficulty::UltraHard),
            _ => Err(QuestionError::InvalidFormat(format!(
                "Invalid difficulty: {}",
                s
            ))),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CharacterQuestion {
    pub id: u32,
    pub difficulty: Difficulty,
    pub song: String,
    pub correct_character: String,
    pub other_characters: Vec<String>,
    pub spotify_uri: String,
    pub youtube_id: String,
}

impl CharacterQuestion {
    pub fn new(
        id: u32,
        difficulty: Difficulty,
        song: String,
        correct_character: String,
        other_characters: Vec<String>,
        spotify_uri: String,
        youtube_id: String,
    ) -> Self {
        Self {
            id,
            difficulty,
            song,
            correct_character,
            other_characters,
            spotify_uri,
            youtube_id,
        }
    }

    fn generate_character_alternatives(&self) -> Vec<String> {
        let mut rng = rand::thread_rng();

        let mut alternatives = vec![self.correct_character.clone()];
        let mut available_characters = self.other_characters.clone();
        available_characters.shuffle(&mut rng);

        alternatives.extend(
            available_characters
                .into_iter()
                .take(6 - alternatives.len()),
        );

        alternatives.shuffle(&mut rng);
        alternatives
    }

    // Helper function to validate alternative count
    fn validate_alternatives(&self) -> QuestionResult<()> {
        if self.other_characters.is_empty() {
            return Err(QuestionError::InvalidFormat(
                "Character question must have at least one alternative".into(),
            ));
        }
        Ok(())
    }
}

impl Question for CharacterQuestion {
    fn get_id(&self) -> u32 {
        self.id
    }

    fn get_correct_answer(&self) -> Vec<String> {
        vec![self.correct_character.clone()]
    }

    fn get_all_possible_alternatives(&self) -> Vec<String> {
        let mut alternatives = vec![self.correct_character.clone()];
        alternatives.extend(self.other_characters.clone());
        alternatives.into_iter().map(|c| c.to_string()).collect()
    }

    fn generate_round_alternatives(&self) -> Vec<String> {
        // Character questions don't need the full question set as alternatives
        // are predefined in the CSV
        self.generate_character_alternatives()
            .into_iter()
            .filter(|c| !self.correct_character.eq(c))
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

pub fn load_from_record(record: &StringRecord) -> QuestionResult<CharacterQuestion> {
    if record.len() < 7 {
        return Err(QuestionError::InvalidFormat(
            "Record does not have enough fields".into(),
        ));
    }

    let difficulty = Difficulty::from_str(&record[1])?;

    // Split and process other characters
    let other_characters: Vec<String> = record[4]
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let question = CharacterQuestion {
        id: record[0]
            .parse()
            .map_err(|_| QuestionError::InvalidFormat("Invalid ID".into()))?,
        difficulty,
        song: record[2].trim().to_string(),
        correct_character: record[3].trim().to_string(),
        other_characters,
        spotify_uri: record[5].to_string(),
        youtube_id: record[6].to_string(),
    };

    question.validate_alternatives()?;
    Ok(question)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_question() -> CharacterQuestion {
        CharacterQuestion::new(
            1,
            Difficulty::Easy,
            "Test Song".into(),
            "Main Character".into(),
            vec![
                "Alternative 1".into(),
                "Alternative 2".into(),
                "Alternative 3".into(),
                "Alternative 4".into(),
                "Alternative 5".into(),
            ],
            "spotify:uri".into(),
            "youtube123".into(),
        )
    }

    #[test]
    fn test_difficulty_from_str() {
        assert!(matches!(Difficulty::from_str("easy"), Ok(Difficulty::Easy)));
        assert!(matches!(
            Difficulty::from_str(" MEDIUM "),
            Ok(Difficulty::Medium)
        ));
        assert!(Difficulty::from_str("invalid").is_err());
    }

    #[test]
    fn test_generate_alternatives() {
        let question = create_test_question();
        let alternatives = question.generate_character_alternatives();

        assert_eq!(alternatives.len(), 6);
        assert!(alternatives.contains(&question.correct_character));

        // Check that all alternatives are unique
        let unique_count = alternatives
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert_eq!(unique_count, alternatives.len());
    }

    #[test]
    fn test_validate_alternatives() {
        let mut question = create_test_question();
        assert!(question.validate_alternatives().is_ok());

        // Test with empty alternatives
        question.other_characters.clear();
        assert!(question.validate_alternatives().is_err());
    }

    #[test]
    fn test_question_trait_implementation() {
        let question = create_test_question();
        let alternatives = question.get_all_possible_alternatives();

        assert_eq!(alternatives.len(), 6);
    }
}
