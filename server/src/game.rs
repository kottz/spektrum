use crate::db::QuestionSet;
use crate::question::GameQuestion;
use rand::seq::SliceRandom;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, time::Instant};
use tracing::warn;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum GamePhase {
    Lobby,
    Score,
    Question,
    GameOver,
}

#[derive(Clone, Debug)]
pub struct PlayerState {
    pub name: String,
    pub score: i32,
    pub round_score: i32,
    pub has_answered: bool,
    pub answer: Option<String>,
}

impl PlayerState {
    pub fn new(name: String) -> Self {
        Self {
            name,
            score: 0,
            round_score: 0,
            has_answered: false,
            answer: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameStateResponse {
    pub phase: GamePhase,
    pub question_type: String,
    pub alternatives: Vec<String>,
    pub scoreboard: Vec<(String, i32)>,
    pub round_scores: Vec<(String, i32)>,
    pub current_song: Option<CurrentSongInfo>,
}

#[derive(Clone, Debug)]
pub struct CurrentSongInfo {
    pub song_name: String,
    pub artist: String,
    pub youtube_id: String,
}

#[derive(Clone, Debug)]
pub struct EventContext {
    pub lobby_id: Uuid,
    pub sender_id: Uuid,
    pub timestamp: Instant,
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
    GetState,
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
    Reconnected {
        game_state: GameStateResponse,
    },
    PlayerLeft {
        name: String,
    },
    PlayerAnswered {
        name: String,
        correct: bool,
        new_score: i32,
        round_score: i32,
    },
    StateChanged {
        phase: GamePhase,
        question_type: String,
        alternatives: Vec<String>,
        scoreboard: Vec<(String, i32)>,
        round_scores: Vec<(String, i32)>,
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
    PlayerNotFound,
    AlreadyAnswered,
    TimeExpired,
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

    let name_regex = regex::Regex::new(r"^[a-zA-Z0-9_\-\. ]+$").map_err(|_| {
        warn!("Failed to compile name regex");
        NameValidationError::InvalidCharacters
    })?;

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
    pub current_alternatives: Vec<String>,
    pub correct_answers: Option<Vec<String>>,
    pub current_question: Option<GameQuestion>,
    pub all_questions: Arc<Vec<GameQuestion>>,
    pub shuffled_question_indices: Vec<usize>,
    pub current_question_index: usize,
}

pub struct GameEngine {
    state: GameState,
}

impl GameEngine {
    pub fn new(
        admin_id: Uuid,
        questions: Arc<Vec<GameQuestion>>,
        set: Option<&QuestionSet>,
        round_duration: u64,
    ) -> Self {
        let mut rng = rand::thread_rng();

        let indices = match set {
            None => {
                // No set -> use all questions
                let mut all_indices: Vec<usize> = (0..questions.len()).collect();
                all_indices.shuffle(&mut rng);
                all_indices
            }
            Some(question_set) => {
                let id_to_index: HashMap<i64, usize> = questions
                    .iter()
                    .enumerate()
                    .map(|(idx, q)| (i64::from(q.id), idx))
                    .collect();

                // Build indices array only for questions that exist in the set
                let mut set_indices: Vec<usize> = question_set
                    .question_ids
                    .iter()
                    .filter_map(|id| id_to_index.get(id).copied())
                    .collect();

                set_indices.shuffle(&mut rng);
                set_indices
            }
        };

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
                all_questions: questions,
                shuffled_question_indices: indices,
                current_question_index: 0,
            },
        }
    }

    pub fn get_round_scores(&self) -> Vec<(String, i32)> {
        self.state
            .players
            .values()
            .map(|p| (p.name.clone(), p.round_score))
            .collect()
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
            GameAction::GetState => vec![GameResponse {
                recipients: Recipients::Single(context.sender_id),
                payload: ResponsePayload::StateChanged {
                    phase: self.state.phase,
                    question_type: self
                        .state
                        .current_question
                        .as_ref()
                        .map(|q| q.get_question_type().to_string())
                        .unwrap_or_default(),
                    alternatives: self.state.current_alternatives.clone(),
                    scoreboard: self.get_scoreboard(),
                    round_scores: self.get_round_scores(),
                },
            }],
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

        self.state
            .players
            .insert(ctx.sender_id, PlayerState::new(name.clone()));

        let current_players: Vec<(String, i32)> = self
            .state
            .players
            .values()
            .map(|p| (p.name.clone(), p.score))
            .collect();

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
                    alternatives: self.state.current_alternatives.clone(),
                    scoreboard: self.get_scoreboard(),
                    round_scores: self.get_round_scores(),
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

        let elapsed = match self.state.round_start_time {
            Some(start_time) => ctx.timestamp.duration_since(start_time),
            None => {
                warn!("Round start time should be set but was not. Using default duration");
                Duration::from_secs(self.state.round_duration / 2)
            }
        };

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

        let score_delta = if correct {
            ((self.state.round_duration as f64 * 100.0 - (elapsed.as_secs_f64() * 100.0)).max(0.0))
                as i32
        } else {
            0
        };

        if correct {
            player.score += score_delta;
        }
        player.round_score = score_delta;
        player.has_answered = true;
        player.answer = Some(answer);

        vec![GameResponse {
            recipients: Recipients::All,
            payload: ResponsePayload::PlayerAnswered {
                name: player.name.clone(),
                correct,
                new_score: player.score,
                round_score: player.round_score,
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
                round_scores: self.get_round_scores(),
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

        // Check if we're out of questions before trying to start a new round
        if self.state.current_question_index >= self.state.all_questions.len() {
            return vec![GameResponse {
                recipients: Recipients::Single(self.state.admin_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidAction,
                    message: "No more questions available. Please end the game.".into(),
                },
            }];
        }

        for player in self.state.players.values_mut() {
            player.has_answered = false;
            player.answer = None;
            player.round_score = 0;
        }

        match self.setup_round(specified_alternatives) {
            Ok(()) => {
                let Some(question) = self.state.current_question.as_ref() else {
                    return vec![GameResponse {
                        recipients: Recipients::Single(self.state.admin_id),
                        payload: ResponsePayload::Error {
                            code: ErrorCode::InvalidAction,
                            message: "Game in invalid state: question not set after setup".into(),
                        },
                    }];
                };

                self.state.phase = GamePhase::Question;
                self.state.round_start_time = Some(ctx.timestamp);

                let mut outputs = vec![GameResponse {
                    recipients: Recipients::All,
                    payload: ResponsePayload::StateChanged {
                        phase: GamePhase::Question,
                        question_type: question.get_question_type().to_string(),
                        alternatives: self.state.current_alternatives.clone(),
                        scoreboard: self.get_scoreboard(),
                        round_scores: self.get_round_scores(),
                    },
                }];

                outputs.push(GameResponse {
                    recipients: Recipients::Single(self.state.admin_id),
                    payload: ResponsePayload::AdminInfo {
                        current_question: question.clone(),
                    },
                });

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
        if self.state.phase != GamePhase::Question {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidPhase,
                    message: "Can only end round from question phase".into(),
                },
            }];
        }

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
                round_scores: self.get_round_scores(),
            },
        }];

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

        self.state.current_question = None;
        self.state.current_question_index += 1;
        self.state.current_alternatives.clear();
        self.state.correct_answers = None;

        let mut responses = Vec::new();

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

    pub fn get_scoreboard(&self) -> Vec<(String, i32)> {
        self.state
            .players
            .values()
            .map(|p| (p.name.clone(), p.score))
            .collect()
    }

    pub fn get_phase(&self) -> GamePhase {
        self.state.phase
    }

    fn setup_round(&mut self, specified_alternatives: Option<Vec<String>>) -> Result<(), String> {
        if self.state.current_question_index >= self.state.shuffled_question_indices.len() {
            return Err("No more questions available".to_string());
        }

        let shuffled_idx = self.state.shuffled_question_indices[self.state.current_question_index];
        let next_question = &self.state.all_questions[shuffled_idx];

        self.state.current_question = Some(next_question.clone());
        self.state.correct_answers = Some(next_question.get_correct_answer());

        if let Some(alts) = specified_alternatives {
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
        Ok(())
    }

    fn get_upcoming_questions(&self, count: usize) -> Vec<GameQuestion> {
        if self.state.current_question_index >= self.state.shuffled_question_indices.len() {
            return Vec::new(); // Return empty vec if we're at or past the end
        }

        let start = self.state.current_question_index;
        let end = std::cmp::min(start + count, self.state.shuffled_question_indices.len());

        self.state.shuffled_question_indices[start..end]
            .iter()
            .map(|&idx| self.state.all_questions[idx].clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
