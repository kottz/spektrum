use crate::db::QuestionSet;
use crate::question::GameQuestion;
use dashmap::DashMap;
use rand::seq::SliceRandom;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;
use tracing::warn;
use uuid::Uuid;

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

//
// New unified update types and packet wrapper
//

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum GamePhase {
    Lobby,
    Score,
    Question,
    GameOver,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminExtraInfo {
    pub upcoming_questions: Vec<GameQuestion>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub enum GameUpdate {
    /// A lightweight acknowledgement of connection.
    Connected {
        player_id: Uuid,
        lobby_id: Uuid,
        name: String,
        round_duration: u64,
    },
    /// A partial (delta) state update.
    StateDelta {
        phase: Option<GamePhase>,
        question_type: Option<String>,
        alternatives: Option<Vec<String>>,
        scoreboard: Option<Vec<(String, i32)>>,
        round_scores: Option<Vec<(String, i32)>>,
        // Optional extra info for admin
        admin_extra: Option<AdminExtraInfo>,
    },
    PlayerLeft {
        name: String,
    },
    Answered {
        name: String,
        correct: bool,
        new_score: i32,
        round_score: i32,
    },
    GameOver {
        final_scores: Vec<(String, i32)>,
        reason: String,
    },
    GameClosed {
        reason: String,
    },
    Error {
        message: String,
    },
    AdminInfo {
        current_question: GameQuestion,
    },
    AdminNextQuestions {
        upcoming_questions: Vec<GameQuestion>,
    },
}

#[derive(Clone, Debug, Serialize)]
pub enum Recipients {
    Single(Uuid),
    Multiple(Vec<Uuid>),
    AllExcept(Vec<Uuid>),
    All,
}

#[derive(Clone, Debug, Serialize)]
pub struct GameUpdatePacket {
    pub recipients: Recipients,
    pub update: GameUpdate,
}

//
// Original types for events, actions, and context remain largely unchanged
//

#[derive(Clone, Debug)]
pub struct EventContext {
    pub lobby_id: Uuid,
    pub sender_id: Uuid,
    pub timestamp: Instant,
}

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
pub struct GameEvent {
    pub context: EventContext,
    pub action: GameAction,
}

//
// We no longer use a separate ResponsePayload and GameResponse type.
// Instead, the engine directly pushes GameUpdatePackets.
//

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug, Serialize)]
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

//
// The new GameEngine – it holds the game state and a sender for updates.
// Instead of returning vectors of responses, it pushes updates into the channel.
//
pub struct GameEngine {
    state: GameState,
    connections: Arc<DashMap<Uuid, UnboundedSender<GameUpdate>>>,
}

impl GameEngine {
    pub fn new(
        admin_id: Uuid,
        questions: Arc<Vec<GameQuestion>>,
        set: Option<&QuestionSet>,
        round_duration: u64,
        connections: Arc<DashMap<Uuid, UnboundedSender<GameUpdate>>>,
    ) -> Self {
        let mut rng = rand::thread_rng();
        let indices = match set {
            None => {
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
            connections,
        }
    }

    pub fn get_round_scores(&self) -> Vec<(String, i32)> {
        self.state
            .players
            .values()
            .map(|p| (p.name.clone(), p.round_score))
            .collect()
    }

    fn get_scoreboard(&self) -> Vec<(String, i32)> {
        self.state
            .players
            .values()
            .map(|p| (p.name.clone(), p.score))
            .collect()
    }

    fn push_update(&self, recipients: Recipients, update: GameUpdate) {
        match recipients {
            Recipients::Single(target) => {
                if self.state.players.contains_key(&target) {
                    if let Some(sender) = self.connections.get(&target) {
                        let _ = sender.send(update);
                    }
                }
            }
            Recipients::Multiple(targets) => {
                for target in targets {
                    if self.state.players.contains_key(&target) {
                        if let Some(sender) = self.connections.get(&target) {
                            let _ = sender.send(update.clone());
                        }
                    }
                }
            }
            Recipients::AllExcept(exclusions) => {
                for player_id in self.state.players.keys() {
                    if !exclusions.contains(player_id) {
                        if let Some(sender) = self.connections.get(player_id) {
                            let _ = sender.send(update.clone());
                        }
                    }
                }
            }
            Recipients::All => {
                for player_id in self.state.players.keys() {
                    if let Some(sender) = self.connections.get(player_id) {
                        let _ = sender.send(update.clone());
                    }
                }
            }
        }
    }

    /// Convenience: push a state delta update to a single recipient.
    fn push_state(&self, recipient: Uuid) {
        let update = GameUpdate::StateDelta {
            phase: Some(self.state.phase),
            question_type: self
                .state
                .current_question
                .as_ref()
                .map(|q| q.get_question_type().to_string()),
            alternatives: Some(self.state.current_alternatives.clone()),
            scoreboard: Some(self.get_scoreboard()),
            round_scores: Some(self.get_round_scores()),
            admin_extra: None,
        };
        self.push_update(Recipients::Single(recipient), update);
    }

    /// Process an event from a client and push one or more updates.
    pub fn process_event(&mut self, event: GameEvent) {
        // Check admin-only actions:
        match &event.action {
            GameAction::StartGame
            | GameAction::StartRound { .. }
            | GameAction::EndRound
            | GameAction::EndGame { .. }
            | GameAction::CloseGame { .. } => {
                if event.context.sender_id != self.state.admin_id {
                    self.push_update(
                        Recipients::Single(event.context.sender_id),
                        GameUpdate::Error {
                            message: "Admin action requires authorization".into(),
                        },
                    );
                    return;
                }
            }
            _ => {}
        }

        // Process events and push updates accordingly.
        match event.action {
            GameAction::Join { name } => self.handle_join(event.context, name),
            GameAction::Leave => self.handle_leave(event.context),
            GameAction::Answer { answer } => self.handle_answer(event.context, answer),
            GameAction::StartGame => self.handle_start_game(event.context),
            GameAction::StartRound {
                specified_alternatives,
            } => self.handle_start_round(event.context, specified_alternatives),
            GameAction::GetState => self.push_state(event.context.sender_id),
            GameAction::EndRound => self.handle_end_round(event.context),
            GameAction::SkipQuestion => self.handle_skip_question(event.context),
            GameAction::EndGame { reason } => self.handle_end_game(event.context, reason),
            GameAction::CloseGame { reason } => self.handle_close_game(event.context, reason),
        }
    }

    fn handle_join(&mut self, ctx: EventContext, name: String) {
        if self.state.admin_id == ctx.sender_id {
            // Assume admin is already connected.
            return;
        }

        let name = name.trim().to_string();
        let existing_names = self.state.players.values().map(|p| &p.name);
        if let Err(validation_error) = crate::game::validate_player_name(&name, existing_names) {
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    message: validation_error.to_message(),
                },
            );
            return;
        }
        if self.state.phase != GamePhase::Lobby {
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    message: "You can't join a game that has already started.".into(),
                },
            );
            return;
        }
        self.state
            .players
            .insert(ctx.sender_id, PlayerState::new(name.clone()));

        // Notify the joining player that they are connected.
        self.push_update(
            Recipients::Single(ctx.sender_id),
            GameUpdate::Connected {
                player_id: ctx.sender_id,
                lobby_id: ctx.lobby_id,
                name: name.clone(),
                round_duration: self.state.round_duration,
            },
        );

        // Broadcast a state update (delta) to all others.
        self.push_update(
            Recipients::AllExcept(vec![ctx.sender_id]),
            GameUpdate::StateDelta {
                phase: Some(self.state.phase),
                question_type: None,
                alternatives: None,
                scoreboard: Some(self.get_scoreboard()),
                round_scores: Some(self.get_round_scores()),
                admin_extra: None,
            },
        );
    }

    fn handle_leave(&mut self, ctx: EventContext) {
        if let Some(player) = self.state.players.remove(&ctx.sender_id) {
            if ctx.sender_id == self.state.admin_id {
                self.push_update(
                    Recipients::All,
                    GameUpdate::GameClosed {
                        reason: "Host left the game".into(),
                    },
                );
            } else {
                self.push_update(
                    Recipients::All,
                    GameUpdate::PlayerLeft { name: player.name },
                );
            }
        } else {
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    message: "Player not found".into(),
                },
            );
        }
    }

    fn handle_answer(&mut self, ctx: EventContext, answer: String) {
        if self.state.phase != GamePhase::Question {
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    message: "Not in Question phase".into(),
                },
            );
            return;
        }

        let (player_name, new_score, round_score, correct) = {
            let player = match self.state.players.get_mut(&ctx.sender_id) {
                Some(p) => p,
                None => {
                    self.push_update(
                        Recipients::Single(ctx.sender_id),
                        GameUpdate::Error {
                            message: "Player not found".into(),
                        },
                    );
                    return;
                }
            };

            if player.has_answered {
                self.push_update(
                    Recipients::Single(ctx.sender_id),
                    GameUpdate::Error {
                        message: "Already answered this round".into(),
                    },
                );
                return;
            }

            let elapsed = match self.state.round_start_time {
                Some(start_time) => ctx.timestamp.duration_since(start_time),
                None => {
                    warn!("Round start time not set; using default duration");
                    Duration::from_secs(self.state.round_duration / 2)
                }
            };

            if elapsed.as_secs() > self.state.round_duration {
                self.push_update(
                    Recipients::Single(ctx.sender_id),
                    GameUpdate::Error {
                        message: "Time expired for this round".into(),
                    },
                );
                return;
            }

            let answer_clone = answer.clone();
            let correct = self
                .state
                .correct_answers
                .as_ref()
                .map_or(false, |answers| answers.contains(&answer_clone));
            let score_delta = if correct {
                ((self.state.round_duration as f64 * 100.0 - (elapsed.as_secs_f64() * 100.0))
                    .max(0.0)) as i32
            } else {
                0
            };

            if correct {
                player.score += score_delta;
            }
            player.round_score = score_delta;
            player.has_answered = true;
            player.answer = Some(answer_clone.clone());

            (
                player.name.clone(),
                player.score,
                player.round_score,
                correct,
            )
        };

        self.push_update(
            Recipients::All,
            GameUpdate::Answered {
                name: player_name,
                correct,
                new_score,
                round_score,
            },
        );
    }

    fn handle_start_game(&mut self, ctx: EventContext) {
        if self.state.phase != GamePhase::Lobby {
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    message: "Can only start game from lobby".into(),
                },
            );
            return;
        }
        self.state.phase = GamePhase::Score;
        self.push_update(
            Recipients::All,
            GameUpdate::StateDelta {
                phase: Some(GamePhase::Score),
                question_type: None,
                alternatives: None,
                scoreboard: Some(self.get_scoreboard()),
                round_scores: Some(self.get_round_scores()),
                admin_extra: None,
            },
        );
        let upcoming = self.get_upcoming_questions(3);
        if !upcoming.is_empty() {
            self.push_update(
                Recipients::Single(self.state.admin_id),
                GameUpdate::AdminNextQuestions {
                    upcoming_questions: upcoming,
                },
            );
        }
    }

    fn handle_start_round(
        &mut self,
        ctx: EventContext,
        specified_alternatives: Option<Vec<String>>,
    ) {
        if self.state.phase != GamePhase::Score {
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    message: "Can only start round from score phase".into(),
                },
            );
            return;
        }
        if self.state.current_question_index >= self.state.all_questions.len() {
            self.push_update(
                Recipients::Single(self.state.admin_id),
                GameUpdate::Error {
                    message: "No more questions available. Please end the game.".into(),
                },
            );
            return;
        }
        // Reset all players for the new round.
        for player in self.state.players.values_mut() {
            player.has_answered = false;
            player.answer = None;
            player.round_score = 0;
        }
        match self.setup_round(specified_alternatives) {
            Ok(()) => {
                let question = match &self.state.current_question {
                    Some(q) => q,
                    None => {
                        self.push_update(
                            Recipients::Single(self.state.admin_id),
                            GameUpdate::Error {
                                message: "Invalid state: question not set".into(),
                            },
                        );
                        return;
                    }
                };
                self.state.phase = GamePhase::Question;
                self.state.round_start_time = Some(ctx.timestamp);
                self.push_update(
                    Recipients::All,
                    GameUpdate::StateDelta {
                        phase: Some(GamePhase::Question),
                        question_type: Some(question.get_question_type().to_string()),
                        alternatives: Some(self.state.current_alternatives.clone()),
                        scoreboard: Some(self.get_scoreboard()),
                        round_scores: Some(self.get_round_scores()),
                        admin_extra: None,
                    },
                );
                self.push_update(
                    Recipients::Single(self.state.admin_id),
                    GameUpdate::AdminInfo {
                        current_question: question.clone(),
                    },
                );
            }
            Err(msg) => {
                self.push_update(
                    Recipients::Single(ctx.sender_id),
                    GameUpdate::Error { message: msg },
                );
            }
        }
    }

    fn handle_end_round(&mut self, ctx: EventContext) {
        if self.state.phase != GamePhase::Question {
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    message: "Can only end round from question phase".into(),
                },
            );
            return;
        }
        self.state.current_question = None;
        self.state.current_question_index += 1;
        self.state.current_alternatives.clear();
        self.state.correct_answers = None;
        self.state.phase = GamePhase::Score;
        self.push_update(
            Recipients::All,
            GameUpdate::StateDelta {
                phase: Some(GamePhase::Score),
                question_type: None,
                alternatives: None,
                scoreboard: Some(self.get_scoreboard()),
                round_scores: Some(self.get_round_scores()),
                admin_extra: None,
            },
        );
        let upcoming = self.get_upcoming_questions(3);
        if !upcoming.is_empty() {
            self.push_update(
                Recipients::Single(self.state.admin_id),
                GameUpdate::AdminNextQuestions {
                    upcoming_questions: upcoming,
                },
            );
        }
    }

    fn handle_skip_question(&mut self, ctx: EventContext) {
        if self.state.phase != GamePhase::Score {
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    message: "Can only skip question during score phase".into(),
                },
            );
            return;
        }
        self.state.current_question = None;
        self.state.current_question_index += 1;
        self.state.current_alternatives.clear();
        self.state.correct_answers = None;
        let upcoming = self.get_upcoming_questions(3);
        if !upcoming.is_empty() {
            self.push_update(
                Recipients::Single(self.state.admin_id),
                GameUpdate::AdminNextQuestions {
                    upcoming_questions: upcoming,
                },
            );
        }
    }

    fn handle_end_game(&mut self, _ctx: EventContext, reason: String) {
        self.state.phase = GamePhase::GameOver;
        self.push_update(
            Recipients::All,
            GameUpdate::GameOver {
                final_scores: self.get_scoreboard(),
                reason,
            },
        );
    }

    fn handle_close_game(&mut self, _ctx: EventContext, reason: String) {
        self.push_update(Recipients::All, GameUpdate::GameClosed { reason });
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
            return Vec::new();
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
        assert!(validate_player_name("John", empty_names.clone()).is_ok());
        assert!(validate_player_name("Player_1", empty_names.clone()).is_ok());
        assert!(validate_player_name("Cool-Name.123", empty_names.clone()).is_ok());
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
        let existing_names = vec!["John".to_string()];
        assert!(matches!(
            validate_player_name("John", existing_names.iter()),
            Err(NameValidationError::AlreadyTaken)
        ));
    }
}
