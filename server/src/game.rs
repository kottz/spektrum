use crate::db::QuestionSet;
use crate::question::GameQuestion;
use crate::server::Connection;
use crate::uuid::Uuid;
use dashmap::DashMap;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::warn;

lazy_static! {
    pub(crate) static ref NAME_VALIDATION_REGEX: Regex =
        Regex::new(r"^[a-zA-Z0-9_\-\. ]+$").expect("Failed to compile player name regex");
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

impl std::fmt::Display for NameValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_message())
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

    if !NAME_VALIDATION_REGEX.is_match(name) {
        return Err(NameValidationError::InvalidCharacters);
    }

    if existing_names.any(|existing_name| existing_name == name) {
        return Err(NameValidationError::AlreadyTaken);
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GamePhase {
    Lobby,
    Score,
    Question,
    GameOver,
    GameClosed,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AdminExtraInfo {
    pub upcoming_questions: Vec<GameQuestion>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "type")]
pub enum GameUpdate {
    /// A lightweight acknowledgement of connection.
    Connected {
        player_id: Uuid,
        name: String,
        round_duration: u64,
    },
    /// A partial (delta) state update.
    StateDelta {
        phase: Option<GamePhase>,
        question_type: Option<String>,
        question_text: Option<String>,
        alternatives: Option<Vec<String>>,
        scoreboard: Option<Vec<(String, i32)>>,
        round_scores: Option<Vec<(String, i32)>>,
        consecutive_misses: Option<Vec<(String, u32)>>,
        // Optional extra info for admin
        admin_extra: Option<AdminExtraInfo>,
    },
    PlayerLeft {
        name: String,
    },
    PlayerKicked {
        reason: String,
    },
    Answered {
        name: String,
        correct: bool,
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
    _AllExcept(Vec<Uuid>),
    All,
}

#[derive(Clone, Debug, Serialize)]
pub struct GameUpdatePacket {
    pub recipients: Recipients,
    pub update: GameUpdate,
}

#[derive(Clone, Debug)]
pub struct EventContext {
    pub sender_id: Uuid,
    pub timestamp: Instant,
}

#[derive(Clone, Debug)]
pub enum GameAction {
    Connect,
    Leave,
    Answer { answer: String },
    StartGame,
    StartRound,
    EndRound,
    SkipQuestion,
    KickPlayer { player_name: String },
    EndGame { reason: String },
    CloseGame { reason: String },
}

#[derive(Clone, Debug)]
pub struct GameEvent {
    pub context: EventContext,
    pub action: GameAction,
}

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
    pub last_lobby_message: Option<Instant>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PlayerState {
    pub name: String,
    pub score: i32,
    pub round_score: i32,
    pub has_answered: bool,
    pub answer: Option<String>,
    pub consecutive_misses: u32,
}

impl PlayerState {
    pub fn new(name: String) -> Self {
        Self {
            name,
            score: 0,
            round_score: 0,
            has_answered: false,
            answer: None,
            consecutive_misses: 0,
        }
    }
}

pub struct GameEngine {
    state: GameState,
    connections: Arc<DashMap<Uuid, Connection>>,
}

impl GameEngine {
    pub fn new(
        admin_id: Uuid,
        questions: Arc<Vec<GameQuestion>>,
        set: Option<&QuestionSet>,
        round_duration: u64,
        connections: Arc<DashMap<Uuid, Connection>>,
    ) -> Self {
        let indices = match set {
            None => {
                let mut all_indices: Vec<usize> = (0..questions.len()).collect();
                fastrand::shuffle(&mut all_indices);
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
                fastrand::shuffle(&mut set_indices);
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
                last_lobby_message: Some(Instant::now()),
            },
            connections,
        }
    }

    pub fn add_player(&mut self, player_id: Uuid, name: String) -> Result<(), NameValidationError> {
        let trimmed = name.trim();
        let existing_names = self.state.players.values().map(|p| &p.name);
        validate_player_name(trimmed, existing_names)?;
        self.state
            .players
            .insert(player_id, PlayerState::new(trimmed.to_string()));
        Ok(())
    }

    pub fn last_update(&self) -> Option<Instant> {
        self.state.last_lobby_message
    }

    pub fn is_finished(&self) -> bool {
        if self.state.phase == GamePhase::GameClosed {
            return true;
        }
        if let Some(last_msg) = self.state.last_lobby_message {
            if Instant::now().duration_since(last_msg) > Duration::from_secs(3600) {
                self.push_update(
                    Recipients::All,
                    GameUpdate::GameClosed {
                        reason: "Lobby closed due to inactivity".into(),
                    },
                );
                return true;
            }
        }
        false
    }

    pub fn get_consecutive_misses(&self) -> Vec<(String, u32)> {
        self.state
            .players
            .values()
            .map(|p| (p.name.clone(), p.consecutive_misses))
            .collect()
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
            .iter()
            .filter(|(id, _)| **id != self.state.admin_id)
            .map(|(_, p)| (p.name.clone(), p.score))
            .collect()
    }

    fn push_update(&self, recipients: Recipients, update: GameUpdate) {
        match recipients {
            Recipients::Single(target) => {
                if self.state.players.contains_key(&target) {
                    if let Some(conn) = self.connections.get(&target) {
                        if let Some(ref tx) = conn.tx {
                            let _ = tx.send(update);
                        }
                    }
                }
            }
            Recipients::Multiple(targets) => {
                for target in targets {
                    if self.state.players.contains_key(&target) {
                        if let Some(conn) = self.connections.get(&target) {
                            if let Some(ref tx) = conn.tx {
                                let _ = tx.send(update.clone());
                            }
                        }
                    }
                }
            }
            Recipients::_AllExcept(exclusions) => {
                for player_id in self.state.players.keys() {
                    if !exclusions.contains(player_id) {
                        if let Some(conn) = self.connections.get(player_id) {
                            if let Some(ref tx) = conn.tx {
                                let _ = tx.send(update.clone());
                            }
                        }
                    }
                }
            }
            Recipients::All => {
                for player_id in self.state.players.keys() {
                    if let Some(conn) = self.connections.get(player_id) {
                        if let Some(ref tx) = conn.tx {
                            let _ = tx.send(update.clone());
                        }
                    }
                }
            }
        }
    }

    pub fn process_event(&mut self, event: GameEvent) {
        self.state.last_lobby_message = Some(Instant::now());
        // Check admin-only actions:
        match &event.action {
            GameAction::StartGame
            | GameAction::StartRound
            | GameAction::EndRound
            | GameAction::KickPlayer { .. }
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

        match event.action {
            GameAction::Connect => self.handle_connect(event.context),
            GameAction::Leave => self.handle_leave(event.context),
            GameAction::Answer { answer } => self.handle_answer(event.context, answer),
            GameAction::StartGame => self.handle_start_game(event.context),
            GameAction::StartRound => self.handle_start_round(event.context),
            GameAction::EndRound => self.handle_end_round(event.context),
            GameAction::SkipQuestion => self.handle_skip_question(event.context),
            GameAction::KickPlayer { player_name } => {
                self.handle_kick_player(event.context, player_name)
            }
            GameAction::EndGame { reason } => self.handle_end_game(event.context, reason),
            GameAction::CloseGame { reason } => self.handle_close_game(event.context, reason),
        }
    }

    fn handle_connect(&self, ctx: EventContext) {
        if let Some(player) = self.state.players.get(&ctx.sender_id) {
            // First send the initial connection acknowledgment
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Connected {
                    player_id: ctx.sender_id,
                    name: player.name.clone(),
                    round_duration: self.state.round_duration,
                },
            );

            // Then send the complete current state
            let state_update = GameUpdate::StateDelta {
                phase: Some(self.state.phase),
                question_type: self
                    .state
                    .current_question
                    .as_ref()
                    .map(|q| q.get_question_type().to_string()),
                question_text: self
                    .state
                    .current_question
                    .as_ref()
                    .and_then(|q| q.question_text.clone()),
                alternatives: Some(self.state.current_alternatives.clone()),
                scoreboard: Some(self.get_scoreboard()),
                round_scores: Some(self.get_round_scores()),
                consecutive_misses: Some(self.get_consecutive_misses()),
                admin_extra: if ctx.sender_id == self.state.admin_id {
                    Some(AdminExtraInfo {
                        upcoming_questions: self.get_upcoming_questions(3),
                    })
                } else {
                    None
                },
            };

            self.push_update(Recipients::Single(ctx.sender_id), state_update);

            // In lobby phase, broadcast scoreboard to all players
            if self.state.phase == GamePhase::Lobby {
                self.push_update(
                    Recipients::_AllExcept(vec![ctx.sender_id]),
                    GameUpdate::StateDelta {
                        phase: None,
                        question_type: None,
                        question_text: None,
                        alternatives: None,
                        scoreboard: Some(self.get_scoreboard()),
                        round_scores: None,
                        consecutive_misses: None,
                        admin_extra: None,
                    },
                );
            }

            // If this is the admin and we're in Question phase, send the current question
            if ctx.sender_id == self.state.admin_id
                && self.state.phase == GamePhase::Question
                && self.state.current_question.is_some()
            {
                if let Some(question) = &self.state.current_question {
                    self.push_update(
                        Recipients::Single(self.state.admin_id),
                        GameUpdate::AdminInfo {
                            current_question: question.clone(),
                        },
                    );
                }
            }
        } else {
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    message: "Player not found. Please register before connecting.".into(),
                },
            );
        }
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
        let (player_name, correct) = {
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
            let correct = self
                .state
                .correct_answers
                .as_ref()
                .is_some_and(|answers| answers.contains(&answer));
            let score_delta = if correct {
                ((5000.0 * (self.state.round_duration as f64 - elapsed.as_secs_f64())
                    / self.state.round_duration as f64)
                    .clamp(0.0, 5000.0)) as i32
            } else {
                0
            };
            if correct {
                player.score += score_delta;
            }
            player.round_score = score_delta;
            player.has_answered = true;
            player.answer = Some(answer);
            (player.name.clone(), correct)
        };
        self.push_update(
            Recipients::All,
            GameUpdate::Answered {
                name: player_name,
                correct,
            },
        );
    }

    fn handle_start_game(&mut self, ctx: EventContext) {
        if self.state.phase != GamePhase::Lobby && self.state.phase != GamePhase::GameOver {
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    message: "Can only start game from lobby or after a finished game".into(),
                },
            );
            return;
        }

        // if we came from a finished game, zero everything out
        if self.state.phase == GamePhase::GameOver {
            self.reset_for_new_game();
        }

        self.state.phase = GamePhase::Score;
        self.push_update(
            Recipients::All,
            GameUpdate::StateDelta {
                phase: Some(GamePhase::Score),
                question_type: None,
                question_text: None,
                alternatives: None,
                scoreboard: Some(self.get_scoreboard()),
                round_scores: Some(self.get_round_scores()),
                consecutive_misses: Some(self.get_consecutive_misses()),
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

    fn handle_start_round(&mut self, ctx: EventContext) {
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
        match self.setup_round() {
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
                        question_text: question.question_text.clone(),
                        alternatives: Some(self.state.current_alternatives.clone()),
                        scoreboard: Some(self.get_scoreboard()),
                        round_scores: Some(self.get_round_scores()),
                        consecutive_misses: Some(self.get_consecutive_misses()),
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

        // AFK check: if a player has not answered, increment their consecutive misses
        for player in self.state.players.values_mut() {
            if !player.has_answered {
                player.consecutive_misses += 1;
            } else {
                player.consecutive_misses = 0;
            }
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
                question_text: None,
                alternatives: None,
                scoreboard: Some(self.get_scoreboard()),
                round_scores: Some(self.get_round_scores()),
                consecutive_misses: Some(self.get_consecutive_misses()),
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

    fn handle_kick_player(&mut self, ctx: EventContext, target_player_name: String) {
        // Find the player ID based on the name
        let target_player_id = self
            .state
            .players
            .iter()
            .find(|(_, p)| p.name == target_player_name)
            .map(|(id, _)| *id);
        let target_player_id = match target_player_id {
            Some(id) => id,
            None => {
                self.push_update(
                    Recipients::Single(ctx.sender_id), // Send error to admin
                    GameUpdate::Error {
                        message: format!("Player '{}' not found.", target_player_name),
                    },
                );
                return;
            }
        };
        // Admin cannot kick themselves
        if target_player_id == self.state.admin_id {
            self.push_update(
                Recipients::Single(ctx.sender_id), // Send error to admin
                GameUpdate::Error {
                    message: "Admin cannot kick themselves.".into(),
                },
            );
            return;
        }

        // Get player name for notification before removing
        if let Some(kicked_player) = self.state.players.get(&target_player_id) {
            let kicked_player_name = kicked_player.name.clone();

            // Notify the kicked player specifically - BEFORE removing them
            self.push_update(
                Recipients::Single(target_player_id),
                GameUpdate::PlayerKicked {
                    reason: "Kicked by admin".to_string(),
                },
            );

            // Now we can safely remove the player
            self.state.players.remove(&target_player_id);

            // Remove the connection entry (important for cleanup and preventing re-connect issues)
            if self.connections.remove(&target_player_id).is_none() {
                warn!(
                    "Attempted to remove connection for kicked player {} but it was already gone.",
                    target_player_id
                );
            }

            // Notify remaining players
            let remaining_players: Vec<Uuid> = self.state.players.keys().cloned().collect();
            self.push_update(
                Recipients::Multiple(remaining_players.clone()), // Send to all *remaining* players
                GameUpdate::PlayerLeft {
                    name: kicked_player_name,
                },
            );

            // // Send updated scoreboard to remaining players
            self.push_update(
                Recipients::Multiple(remaining_players),
                GameUpdate::StateDelta {
                    phase: None, // Phase doesn't change
                    question_type: None,
                    question_text: None,
                    alternatives: None,
                    scoreboard: Some(self.get_scoreboard()), // Update scoreboard
                    round_scores: None, // Round scores might be irrelevant now, maybe send? Optional.
                    consecutive_misses: Some(self.get_consecutive_misses()),
                    admin_extra: None, // Admin already knows
                },
            );
        } else {
            // This case should theoretically not happen if find worked, but handle defensively
            self.push_update(
                Recipients::Single(ctx.sender_id), // Send error to admin
                GameUpdate::Error {
                    message: format!(
                        "Failed to remove player '{}' internally.",
                        target_player_name
                    ),
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

    fn handle_close_game(&self, _ctx: EventContext, reason: String) {
        self.push_update(Recipients::All, GameUpdate::GameClosed { reason });
    }

    fn setup_round(&mut self) -> Result<(), String> {
        if self.state.current_question_index >= self.state.shuffled_question_indices.len() {
            return Err("No more questions available".to_string());
        }
        let shuffled_idx = self.state.shuffled_question_indices[self.state.current_question_index];
        let next_question = &self.state.all_questions[shuffled_idx];
        self.state.current_question = Some(next_question.clone());
        self.state.correct_answers = Some(next_question.get_correct_answer());
        self.state.current_alternatives = next_question.generate_round_alternatives();
        if let Some(correct_answers) = self.state.correct_answers.clone() {
            for answer in correct_answers {
                if !self.state.current_alternatives.contains(&answer) {
                    self.state.current_alternatives.push(answer);
                }
            }
        }
        self.state.current_alternatives.dedup();
        fastrand::shuffle(&mut self.state.current_alternatives);
        Ok(())
    }

    fn reset_for_new_game(&mut self) {
        // scramble the questions again
        self.state.shuffled_question_indices = {
            let mut v: Vec<usize> = (0..self.state.all_questions.len()).collect();
            fastrand::shuffle(&mut v);
            v
        };
        self.state.current_question_index = 0;
        self.state.current_question = None;
        self.state.current_alternatives.clear();
        self.state.correct_answers = None;

        // wipe every player’s scoreboard
        for p in self.state.players.values_mut() {
            p.score = 0;
            p.round_score = 0;
            p.has_answered = false;
            p.answer = None;
        }
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
    use crate::question::{Color, GameQuestion, GameQuestionOption, QuestionType};
    use std::time::Duration;

    fn create_test_questions() -> Vec<GameQuestion> {
        vec![
            // Color question
            GameQuestion {
                id: 1,
                question_type: QuestionType::Color,
                question_text: None,
                title: "What color is predominantly used in this video?".to_string(),
                artist: Some("Test Artist".to_string()),
                youtube_id: "test123".to_string(),
                options: vec![
                    GameQuestionOption {
                        option: "Red".to_string(),
                        is_correct: true,
                    },
                    GameQuestionOption {
                        option: "Blue".to_string(),
                        is_correct: false,
                    },
                ],
            },
            // Text question
            GameQuestion {
                id: 2,
                question_type: QuestionType::Text,
                question_text: None,
                title: "What is the main theme of this video?".to_string(),
                artist: Some("Test Artist".to_string()),
                youtube_id: "test456".to_string(),
                options: vec![
                    GameQuestionOption {
                        option: "Love".to_string(),
                        is_correct: true,
                    },
                    GameQuestionOption {
                        option: "War".to_string(),
                        is_correct: false,
                    },
                    GameQuestionOption {
                        option: "Peace".to_string(),
                        is_correct: false,
                    },
                ],
            },
            // Year question
            GameQuestion {
                id: 3,
                question_type: QuestionType::Year,
                question_text: None,
                title: "When was this video released?".to_string(),
                artist: Some("Test Artist".to_string()),
                youtube_id: "test789".to_string(),
                options: vec![GameQuestionOption {
                    option: "2020".to_string(),
                    is_correct: true,
                }],
            },
        ]
    }

    fn setup_test_game() -> (GameEngine, Uuid) {
        let admin_id = Uuid::new_v4();
        let questions = Arc::new(create_test_questions());
        let connections = Arc::new(DashMap::new());

        // Create a connection for admin
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        connections.insert(
            admin_id,
            Connection {
                lobby_id: Uuid::new_v4(),
                tx: Some(tx),
            },
        );

        (
            GameEngine::new(admin_id, questions, None, 30, connections),
            admin_id,
        )
    }

    fn add_test_player(engine: &mut GameEngine, name: &str) -> Uuid {
        let player_id = Uuid::new_v4();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        engine.connections.insert(
            player_id,
            Connection {
                lobby_id: engine
                    .connections
                    .get(&engine.state.admin_id)
                    .unwrap()
                    .lobby_id,
                tx: Some(tx),
            },
        );
        engine.add_player(player_id, name.to_string()).unwrap();
        player_id
    }

    #[test]
    fn test_full_game_flow() {
        let (mut engine, admin_id) = setup_test_game();
        let player1_id = add_test_player(&mut engine, "Player1");
        let player2_id = add_test_player(&mut engine, "Player2");

        let now = Instant::now();

        // Start game
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: now,
            },
            action: GameAction::StartGame,
        });
        assert_eq!(engine.state.phase, GamePhase::Score);

        // Play through all questions
        for _ in 0..create_test_questions().len() {
            // Start round
            engine.process_event(GameEvent {
                context: EventContext {
                    sender_id: admin_id,
                    timestamp: now,
                },
                action: GameAction::StartRound,
            });
            assert_eq!(engine.state.phase, GamePhase::Question);

            // Players answer
            let correct_answer = engine.state.correct_answers.as_ref().unwrap()[0].clone();

            // Player 1 answers correctly quickly
            engine.process_event(GameEvent {
                context: EventContext {
                    sender_id: player1_id,
                    timestamp: now + Duration::from_secs(1),
                },
                action: GameAction::Answer {
                    answer: correct_answer.clone(),
                },
            });

            // Player 2 answers correctly but slower
            engine.process_event(GameEvent {
                context: EventContext {
                    sender_id: player2_id,
                    timestamp: now + Duration::from_secs(5),
                },
                action: GameAction::Answer {
                    answer: correct_answer,
                },
            });

            // Verify both players answered
            assert!(engine.state.players[&player1_id].has_answered);
            assert!(engine.state.players[&player2_id].has_answered);

            // Verify Player 1 got more points (answered faster)
            assert!(
                engine.state.players[&player1_id].round_score
                    > engine.state.players[&player2_id].round_score
            );

            // End round
            engine.process_event(GameEvent {
                context: EventContext {
                    sender_id: admin_id,
                    timestamp: now,
                },
                action: GameAction::EndRound,
            });
            assert_eq!(engine.state.phase, GamePhase::Score);
        }

        // End game
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: now,
            },
            action: GameAction::EndGame {
                reason: "Game completed".to_string(),
            },
        });
        assert_eq!(engine.state.phase, GamePhase::GameOver);
        engine.connections.clear();
    }

    #[test]
    fn test_question_types() {
        let (mut engine, admin_id) = setup_test_game();
        add_test_player(&mut engine, "Player1");

        // Start game
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartGame,
        });

        // Test each question type
        for _ in 0..create_test_questions().len() {
            engine.process_event(GameEvent {
                context: EventContext {
                    sender_id: admin_id,
                    timestamp: Instant::now(),
                },
                action: GameAction::StartRound,
            });

            // Verify alternatives are generated correctly
            match engine
                .state
                .current_question
                .as_ref()
                .unwrap()
                .question_type
            {
                QuestionType::Color => {
                    assert_eq!(engine.state.current_alternatives.len(), 6);
                    assert!(engine
                        .state
                        .current_alternatives
                        .iter()
                        .all(|c| c.parse::<Color>().is_ok()));
                }
                QuestionType::Year => {
                    assert_eq!(engine.state.current_alternatives.len(), 5);
                    assert!(engine
                        .state
                        .current_alternatives
                        .iter()
                        .all(|y| y.parse::<i32>().is_ok()));
                }
                _ => {
                    assert!(!engine.state.current_alternatives.is_empty());
                }
            }

            engine.process_event(GameEvent {
                context: EventContext {
                    sender_id: admin_id,
                    timestamp: Instant::now(),
                },
                action: GameAction::EndRound,
            });
        }
        engine.connections.clear();
    }

    #[test]
    fn test_error_conditions() {
        let (mut engine, admin_id) = setup_test_game();
        let player_id = add_test_player(&mut engine, "Player1");

        // Test answering before game starts
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Answer {
                answer: "test".to_string(),
            },
        });
        assert!(!engine.state.players[&player_id].has_answered);

        // Start game properly
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartGame,
        });
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartRound,
        });

        // Test double answer
        let answer = engine.state.current_alternatives[0].clone();
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Answer {
                answer: answer.clone(),
            },
        });
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Answer { answer },
        });

        // Test late answer
        let late_player_id = add_test_player(&mut engine, "LatePlayer");
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: late_player_id,
                timestamp: Instant::now() + Duration::from_secs(31),
            },
            action: GameAction::Answer {
                answer: "test".to_string(),
            },
        });
        assert!(!engine.state.players[&late_player_id].has_answered);
        engine.connections.clear();
    }

    #[test]
    fn test_admin_features() {
        let (mut engine, admin_id) = setup_test_game();
        let player_id = add_test_player(&mut engine, "Player1");

        // Test player can't start game
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartGame,
        });
        assert_eq!(engine.state.phase, GamePhase::Lobby);

        // Test admin can start game
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartGame,
        });
        assert_eq!(engine.state.phase, GamePhase::Score);

        // Verify upcoming questions are available
        let upcoming = engine.get_upcoming_questions(3);
        assert!(!upcoming.is_empty());
        assert!(upcoming.len() <= 3);
        engine.connections.clear();
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::CloseGame {
                reason: "Test close".to_string(),
            },
        });
        engine.connections.clear();
    }

    #[tokio::test]
    async fn test_player_reconnect_lobby() {
        let (mut engine, _admin_id) = setup_test_game();
        let player_id = add_test_player(&mut engine, "Player1");

        //do not start the game, stay in lobby
        let player_initial_state = engine.state.players.get(&player_id).unwrap().clone();

        // Simulate disconnection (remove the connection)
        engine.connections.remove(&player_id);

        // Re-add the player with a new connection
        let (player_tx, mut player_rx) = tokio::sync::mpsc::unbounded_channel();
        engine.connections.insert(
            player_id,
            Connection {
                lobby_id: engine
                    .connections
                    .get(&engine.state.admin_id)
                    .unwrap()
                    .lobby_id,
                tx: Some(player_tx),
            },
        );

        // Reconnect
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Connect,
        });

        // Verify Connected message
        match player_rx.recv().await.unwrap() {
            GameUpdate::Connected { player_id: pid, .. } => {
                assert_eq!(pid, player_id, "Correct player ID in Connected");
            }
            other => panic!("Expected Connected, got {:?}", other),
        }

        // Verify StateDelta message
        match player_rx.recv().await.unwrap() {
            GameUpdate::StateDelta {
                phase,
                scoreboard,
                admin_extra,
                ..
            } => {
                assert_eq!(phase, Some(GamePhase::Lobby), "Correct phase");
                assert!(scoreboard.is_some(), "Scoreboard should be present");
                assert!(admin_extra.is_none(), "Admin extra should not be present");

                // Additional checks for player state (important!)
                let player_state = engine.state.players.get(&player_id).unwrap();
                assert_eq!(
                    player_state.has_answered, player_initial_state.has_answered,
                    "has_answered should be preserved"
                );
                assert_eq!(
                    player_state.score, player_initial_state.score,
                    "Score should be preserved"
                );
                assert_eq!(
                    player_state.round_score, player_initial_state.round_score,
                    "Round score should be preserved"
                );
                assert_eq!(
                    player_state.answer, player_initial_state.answer,
                    "Answer should be preserved"
                );

                if let Ok(GameUpdate::StateDelta { scoreboard: sb, .. }) = player_rx.try_recv() {
                    assert!(sb.is_some(), "Scoreboard should be present in rebroadcast");
                }
            }
            other => panic!("Expected StateDelta, got {:?}", other),
        }

        engine.connections.clear();
        player_rx.close();
    }

    #[tokio::test]
    async fn test_player_reconnect_score() {
        let (mut engine, admin_id) = setup_test_game();
        let player_id = add_test_player(&mut engine, "Player1");

        // Start the game and a round
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartGame,
        });

        let player_initial_state = engine.state.players.get(&player_id).unwrap().clone();

        // Simulate disconnection (remove the connection)
        engine.connections.remove(&player_id);

        // Re-add the player with a new connection
        let (player_tx, mut player_rx) = tokio::sync::mpsc::unbounded_channel();
        engine.connections.insert(
            player_id,
            Connection {
                lobby_id: engine
                    .connections
                    .get(&engine.state.admin_id)
                    .unwrap()
                    .lobby_id,
                tx: Some(player_tx),
            },
        );

        // Reconnect
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Connect,
        });

        // Verify Connected message
        match player_rx.recv().await.unwrap() {
            GameUpdate::Connected { player_id: pid, .. } => {
                assert_eq!(pid, player_id, "Correct player ID in Connected");
            }
            other => panic!("Expected Connected, got {:?}", other),
        }

        // Verify StateDelta message
        match player_rx.recv().await.unwrap() {
            GameUpdate::StateDelta {
                phase,
                scoreboard,
                admin_extra,
                ..
            } => {
                assert_eq!(phase, Some(GamePhase::Score), "Correct phase");
                assert!(scoreboard.is_some(), "Scoreboard should be present");
                assert!(admin_extra.is_none(), "Admin extra should not be present");

                // Additional checks for player state (important!)
                let player_state = engine.state.players.get(&player_id).unwrap();
                assert_eq!(
                    player_state.has_answered, player_initial_state.has_answered,
                    "has_answered should be preserved"
                );
                assert_eq!(
                    player_state.score, player_initial_state.score,
                    "Score should be preserved"
                );
                assert_eq!(
                    player_state.round_score, player_initial_state.round_score,
                    "Round score should be preserved"
                );
                assert_eq!(
                    player_state.answer, player_initial_state.answer,
                    "Answer should be preserved"
                );
            }
            other => panic!("Expected StateDelta, got {:?}", other),
        }

        engine.connections.clear();
        player_rx.close();
    }

    #[test]
    fn test_name_validation_errors() {
        let empty_names = std::iter::empty();

        // Test too short name
        assert!(matches!(
            validate_player_name("a", empty_names.clone()),
            Err(NameValidationError::TooShort)
        ));

        // Test too long name
        assert!(matches!(
            validate_player_name(&"a".repeat(17), empty_names.clone()),
            Err(NameValidationError::TooLong)
        ));

        // Test invalid characters
        assert!(matches!(
            validate_player_name("Invalid@Name", empty_names.clone()),
            Err(NameValidationError::InvalidCharacters)
        ));

        // Test duplicate name
        let existing_names = ["TestName".to_string()];
        assert!(matches!(
            validate_player_name("TestName", existing_names.iter()),
            Err(NameValidationError::AlreadyTaken)
        ));
    }

    #[test]
    fn test_name_validation_error_messages() {
        // Test all variants of NameValidationError and their messages
        let errors = vec![
            (
                NameValidationError::TooShort,
                "Name must be at least 2 characters long.",
            ),
            (
                NameValidationError::TooLong,
                "Name cannot be longer than 16 characters.",
            ),
            (
                NameValidationError::InvalidCharacters,
                "Name can only contain letters, numbers, spaces, and the symbols: _ - .",
            ),
            (
                NameValidationError::AlreadyTaken,
                "This name is already taken.",
            ),
        ];

        // Test to_message() method
        for (error, expected_message) in errors.iter() {
            assert_eq!(error.to_message(), *expected_message);
        }

        // Test Display implementation
        for (error, expected_message) in errors {
            assert_eq!(format!("{}", error), expected_message);
        }
    }

    #[tokio::test]
    async fn test_connection_handling() {
        let (mut engine, _admin_id) = setup_test_game();
        let player_id = add_test_player(&mut engine, "Player1");

        // Test leave for regular player
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Leave,
        });
        assert!(!engine.state.players.contains_key(&player_id));

        // Test leave for non-existent player
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: Uuid::new_v4(),
                timestamp: Instant::now(),
            },
            action: GameAction::Leave,
        });
        engine.connections.clear();
    }

    #[test]
    fn test_game_state_transitions() {
        let (mut engine, admin_id) = setup_test_game();
        add_test_player(&mut engine, "Player1");

        // Test skip question
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartGame,
        });

        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::SkipQuestion,
        });

        // Test close game
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::CloseGame {
                reason: "Test close".to_string(),
            },
        });
        engine.connections.clear();
    }

    #[test]
    fn test_game_timeout() {
        let (mut engine, _) = setup_test_game();
        assert!(!engine.is_finished());

        // Set last message to more than an hour ago
        engine.state.last_lobby_message = Some(Instant::now() - Duration::from_secs(3601));
        assert!(engine.is_finished());

        // Test GameOver phase
        engine.state.phase = GamePhase::GameOver;
        assert!(engine.is_finished());

        // Test last_update
        assert_eq!(engine.last_update(), engine.state.last_lobby_message);
        engine.connections.clear();
    }

    #[test]
    fn test_question_set_handling() {
        let admin_id = Uuid::new_v4();
        let questions = Arc::new(create_test_questions());
        let connections = Arc::new(DashMap::new());

        let question_set = QuestionSet {
            id: 1,
            question_ids: vec![1, 2], // Only include first two questions
            name: "Test Set".to_string(),
        };

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        connections.insert(
            admin_id,
            Connection {
                lobby_id: Uuid::new_v4(),
                tx: Some(tx),
            },
        );

        let engine = GameEngine::new(admin_id, questions, Some(&question_set), 30, connections);

        assert_eq!(engine.state.shuffled_question_indices.len(), 2);
        engine.connections.clear();
    }

    #[tokio::test]
    async fn test_admin_reconnect() {
        let (mut engine, admin_id) = setup_test_game();

        // Add the admin to players
        engine.add_player(admin_id, "Admin".to_string()).unwrap();

        // Create channel to capture admin messages
        let (admin_tx, mut admin_rx) = tokio::sync::mpsc::unbounded_channel();
        engine.connections.insert(
            admin_id,
            Connection {
                lobby_id: Uuid::new_v4(),
                tx: Some(admin_tx),
            },
        );

        // First test: admin reconnects in lobby, should get upcoming questions
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Connect,
        });

        // Should receive Connected message first
        match admin_rx
            .try_recv()
            .expect("Should receive Connected message")
        {
            GameUpdate::Connected { player_id, .. } => {
                assert_eq!(player_id, admin_id);
            }
            other => panic!("Expected Connected message, got {:?}", other),
        }

        // Should receive StateDelta with admin_extra next
        match admin_rx
            .try_recv()
            .expect("Should receive StateDelta message")
        {
            GameUpdate::StateDelta { admin_extra, .. } => {
                assert!(admin_extra.is_some(), "Admin should receive admin_extra");
            }
            other => panic!("Expected StateDelta message, got {:?}", other),
        }

        // Second test: admin reconnects during Question phase
        engine.state.phase = GamePhase::Question;
        let test_question = engine.state.all_questions[0].clone();
        engine.state.current_question = Some(test_question.clone());

        // Create new channel for clean message capture
        let (admin_tx2, mut admin_rx2) = tokio::sync::mpsc::unbounded_channel();
        engine.connections.insert(
            admin_id,
            Connection {
                lobby_id: Uuid::new_v4(),
                tx: Some(admin_tx2),
            },
        );

        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Connect,
        });

        // Should receive Connected message first
        match admin_rx2
            .try_recv()
            .expect("Should receive Connected message")
        {
            GameUpdate::Connected { player_id, .. } => {
                assert_eq!(player_id, admin_id);
            }
            other => panic!("Expected Connected message, got {:?}", other),
        }

        // Should receive StateDelta next
        match admin_rx2
            .try_recv()
            .expect("Should receive StateDelta message")
        {
            GameUpdate::StateDelta { admin_extra, .. } => {
                assert!(admin_extra.is_some(), "Admin should receive admin_extra");
            }
            other => panic!("Expected StateDelta message, got {:?}", other),
        }

        // Should receive current question info
        match admin_rx2
            .try_recv()
            .expect("Should receive AdminInfo message")
        {
            GameUpdate::AdminInfo { current_question } => {
                assert_eq!(current_question.id, test_question.id);
            }
            other => panic!("Expected AdminInfo message, got {:?}", other),
        }
        engine.connections.clear();
        admin_rx2.close();
        admin_rx.close();
    }

    #[tokio::test]
    async fn test_admin_leave_message() {
        // Setup game with initial admin
        let admin_id = Uuid::new_v4();
        let admin_lobby_id = Uuid::new_v4();
        let connections = Arc::new(DashMap::new());
        let (admin_tx, _admin_rx) = tokio::sync::mpsc::unbounded_channel();

        // Add admin connection
        connections.insert(
            admin_id,
            Connection {
                lobby_id: admin_lobby_id,
                tx: Some(admin_tx),
            },
        );

        // Create game engine
        let mut engine = GameEngine::new(
            admin_id,
            Arc::new(create_test_questions()),
            None,
            30,
            connections,
        );

        // IMPORTANT: Add admin to players first
        engine.add_player(admin_id, "Admin".to_string()).unwrap();

        // Add a player
        let player_id = Uuid::new_v4();
        let (player_tx, mut player_rx) = tokio::sync::mpsc::unbounded_channel();

        // Add player connection with same lobby id as admin
        engine.connections.insert(
            player_id,
            Connection {
                lobby_id: admin_lobby_id,
                tx: Some(player_tx),
            },
        );

        // Add player to game
        engine
            .add_player(player_id, "TestPlayer".to_string())
            .unwrap();

        // Admin leaves
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Leave,
        });

        // Use tokio::spawn to handle message receiving in parallel
        let message = tokio::spawn(async move { player_rx.recv().await })
            .await
            .unwrap()
            .expect("Channel shouldn't be closed");

        // Verify the message
        match message {
            GameUpdate::GameClosed { reason } => {
                assert_eq!(reason, "Host left the game");
            }
            other => panic!("Expected GameClosed message, got {:?}", other),
        }

        // State verifications
        assert!(!engine.state.players.contains_key(&admin_id));
        engine.connections.clear();
    }

    #[test]
    fn test_error_paths() {
        let (mut engine, admin_id) = setup_test_game();

        // Test starting round in wrong phase
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartRound,
        });
        assert_eq!(engine.state.phase, GamePhase::Lobby);

        // Start game and test ending round in wrong phase
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartGame,
        });

        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::EndRound,
        });
        assert_eq!(engine.state.phase, GamePhase::Score);

        // Test answer with no round_start_time
        let player_id = add_test_player(&mut engine, "Player1");
        engine.state.round_start_time = None;
        engine.state.phase = GamePhase::Question;

        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Answer {
                answer: "test".to_string(),
            },
        });
        engine.connections.clear();
    }

    #[tokio::test]
    async fn test_round_start_errors() {
        let (mut engine, admin_id) = setup_test_game();

        // First get into Score phase properly
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartGame,
        });
        assert_eq!(
            engine.state.phase,
            GamePhase::Score,
            "Should be in Score phase before test"
        );

        // Set current_question_index past the end to trigger "no more questions" error
        engine.state.current_question_index = engine.state.shuffled_question_indices.len();

        // Store the current index to verify it doesn't change
        let initial_index = engine.state.current_question_index;

        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartRound,
        });

        // Verify error conditions:
        // 1. Phase should not have changed
        assert_eq!(engine.state.phase, GamePhase::Score);
        // 2. Question index should not have changed
        assert_eq!(engine.state.current_question_index, initial_index);
        // 3. No current question should be set
        assert!(engine.state.current_question.is_none());
        // 4. No alternatives should be set
        assert!(engine.state.current_alternatives.is_empty());
        engine.connections.clear();
    }

    #[tokio::test]
    async fn test_skip_question() {
        let (mut engine, admin_id) = setup_test_game();

        // Start game to get into Score phase
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartGame,
        });
        assert_eq!(
            engine.state.phase,
            GamePhase::Score,
            "Game should start in Score phase"
        );

        // Store initial state
        let initial_index = engine.state.current_question_index;

        // Skip question in correct phase
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::SkipQuestion,
        });

        // Verify skip worked:
        assert_eq!(
            engine.state.phase,
            GamePhase::Score,
            "Phase should remain Score"
        );
        assert_eq!(
            engine.state.current_question_index,
            initial_index + 1,
            "Question index should increment"
        );
        assert!(
            engine.state.current_question.is_none(),
            "Current question should be cleared"
        );
        assert!(
            engine.state.current_alternatives.is_empty(),
            "Alternatives should be cleared"
        );
        engine.connections.clear();
    }

    #[tokio::test]
    async fn test_skip_question_errors() {
        let (mut engine, admin_id) = setup_test_game();

        // Start game to get into proper state
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartGame,
        });
        assert_eq!(
            engine.state.phase,
            GamePhase::Score,
            "Game should start in Score phase"
        );

        // Move to Question phase (wrong phase for skip)
        engine.state.phase = GamePhase::Question;

        // Store state before attempting skip
        let initial_index = engine.state.current_question_index;
        let initial_question = engine.state.current_question.clone();
        let initial_alternatives = engine.state.current_alternatives.clone();

        // Try to skip in wrong phase
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::SkipQuestion,
        });

        // Verify nothing changed due to error:
        assert_eq!(
            engine.state.phase,
            GamePhase::Question,
            "Phase should not change"
        );
        assert_eq!(
            engine.state.current_question_index, initial_index,
            "Question index should not change"
        );
        assert_eq!(
            engine.state.current_question, initial_question,
            "Current question should not change"
        );
        assert_eq!(
            engine.state.current_alternatives, initial_alternatives,
            "Alternatives should not change"
        );
        engine.connections.clear();
    }

    #[test]
    fn test_setup_round_errors() {
        let (mut engine, _) = setup_test_game();

        // Test setup_round error path
        engine.state.current_question_index = engine.state.shuffled_question_indices.len();
        let result = engine.setup_round();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_error_handling_branches() {
        let (mut engine, admin_id) = setup_test_game();
        let player_id = add_test_player(&mut engine, "Player1");

        // Test admin leave
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Leave,
        });

        // Assert admin was removed
        assert!(
            !engine.state.players.contains_key(&admin_id),
            "Admin should be removed after leaving"
        );

        // Test answer with non-existent player
        let invalid_id = Uuid::new_v4();
        engine.state.phase = GamePhase::Question;
        engine.state.round_start_time = Some(Instant::now());

        // Store state before invalid player attempts to answer
        let initial_scoreboard = engine.get_scoreboard();

        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: invalid_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Answer {
                answer: "test".to_string(),
            },
        });

        // Assert nothing changed after invalid player's answer
        assert_eq!(
            engine.get_scoreboard(),
            initial_scoreboard,
            "Scoreboard should not change from invalid player's answer"
        );
        assert!(
            !engine.state.players.contains_key(&invalid_id),
            "Invalid player should not be added"
        );

        // Test connection removal cases
        let initial_connection_count = engine.connections.len();
        engine.connections.remove(&player_id);
        assert_eq!(
            engine.connections.len(),
            initial_connection_count - 1,
            "Connection count should decrease"
        );

        // Test message sending to removed connection
        // Note: Since we can't directly observe message sending, we at least verify the system doesn't crash
        let pre_update_state = engine.state.clone();

        // Test Single recipient
        engine.push_update(
            Recipients::Single(player_id),
            GameUpdate::Error {
                message: "Test error".into(),
            },
        );
        assert_eq!(
            engine.state.phase, pre_update_state.phase,
            "Game state should not change after failed message"
        );

        // Test Multiple recipients
        engine.push_update(
            Recipients::Multiple(vec![player_id]),
            GameUpdate::Error {
                message: "Test error".into(),
            },
        );
        assert_eq!(
            engine.state.phase, pre_update_state.phase,
            "Game state should not change after failed multiple message"
        );

        // Test AllExcept
        engine.push_update(
            Recipients::_AllExcept(vec![admin_id]),
            GameUpdate::Error {
                message: "Test error".into(),
            },
        );
        assert_eq!(
            engine.state.phase, pre_update_state.phase,
            "Game state should not change after failed except message"
        );

        // Test All
        engine.push_update(
            Recipients::All,
            GameUpdate::Error {
                message: "Test error".into(),
            },
        );
        assert_eq!(
            engine.state.phase, pre_update_state.phase,
            "Game state should not change after failed broadcast"
        );

        let final_connection_count = engine.connections.len();
        engine.connections.clear();
        assert_eq!(
            engine.connections.len(),
            0,
            "All connections should be cleared"
        );
        assert!(
            final_connection_count > 0,
            "Should have had connections before clearing"
        );
        engine.connections.clear();
    }

    #[tokio::test]
    async fn test_regular_player_connect() {
        // Setup
        let admin_id = Uuid::new_v4();
        let admin_lobby_id = Uuid::new_v4();
        let connections = Arc::new(DashMap::new());

        // Create game engine with admin
        let mut engine = GameEngine::new(
            admin_id,
            Arc::new(create_test_questions()),
            None,
            30,
            connections,
        );

        // Add a player with message capture
        let player_id = Uuid::new_v4();
        let (player_tx, mut player_rx) = tokio::sync::mpsc::unbounded_channel();
        engine.connections.insert(
            player_id,
            Connection {
                lobby_id: admin_lobby_id,
                tx: Some(player_tx),
            },
        );
        engine.add_player(player_id, "Player1".to_string()).unwrap();

        // Test connect
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Connect,
        });

        // Verify player receives correct messages
        match player_rx
            .recv()
            .await
            .expect("Should receive Connected message")
        {
            GameUpdate::Connected {
                player_id: pid,
                name,
                ..
            } => {
                assert_eq!(pid, player_id);
                assert_eq!(name, "Player1");
            }
            other => panic!("Expected Connected message, got {:?}", other),
        }

        match player_rx.recv().await.expect("Should receive StateDelta") {
            GameUpdate::StateDelta {
                phase, admin_extra, ..
            } => {
                assert_eq!(phase, Some(GamePhase::Lobby));
                assert!(
                    admin_extra.is_none(),
                    "Regular player should not get admin_extra"
                );
            }
            other => panic!("Expected StateDelta, got {:?}", other),
        }
        engine.connections.clear();
        player_rx.close();
    }

    #[tokio::test]
    async fn test_admin_connect_during_question() {
        // Setup
        let admin_id = Uuid::new_v4();
        let admin_lobby_id = Uuid::new_v4();
        let connections = Arc::new(DashMap::new());
        let (admin_tx, mut admin_rx) = tokio::sync::mpsc::unbounded_channel();

        // Create game engine with admin
        let mut engine = GameEngine::new(
            admin_id,
            Arc::new(create_test_questions()),
            None,
            30,
            connections.clone(),
        );
        engine.add_player(admin_id, "Admin".to_string()).unwrap();

        // Set up question phase
        engine.state.phase = GamePhase::Question;
        let question = engine.state.all_questions[0].clone();
        engine.state.current_question = Some(question.clone());

        // Add admin connection and test connect
        connections.insert(
            admin_id,
            Connection {
                lobby_id: admin_lobby_id,
                tx: Some(admin_tx),
            },
        );

        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Connect,
        });

        // Verify admin receives all expected messages
        match admin_rx
            .recv()
            .await
            .expect("Should receive Connected message")
        {
            GameUpdate::Connected {
                player_id: pid,
                name,
                ..
            } => {
                assert_eq!(pid, admin_id);
                assert_eq!(name, "Admin");
            }
            other => panic!("Expected Connected message, got {:?}", other),
        }

        match admin_rx.recv().await.expect("Should receive StateDelta") {
            GameUpdate::StateDelta {
                phase, admin_extra, ..
            } => {
                assert_eq!(phase, Some(GamePhase::Question));
                assert!(admin_extra.is_some(), "Admin should get admin_extra");
            }
            other => panic!("Expected StateDelta, got {:?}", other),
        }

        match admin_rx.recv().await.expect("Should receive AdminInfo") {
            GameUpdate::AdminInfo {
                current_question: q,
            } => {
                assert_eq!(q.id, question.id);
            }
            other => panic!("Expected AdminInfo, got {:?}", other),
        }
        engine.connections.clear();
        admin_rx.close();
    }

    // #[tokio::test]
    // async fn test_connect_unregistered_player() {
    //     // Setup
    //     let admin_id = Uuid::new_v4();
    //     let admin_lobby_id = Uuid::new_v4();
    //     let connections = Arc::new(DashMap::new());
    //
    //     // Create game engine
    //     let mut engine = GameEngine::new(
    //         admin_id,
    //         Arc::new(create_test_questions()),
    //         None,
    //         30,
    //         connections.clone(),
    //     );
    //
    //     // Test unregistered player connect
    //     let invalid_id = Uuid::new_v4();
    //     let (invalid_tx, mut invalid_rx) = tokio::sync::mpsc::unbounded_channel();
    //     connections.insert(
    //         invalid_id,
    //         Connection {
    //             lobby_id: admin_lobby_id,
    //             tx: Some(invalid_tx),
    //         },
    //     );
    //
    //     engine.process_event(GameEvent {
    //         context: EventContext {
    //             sender_id: invalid_id,
    //             timestamp: Instant::now(),
    //         },
    //         action: GameAction::Connect,
    //     });
    //
    //     // Verify error message
    //     match invalid_rx
    //         .recv()
    //         .await
    //         .expect("Should receive Error message")
    //     {
    //         GameUpdate::Error { message } => {
    //             assert_eq!(
    //                 message,
    //                 "Player not found. Please register before connecting."
    //             );
    //         }
    //         other => panic!("Expected Error message, got {:?}", other),
    //     }
    //     engine.connections.clear();
    // }

    #[tokio::test]
    async fn test_admin_leave() {
        // Setup game with channels
        let admin_id = Uuid::new_v4();
        let admin_lobby_id = Uuid::new_v4();
        let connections = Arc::new(DashMap::new());
        let (admin_tx, _admin_rx) = tokio::sync::mpsc::unbounded_channel();

        connections.insert(
            admin_id,
            Connection {
                lobby_id: admin_lobby_id,
                tx: Some(admin_tx),
            },
        );

        let mut engine = GameEngine::new(
            admin_id,
            Arc::new(create_test_questions()),
            None,
            30,
            connections,
        );

        // Add admin to players
        engine.add_player(admin_id, "Admin".to_string()).unwrap();

        // Add a player and capture their messages
        let player_id = Uuid::new_v4();
        let (player_tx, mut player_rx) = tokio::sync::mpsc::unbounded_channel();
        engine.connections.insert(
            player_id,
            Connection {
                lobby_id: admin_lobby_id,
                tx: Some(player_tx),
            },
        );
        engine.add_player(player_id, "Player1".to_string()).unwrap();

        // Test admin disconnect
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Leave,
        });

        // Verify admin was removed
        assert!(
            !engine.state.players.contains_key(&admin_id),
            "Admin should be removed from players"
        );

        // Verify player received game closed message
        match player_rx
            .recv()
            .await
            .expect("Should receive GameClosed message")
        {
            GameUpdate::GameClosed { reason } => {
                assert_eq!(reason, "Host left the game");
            }
            other => panic!("Expected GameClosed message, got {:?}", other),
        }
        engine.connections.clear();
        player_rx.close();
    }

    #[tokio::test]
    async fn test_start_game_twice() {
        // Create admin information
        let admin_id = Uuid::new_v4();
        let admin_lobby_id = Uuid::new_v4();
        let connections = Arc::new(DashMap::new());

        // Create channel to capture admin messages
        let (admin_tx, mut admin_rx) = tokio::sync::mpsc::unbounded_channel();

        // Add admin connection
        connections.insert(
            admin_id,
            Connection {
                lobby_id: admin_lobby_id,
                tx: Some(admin_tx),
            },
        );

        // Create game engine directly
        let mut engine = GameEngine::new(
            admin_id,
            Arc::new(create_test_questions()),
            None,
            30,
            connections,
        );

        // Add admin to players
        engine.add_player(admin_id, "Admin".to_string()).unwrap();

        // First start - should succeed
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartGame,
        });

        // Drain the success messages from first start
        while let Ok(msg) = admin_rx.try_recv() {
            match msg {
                GameUpdate::StateDelta { phase, .. } => {
                    assert_eq!(
                        phase,
                        Some(GamePhase::Score),
                        "First start should move to Score phase"
                    );
                }
                GameUpdate::AdminNextQuestions { .. } => {
                    // Expected message
                }
                other => panic!("Unexpected message during first start: {:?}", other),
            }
        }

        // Second start - should fail
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartGame,
        });

        // Should receive error message
        match admin_rx.recv().await.expect("Should receive Error message") {
            GameUpdate::Error { message } => {
                assert_eq!(
                    message,
                    "Can only start game from lobby or after a finished game"
                );
            }
            other => panic!("Expected Error message, got {:?}", other),
        }

        // Verify game is still in Score phase
        assert_eq!(engine.state.phase, GamePhase::Score);
        engine.connections.clear();
    }

    #[test]
    fn test_inactivity_timeout() {
        let (mut engine, _) = setup_test_game();

        // Test case where last_lobby_message is None
        engine.state.last_lobby_message = None;
        assert!(!engine.is_finished());
        engine.connections.clear();
    }

    #[test]
    fn test_restart_game_in_same_lobby() {
        use std::time::Instant;

        // build a game with an admin and some questions
        let (mut engine, admin_id) = setup_test_game();

        // ── first game starts ────────────────────────────────────────────────────
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartGame,
        });
        assert_eq!(engine.state.phase, GamePhase::Score);

        // ── admin ends the game ──────────────────────────────────────────────────
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::EndGame {
                reason: "finished".to_string(),
            },
        });
        assert_eq!(engine.state.phase, GamePhase::GameOver);

        // ── admin starts a new game in the same lobby ────────────────────────────
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartGame,
        });
        assert_eq!(engine.state.phase, GamePhase::Score);

        // every player’s score must be back to zero
        assert!(engine
            .state
            .players
            .values()
            .all(|p| p.score == 0 && p.round_score == 0));

        // fresh question sequence
        assert_eq!(engine.state.current_question_index, 0);
    }

    // #[tokio::test]
    // async fn test_admin_kick_player() {
    //     let (mut engine, admin_id) = setup_test_game();
    //     let player1_id = add_test_player(&mut engine, "Player1");
    //     let player2_id = add_test_player(&mut engine, "Player2");
    //
    //     // Capture messages for player 1
    //     let (_player1_tx, mut player1_rx) = tokio::sync::mpsc::unbounded_channel();
    //     engine
    //         .connections
    //         .entry(player1_id)
    //         .and_modify(|c| c.tx = Some(_player1_tx));
    //
    //     let now = Instant::now();
    //
    //     // Admin kicks Player1
    //     engine.process_event(GameEvent {
    //         context: EventContext {
    //             sender_id: admin_id,
    //             timestamp: now,
    //         },
    //         action: GameAction::KickPlayer {
    //             player_name: "Player1".to_string(),
    //         },
    //     });
    //
    //     // Verify Player1 is removed from state
    //     assert!(!engine.state.players.contains_key(&player1_id));
    //     assert!(engine.state.players.contains_key(&player2_id)); // Player2 should remain
    //
    //     // Verify Player1's connection is removed from the shared map
    //     assert!(!engine.connections.contains_key(&player1_id));
    //
    //     // Verify Player1 received the Kicked message
    //     match player1_rx
    //         .recv()
    //         .await
    //         .expect("Player1 should receive a message")
    //     {
    //         GameUpdate::PlayerKicked { reason } => {
    //             assert_eq!(reason, "Kicked by admin");
    //         }
    //         other => panic!("Player1 expected PlayerKicked, got {:?}", other),
    //     }
    //
    //     // Verify scoreboard update sent to remaining players (Player2 and Admin)
    //     // (Need to capture messages for Player2 and Admin to fully verify)
    //
    //     engine.connections.clear(); // Cleanup
    //     player1_rx.close();
    // }
    //
    // #[tokio::test]
    // async fn test_admin_kick_nonexistent_player() {
    //     let (mut engine, admin_id) = setup_test_game();
    //     add_test_player(&mut engine, "Player1");
    //
    //     // Capture messages for admin
    //     let (_admin_tx, mut admin_rx) = tokio::sync::mpsc::unbounded_channel();
    //     engine
    //         .connections
    //         .entry(admin_id)
    //         .and_modify(|c| c.tx = Some(_admin_tx));
    //
    //     let initial_player_count = engine.state.players.len();
    //     let now = Instant::now();
    //
    //     // Admin tries to kick a player who doesn't exist
    //     engine.process_event(GameEvent {
    //         context: EventContext {
    //             sender_id: admin_id,
    //             timestamp: now,
    //         },
    //         action: GameAction::KickPlayer {
    //             player_name: "Ghost".to_string(),
    //         },
    //     });
    //
    //     // Verify no players were removed
    //     assert_eq!(engine.state.players.len(), initial_player_count);
    //
    //     // Verify admin received an error message
    //     match admin_rx
    //         .recv()
    //         .await
    //         .expect("Admin should receive a message")
    //     {
    //         GameUpdate::Error { message } => {
    //             assert!(message.contains("Player 'Ghost' not found"));
    //         }
    //         other => panic!("Admin expected Error, got {:?}", other),
    //     }
    //     engine.connections.clear(); // Cleanup
    //     admin_rx.close();
    // }
    //
    // #[tokio::test]
    // async fn test_admin_kick_self() {
    //     let (mut engine, admin_id) = setup_test_game();
    //     engine.add_player(admin_id, "Admin".to_string()).unwrap(); // Ensure admin is in players map
    //
    //     // Capture messages for admin
    //     let (_admin_tx, mut admin_rx) = tokio::sync::mpsc::unbounded_channel();
    //     engine
    //         .connections
    //         .entry(admin_id)
    //         .and_modify(|c| c.tx = Some(_admin_tx));
    //
    //     let initial_player_count = engine.state.players.len();
    //     let now = Instant::now();
    //
    //     // Admin tries to kick themselves
    //     engine.process_event(GameEvent {
    //         context: EventContext {
    //             sender_id: admin_id,
    //             timestamp: now,
    //         },
    //         action: GameAction::KickPlayer {
    //             player_name: "Admin".to_string(),
    //         },
    //     });
    //
    //     // Verify admin was not removed
    //     assert!(engine.state.players.contains_key(&admin_id));
    //     assert_eq!(engine.state.players.len(), initial_player_count);
    //
    //     // Verify admin received an error message
    //     match admin_rx
    //         .recv()
    //         .await
    //         .expect("Admin should receive a message")
    //     {
    //         GameUpdate::Error { message } => {
    //             assert!(message.contains("Admin cannot kick themselves"));
    //         }
    //         other => panic!("Admin expected Error, got {:?}", other),
    //     }
    //
    //     engine.connections.clear(); // Cleanup
    //     admin_rx.close();
    // }
}
