use crate::question::{GameQuestion, Question};
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum GamePhase {
    Lobby,
    Score,
    Question,
    GameOver,
}

// PlayerState remains mostly the same, just using String for answers
#[derive(Clone, Debug)]
pub struct PlayerState {
    pub name: String,
    pub score: i32,
    pub has_answered: bool,
    pub answer: Option<String>,
}

impl PlayerState {
    pub fn new(name: String) -> Self {
        Self {
            name,
            score: 0,
            has_answered: false,
            answer: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EventContext {
    pub lobby_id: Uuid,
    pub sender_id: Uuid,
    pub timestamp: Instant,
    pub is_admin: bool,
}

#[derive(Clone, Debug)]
pub enum Recipients {
    Single(Uuid),
    Multiple(Vec<Uuid>),
    AllExcept(Vec<Uuid>),
    All,
}

#[derive(Clone, Debug)]
pub struct GameEvent {
    pub context: EventContext,
    pub action: GameAction,
}

// Updated GameAction to be question-type agnostic
#[derive(Clone, Debug)]
pub enum GameAction {
    Join {
        name: String,
    },
    Leave,
    Answer {
        answer: String,
    },
    StartGame,
    StartRound {
        specified_alternatives: Option<Vec<String>>,
    },
    EndRound,
    SkipQuestion,
    EndGame {
        reason: String,
    },
    CloseGame {
        reason: String,
    },
}

#[derive(Clone, Debug)]
pub struct GameResponse {
    pub recipients: Recipients,
    pub payload: ResponsePayload,
}

// Updated ResponsePayload to handle any question type
#[derive(Clone, Debug)]
pub enum ResponsePayload {
    Joined {
        player_id: Uuid,
        lobby_id: Uuid,
        name: String,
        round_duration: u64,
        current_players: Vec<(String, i32)>,
    },
    PlayerLeft {
        name: String,
    },
    PlayerAnswered {
        name: String,
        correct: bool,
        new_score: i32,
    },
    StateChanged {
        phase: GamePhase,
        question_type: String,
        alternatives: Vec<String>,
        scoreboard: Vec<(String, i32)>,
    },
    GameOver {
        final_scores: Vec<(String, i32)>,
        reason: String,
    },
    GameClosed {
        reason: String,
    },
    AdminInfo {
        current_question: GameQuestion,
    },
    AdminNextQuestions {
        upcoming_questions: Vec<GameQuestion>,
    },
    Error {
        code: ErrorCode,
        message: String,
    },
}

#[derive(Clone, Debug)]
pub enum ErrorCode {
    NotAuthorized,
    InvalidPhase,
    InvalidAction,
    GameClosed,
    PlayerNotFound,
    AlreadyAnswered,
    TimeExpired,
    LobbyNotFound,
    InvalidName,
}

#[derive(Debug)]
pub enum NameValidationError {
    TooShort,
    TooLong,
    InvalidCharacters,
    AlreadyTaken,
}

impl NameValidationError {
    fn to_message(&self) -> String {
        match self {
            Self::TooShort => "Name must be at least 2 characters long.".into(),
            Self::TooLong => "Name cannot be longer than 16 characters.".into(),
            Self::InvalidCharacters => {
                "Name can only contain letters, numbers, spaces, and the symbols: _ - .".into()
            }
            Self::AlreadyTaken => "This name is already taken.".into(),
        }
    }
}

fn validate_player_name<'a>(
    name: &str,
    mut existing_names: impl Iterator<Item = &'a String>,
) -> Result<(), NameValidationError> {
    let name = name.trim();

    if name.len() < 2 {
        return Err(NameValidationError::TooShort);
    }
    if name.len() > 16 {
        return Err(NameValidationError::TooLong);
    }

    let name_regex = regex::Regex::new(r"^[a-zA-Z0-9_\-\. ]+$").unwrap();
    if !name_regex.is_match(name) {
        return Err(NameValidationError::InvalidCharacters);
    }

    if existing_names.any(|existing_name| existing_name == name) {
        return Err(NameValidationError::AlreadyTaken);
    }

    Ok(())
}

pub struct GameState {
    pub phase: GamePhase,
    pub players: HashMap<Uuid, PlayerState>,
    pub admin_id: Uuid,
    pub round_start_time: Option<Instant>,
    pub round_duration: u64,
    // Remove color specific fields:
    // pub colors: Vec<ColorDef>,
    // pub correct_colors: Vec<String>,
    // pub all_colors: Vec<ColorDef>,

    // Replace song fields with question fields:
    pub current_alternatives: Vec<String>,
    pub correct_answers: Option<Vec<String>>,
    pub current_question: Option<GameQuestion>,
    pub used_questions: HashSet<u32>, // Track by question ID instead of spotify_uri
    pub all_questions: Vec<GameQuestion>, // Replace all_songs
    pub current_question_index: usize, // Rename but same functionality
}

pub struct GameEngine {
    state: GameState,
}

impl GameEngine {
    pub fn new(admin_id: Uuid, mut questions: Vec<GameQuestion>, round_duration: u64) -> Self {
        let mut rng = rand::thread_rng();
        questions.shuffle(&mut rng);
        Self {
            state: GameState {
                phase: GamePhase::Lobby,
                players: HashMap::new(),
                admin_id,
                round_start_time: None,
                round_duration,
                current_alternatives: Vec::new(),
                correct_answers: None,
                current_question: None,
                used_questions: HashSet::new(),
                all_questions: questions,
                current_question_index: 0,
            },
        }
    }

    pub fn process_event(&mut self, event: GameEvent) -> Vec<GameResponse> {
        let GameEvent { context, action } = event;

        match &action {
            GameAction::StartGame
            | GameAction::StartRound { .. }
            | GameAction::EndRound
            | GameAction::EndGame { .. }
            | GameAction::CloseGame { .. } => {
                if context.sender_id != self.state.admin_id {
                    return vec![GameResponse {
                        recipients: Recipients::Single(context.sender_id),
                        payload: ResponsePayload::Error {
                            code: ErrorCode::NotAuthorized,
                            message: "Admin action requires authorization".into(),
                        },
                    }];
                }
            }
            _ => {}
        }

        match action {
            GameAction::Join { name } => self.handle_join(context, name),
            GameAction::Leave => self.handle_leave(context),
            GameAction::Answer { answer } => self.handle_answer(context, answer),
            GameAction::StartGame => self.handle_start_game(context),
            GameAction::StartRound {
                specified_alternatives,
            } => self.handle_start_round(context, specified_alternatives),
            GameAction::EndRound => self.handle_end_round(context),
            GameAction::SkipQuestion => self.handle_skip_question(context),
            GameAction::EndGame { reason } => self.handle_end_game(context, reason),
            GameAction::CloseGame { reason } => self.handle_close_game(context, reason),
        }
    }

    fn handle_join(&mut self, ctx: EventContext, name: String) -> Vec<GameResponse> {
        if self.state.admin_id == ctx.sender_id {
            return vec![];
        }

        let name = name.trim().to_string();

        let existing_names = self.state.players.values().map(|p| &p.name);
        if let Err(validation_error) = validate_player_name(&name, existing_names) {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidName,
                    message: validation_error.to_message(),
                },
            }];
        }

        if self.state.phase != GamePhase::Lobby {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidPhase,
                    message: "You can't join a game that has already started.".into(),
                },
            }];
        }

        let current_players: Vec<(String, i32)> = self
            .state
            .players
            .values()
            .map(|p| (p.name.clone(), p.score))
            .collect();

        self.state
            .players
            .insert(ctx.sender_id, PlayerState::new(name.clone()));

        vec![
            GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Joined {
                    player_id: ctx.sender_id,
                    lobby_id: ctx.lobby_id,
                    name,
                    round_duration: self.state.round_duration,
                    current_players,
                },
            },
            GameResponse {
                recipients: Recipients::AllExcept(vec![ctx.sender_id]),
                payload: ResponsePayload::StateChanged {
                    phase: self.state.phase,
                    question_type: "".to_string(),
                    alternatives: self.state.current_alternatives.clone(), // Changed from colors
                    scoreboard: self.get_scoreboard(),
                },
            },
        ]
    }

    fn handle_leave(&mut self, ctx: EventContext) -> Vec<GameResponse> {
        if let Some(player) = self.state.players.remove(&ctx.sender_id) {
            if ctx.sender_id == self.state.admin_id {
                vec![GameResponse {
                    recipients: Recipients::All,
                    payload: ResponsePayload::GameClosed {
                        reason: "Host left the game".into(),
                    },
                }]
            } else {
                vec![GameResponse {
                    recipients: Recipients::All,
                    payload: ResponsePayload::PlayerLeft { name: player.name },
                }]
            }
        } else {
            vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::PlayerNotFound,
                    message: "Player not found".into(),
                },
            }]
        }
    }

    fn handle_answer(&mut self, ctx: EventContext, answer: String) -> Vec<GameResponse> {
        if self.state.phase != GamePhase::Question {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidPhase,
                    message: "Not in Question phase".into(),
                },
            }];
        }

        let player = match self.state.players.get_mut(&ctx.sender_id) {
            Some(p) => p,
            None => {
                return vec![GameResponse {
                    recipients: Recipients::Single(ctx.sender_id),
                    payload: ResponsePayload::Error {
                        code: ErrorCode::PlayerNotFound,
                        message: "Player not found".into(),
                    },
                }]
            }
        };

        if player.has_answered {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::AlreadyAnswered,
                    message: "Already answered this round".into(),
                },
            }];
        }

        let elapsed = ctx.timestamp.duration_since(
            self.state
                .round_start_time
                .expect("Round start time should be set"),
        );

        if elapsed.as_secs() > self.state.round_duration {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::TimeExpired,
                    message: "Time expired for this round".into(),
                },
            }];
        }

        let correct = self
            .state
            .correct_answers
            .as_ref()
            .map_or(false, |answers| answers.contains(&answer));
        let new_score = if correct {
            let score_delta = ((self.state.round_duration as f64 * 100.0
                - (elapsed.as_secs_f64() * 100.0))
                .max(0.0)) as i32;
            player.score += score_delta;
            player.score
        } else {
            player.score
        };

        player.has_answered = true;
        player.answer = Some(answer);

        vec![GameResponse {
            recipients: Recipients::All,
            payload: ResponsePayload::PlayerAnswered {
                name: player.name.clone(),
                correct,
                new_score,
            },
        }]
    }

    fn handle_start_game(&mut self, ctx: EventContext) -> Vec<GameResponse> {
        if self.state.phase != GamePhase::Lobby {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidPhase,
                    message: "Can only start game from lobby".into(),
                },
            }];
        }

        self.state.phase = GamePhase::Score;

        let mut responses = vec![GameResponse {
            recipients: Recipients::All,
            payload: ResponsePayload::StateChanged {
                phase: GamePhase::Score,
                question_type: "".to_string(),
                alternatives: Vec::new(),
                scoreboard: self.get_scoreboard(),
            },
        }];

        // Send next songs to admin after transitioning to Score phase
        let upcoming = self.get_upcoming_questions(3);
        if !upcoming.is_empty() {
            responses.push(GameResponse {
                recipients: Recipients::Single(self.state.admin_id),
                payload: ResponsePayload::AdminNextQuestions {
                    upcoming_questions: upcoming,
                },
            });
        }

        responses
    }

    fn handle_start_round(
        &mut self,
        ctx: EventContext,
        specified_alternatives: Option<Vec<String>>,
    ) -> Vec<GameResponse> {
        if self.state.phase != GamePhase::Score {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidPhase,
                    message: "Can only start round from score phase".into(),
                },
            }];
        }

        // Reset all players' answer state
        for player in self.state.players.values_mut() {
            player.has_answered = false;
            player.answer = None;
        }

        // Setup the round with the current question or specified alternatives
        match self.setup_round(specified_alternatives) {
            Ok(()) => {
                self.state.phase = GamePhase::Question;
                self.state.round_start_time = Some(ctx.timestamp);
                // find if questoins are color or character

                let mut outputs = Vec::new();
                outputs.push(GameResponse {
                    recipients: Recipients::All,
                    payload: ResponsePayload::StateChanged {
                        phase: GamePhase::Question,
                        question_type: self
                            .state
                            .current_question
                            .as_ref()
                            .unwrap()
                            .get_question_type()
                            .to_string(),
                        alternatives: self.state.current_alternatives.clone(),
                        scoreboard: self.get_scoreboard(),
                    },
                });

                // If there's a current question, send it to the admin
                if let Some(question) = &self.state.current_question {
                    outputs.push(GameResponse {
                        recipients: Recipients::Single(self.state.admin_id),
                        payload: ResponsePayload::AdminInfo {
                            current_question: question.clone(),
                        },
                    });
                }

                outputs
            }
            Err(msg) => vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidAction,
                    message: msg,
                },
            }],
        }
    }

    fn handle_end_round(&mut self, ctx: EventContext) -> Vec<GameResponse> {
        // Phase check remains the same
        if self.state.phase != GamePhase::Question {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidPhase,
                    message: "Can only end round from question phase".into(),
                },
            }];
        }

        // Mark current question as used
        if let Some(question) = &self.state.current_question {
            self.state.used_questions.insert(question.get_id());
        }

        // Reset state for next round
        self.state.current_question = None;
        self.state.current_question_index += 1;
        self.state.current_alternatives.clear();
        self.state.correct_answers = None;
        self.state.phase = GamePhase::Score;

        let mut responses = vec![GameResponse {
            recipients: Recipients::All,
            payload: ResponsePayload::StateChanged {
                phase: GamePhase::Score,
                question_type: "".to_string(),
                alternatives: Vec::new(),
                scoreboard: self.get_scoreboard(),
            },
        }];

        // Send next 3 upcoming questions to admin
        let upcoming = self.get_upcoming_questions(3);
        if !upcoming.is_empty() {
            responses.push(GameResponse {
                recipients: Recipients::Single(self.state.admin_id),
                payload: ResponsePayload::AdminNextQuestions {
                    upcoming_questions: upcoming,
                },
            });
        }

        responses
    }

    fn handle_skip_question(&mut self, ctx: EventContext) -> Vec<GameResponse> {
        if self.state.phase != GamePhase::Score {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidPhase,
                    message: "Can only skip question during scoreboard phase".into(),
                },
            }];
        }

        // Mark current question as used if it exists
        if let Some(question) = &self.state.current_question {
            self.state.used_questions.insert(question.get_id());
        }

        // Reset state for next question
        self.state.current_question = None;
        self.state.current_question_index += 1;
        self.state.current_alternatives.clear();
        self.state.correct_answers = None;

        let mut responses = vec![GameResponse {
            recipients: Recipients::All,
            payload: ResponsePayload::StateChanged {
                phase: GamePhase::Score,
                question_type: "".to_string(),
                alternatives: Vec::new(),
                scoreboard: self.get_scoreboard(),
            },
        }];

        // Send next 3 upcoming questions to admin
        let upcoming = self.get_upcoming_questions(3);
        if !upcoming.is_empty() {
            responses.push(GameResponse {
                recipients: Recipients::Single(self.state.admin_id),
                payload: ResponsePayload::AdminNextQuestions {
                    upcoming_questions: upcoming,
                },
            });
        }

        responses
    }

    fn handle_end_game(&mut self, _ctx: EventContext, reason: String) -> Vec<GameResponse> {
        self.state.phase = GamePhase::GameOver;

        vec![GameResponse {
            recipients: Recipients::All,
            payload: ResponsePayload::GameOver {
                final_scores: self.get_scoreboard(),
                reason,
            },
        }]
    }

    fn handle_close_game(&mut self, _ctx: EventContext, reason: String) -> Vec<GameResponse> {
        vec![GameResponse {
            recipients: Recipients::All,
            payload: ResponsePayload::GameClosed { reason },
        }]
    }

    fn get_scoreboard(&self) -> Vec<(String, i32)> {
        self.state
            .players
            .values()
            .map(|p| (p.name.clone(), p.score))
            .collect()
    }

    fn setup_round(&mut self, specified_alternatives: Option<Vec<String>>) -> Result<(), String> {
        println!(
            "Setting up round with specified alternatives: {:?}",
            specified_alternatives
        );
        if self.state.current_question_index >= self.state.all_questions.len() {
            return Err("No more questions available".to_string());
        }

        let next_question = &self.state.all_questions[self.state.current_question_index];

        if self.state.used_questions.contains(&next_question.get_id()) {
            // This should ideally not happen if the index is managed correctly,
            // but adding a safeguard.
            return Err("This question has already been used".to_string());
        }

        self.state.current_question = Some(next_question.clone());
        self.state.correct_answers = Some(next_question.get_correct_answer());

        if let Some(alts) = specified_alternatives {
            println!("Using specified alternatives: {:?}", alts);
            self.state.current_alternatives = alts;
        } else {
            self.state.current_alternatives = next_question.generate_round_alternatives();
            // Ensure all correct answers are included and alternatives are unique, then shuffle
            if let Some(correct_answers) = self.state.correct_answers.clone() {
                for answer in correct_answers {
                    if !self.state.current_alternatives.contains(&answer) {
                        self.state.current_alternatives.push(answer);
                    }
                }
            }
            self.state.current_alternatives.dedup();
            let mut rng = rand::thread_rng();
            self.state.current_alternatives.shuffle(&mut rng);
        }

        println!(
            "Round setup current_question: {:?}",
            self.state.current_question
        );
        println!(
            "Round setup current answers: {:?}",
            self.state.correct_answers
        );
        println!(
            "Round setup complete alternatives: {:?}",
            self.state.current_alternatives
        );

        Ok(())
    }
    // fn setup_round_colors(&mut self, specified_colors: Option<Vec<String>>) -> Result<(), String> {
    //     self.state.colors.clear();
    //     self.state.correct_colors.clear();
    //
    //     if let Some(specs) = specified_colors {
    //         // Handle manually specified colors
    //         let chosen: Vec<ColorDef> = specs
    //             .iter()
    //             .filter_map(|name| {
    //                 self.state
    //                     .all_colors
    //                     .iter()
    //                     .find(|c| c.name.eq_ignore_ascii_case(name))
    //                     .cloned()
    //             })
    //             .collect();
    //
    //         if chosen.is_empty() {
    //             return Err("No valid specified colors".to_string());
    //         }
    //         self.state.colors = chosen.clone();
    //         self.state.correct_colors = chosen.iter().map(|c| c.name.clone()).collect();
    //         self.state.current_song = None;
    //         Ok(())
    //     } else {
    //         // Handle song-based colors
    //         if self.state.current_song_index >= self.state.all_songs.len() {
    //             return Err("No available songs".to_string());
    //         }
    //
    //         let chosen_song = self.state.all_songs[self.state.current_song_index].clone();
    //         if self.state.used_songs.contains(&chosen_song.spotify_uri) {
    //             return Err("No available songs".to_string());
    //         }
    //
    //         let chosen_correct_colors: Vec<ColorDef> = chosen_song
    //             .colors
    //             .iter()
    //             .filter_map(|cname| {
    //                 self.state
    //                     .all_colors
    //                     .iter()
    //                     .find(|c| c.name.eq_ignore_ascii_case(cname))
    //                     .cloned()
    //             })
    //             .collect();
    //
    //         if chosen_correct_colors.is_empty() {
    //             return Err("Song has no valid colors".to_string());
    //         }
    //
    //         self.state.current_song = Some(chosen_song);
    //         self.state.correct_colors = chosen_correct_colors
    //             .iter()
    //             .map(|c| c.name.clone())
    //             .collect();
    //         self.state.colors = self.generate_round_colors(chosen_correct_colors);
    //         Ok(())
    //     }
    // }
    //
    // fn calculate_color_weights(&self) -> HashMap<String, f64> {
    //     let mut color_counts: HashMap<String, usize> = HashMap::new();
    //     let total_songs = self.state.all_songs.len() as f64;
    //
    //     // Count how many songs each color appears in
    //     for song in &self.state.all_songs {
    //         for color in &song.colors {
    //             *color_counts.entry(color.clone()).or_insert(0) += 1;
    //         }
    //     }
    //
    //     // Convert counts to adjusted weights using square root to boost rare colors
    //     let mut color_weights: HashMap<String, f64> = HashMap::new();
    //     for (color, count) in color_counts {
    //         // Calculate base proportion
    //         let base_proportion = count as f64 / total_songs;
    //
    //         // Apply square root transformation and add a minimum boost
    //         // This compresses the range between common and rare colors
    //         let adjusted_weight = base_proportion.sqrt() + 0.15;
    //
    //         color_weights.insert(color, adjusted_weight);
    //     }
    //
    //     // Also ensure any colors in all_colors that haven't appeared in songs get a minimum weight
    //     for color in &self.state.all_colors {
    //         color_weights.entry(color.name.clone()).or_insert(0.15); // Minimum weight for colors that never appear
    //     }
    //
    //     color_weights
    // }
    //
    // fn generate_round_colors(&self, correct_colors: Vec<ColorDef>) -> Vec<ColorDef> {
    //     let mut round_colors = correct_colors.clone();
    //     let mut rng = rand::thread_rng();
    //
    //     if round_colors.len() >= 6 {
    //         round_colors.shuffle(&mut rng);
    //         return round_colors;
    //     }
    //
    //     // Calculate weights for all colors
    //     let color_weights = self.calculate_color_weights();
    //
    //     // Get available colors (not in correct_colors)
    //     let mut available: Vec<ColorDef> = self
    //         .state
    //         .all_colors
    //         .iter()
    //         .filter(|col| !correct_colors.contains(col))
    //         .cloned()
    //         .collect();
    //
    //     // Create initial weights for available colors
    //     let mut weights: Vec<f64> = available
    //         .iter()
    //         .map(|color| color_weights.get(&color.name).copied().unwrap_or(0.15))
    //         .collect();
    //
    //     // Select remaining colors
    //     while round_colors.len() < 6 && !available.is_empty() {
    //         // Create distribution for current weights
    //         if let Ok(dist) = WeightedIndex::new(&weights) {
    //             let idx = dist.sample(&mut rng);
    //             round_colors.push(available.remove(idx));
    //             weights.remove(idx);
    //         } else {
    //             // Fallback to simple random if weights are invalid
    //             let idx = rng.gen_range(0..available.len());
    //             round_colors.push(available.remove(idx));
    //             weights.remove(idx);
    //         }
    //     }
    //
    //     round_colors.shuffle(&mut rng);
    //     round_colors
    // }
    //
    // // Helper function to view the actual weights being used
    // fn get_color_statistics(&self) -> Vec<ColorStats> {
    //     let color_weights = self.calculate_color_weights();
    //     let mut stats: Vec<ColorStats> = color_weights
    //         .into_iter()
    //         .map(|(name, weight)| {
    //             let song_count = self
    //                 .state
    //                 .all_songs
    //                 .iter()
    //                 .filter(|song| song.colors.contains(&name))
    //                 .count();
    //
    //             ColorStats {
    //                 name,
    //                 frequency: weight,
    //                 song_count,
    //             }
    //         })
    //         .collect();
    //
    //     stats.sort_by(|a, b| b.frequency.partial_cmp(&a.frequency).unwrap());
    //     stats
    // }

    fn get_upcoming_questions(&self, count: usize) -> Vec<GameQuestion> {
        let start = self.state.current_question_index;
        let end = std::cmp::min(start + count, self.state.all_questions.len());
        let mut upcoming = Vec::new();

        for i in start..end {
            if !self
                .state
                .used_questions
                .contains(&self.state.all_questions[i].get_id())
            {
                upcoming.push(self.state.all_questions[i].clone());
            }
        }
        upcoming
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::question::character::{CharacterQuestion, Difficulty};
    use crate::question::color::{Color, ColorQuestion};

    fn setup_test_data() -> Vec<GameQuestion> {
        let color_questions = vec![
            GameQuestion::Color(ColorQuestion::new(
                1,
                "Test Song 1".to_string(),
                "Test Artist".to_string(),
                vec![Color::Red],
                "test:song:1".to_string(),
                "xyz123".to_string(),
            )),
            GameQuestion::Color(ColorQuestion::new(
                2,
                "Test Song 2".to_string(),
                "Test Artist".to_string(),
                vec![Color::Blue],
                "test:song:2".to_string(),
                "xyz456".to_string(),
            )),
        ];

        let character_questions = vec![GameQuestion::Character(CharacterQuestion::new(
            3,
            Difficulty::Easy,
            "Test Song 3".to_string(),
            "Character A".to_string(),
            vec!["Character B".to_string(), "Character C".to_string()],
            "test:song:3".to_string(),
            "xyz789".to_string(),
        ))];

        let mut questions = color_questions;
        questions.extend(character_questions);
        questions
    }

    #[test]
    fn test_game_initialization() {
        let questions = setup_test_data();
        let admin_id = Uuid::new_v4();
        let engine = GameEngine::new(admin_id, questions, 30);
        assert_eq!(engine.state.phase, GamePhase::Lobby);
        assert!(engine.state.current_alternatives.is_empty());
        assert!(engine.state.correct_answer.is_none());
    }

    #[test]
    fn test_join_game() {
        let questions = setup_test_data();
        let admin_id = Uuid::new_v4();
        let mut engine = GameEngine::new(admin_id, questions, 30);
        let player_id = Uuid::new_v4();

        let ctx = EventContext {
            lobby_id: Uuid::new_v4(),
            sender_id: player_id,
            timestamp: Instant::now(),
            is_admin: false,
        };

        let event = GameEvent {
            context: ctx,
            action: GameAction::Join {
                name: "TestPlayer".to_string(),
            },
        };

        let responses = engine.process_event(event);
        assert!(!responses.is_empty());
        assert!(engine.state.players.contains_key(&player_id));

        // Verify response structure
        if let ResponsePayload::Joined { name, .. } = &responses[0].payload {
            assert_eq!(name, "TestPlayer");
        } else {
            panic!("Expected Joined response");
        }

        // Verify StateChanged has empty alternatives in lobby
        if let ResponsePayload::StateChanged { alternatives, .. } = &responses[1].payload {
            assert!(alternatives.is_empty());
        } else {
            panic!("Expected StateChanged response");
        }
    }

    #[test]
    fn test_game_start() {
        let questions = setup_test_data();
        let admin_id = Uuid::new_v4();
        let mut engine = GameEngine::new(admin_id, questions, 30);

        let ctx = EventContext {
            lobby_id: Uuid::new_v4(),
            sender_id: admin_id,
            timestamp: Instant::now(),
            is_admin: true,
        };

        let event = GameEvent {
            context: ctx,
            action: GameAction::StartGame,
        };

        let responses = engine.process_event(event);
        assert!(!responses.is_empty());
        assert_eq!(engine.state.phase, GamePhase::Score);

        // Verify admin gets upcoming questions
        if let Some(response) = responses
            .iter()
            .find(|r| matches!(r.payload, ResponsePayload::AdminNextQuestions { .. }))
        {
            if let ResponsePayload::AdminNextQuestions { upcoming_questions } = &response.payload {
                assert!(!upcoming_questions.is_empty());
            }
        } else {
            panic!("Expected AdminNextQuestions response");
        }
    }

    #[test]
    fn test_answer_handling() {
        let questions = setup_test_data();
        let admin_id = Uuid::new_v4();
        let mut engine = GameEngine::new(admin_id, questions, 30);
        let player_id = Uuid::new_v4();

        // Setup game state
        engine.state.phase = GamePhase::Question;
        engine.state.round_start_time = Some(Instant::now());
        engine.state.correct_answer = Some("RED".to_string());
        engine.state.current_alternatives = vec!["RED".to_string(), "BLUE".to_string()];
        engine
            .state
            .players
            .insert(player_id, PlayerState::new("TestPlayer".to_string()));

        let ctx = EventContext {
            lobby_id: Uuid::new_v4(),
            sender_id: player_id,
            timestamp: Instant::now(),
            is_admin: false,
        };

        let event = GameEvent {
            context: ctx,
            action: GameAction::Answer {
                answer: "RED".to_string(),
            },
        };

        let responses = engine.process_event(event);
        if let ResponsePayload::PlayerAnswered { correct, .. } = &responses[0].payload {
            assert!(correct);
        } else {
            panic!("Expected PlayerAnswered response");
        }
    }

    #[test]
    fn test_name_validation() {
        let empty_names = std::iter::empty();

        // Test valid names
        assert!(validate_player_name("John", empty_names.clone()).is_ok());
        assert!(validate_player_name("Player_1", empty_names.clone()).is_ok());
        assert!(validate_player_name("Cool-Name.123", empty_names.clone()).is_ok());

        // Test invalid names
        assert!(matches!(
            validate_player_name("a", empty_names.clone()),
            Err(NameValidationError::TooShort)
        ));
        assert!(matches!(
            validate_player_name("a".repeat(17).as_str(), empty_names.clone()),
            Err(NameValidationError::TooLong)
        ));
        assert!(matches!(
            validate_player_name("Invalid@Name", empty_names.clone()),
            Err(NameValidationError::InvalidCharacters)
        ));

        // Test duplicate names
        let existing_names = vec!["John".to_string()];
        assert!(matches!(
            validate_player_name("John", existing_names.iter()),
            Err(NameValidationError::AlreadyTaken)
        ));
    }
}
