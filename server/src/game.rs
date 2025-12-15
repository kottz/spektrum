use crate::db::QuestionSet;
use crate::question::GameQuestion;
use crate::uuid::Uuid;
use axum::extract::ws::Utf8Bytes;
use bytes::{BufMut, BytesMut};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, instrument, warn};

lazy_static! {
    pub(crate) static ref NAME_VALIDATION_REGEX: Regex =
        Regex::new(r"^[\p{L}\p{N}_\-\. ]+$").expect("Failed to compile player name regex");
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
    mut existing_names: impl Iterator<Item = &'a str>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GamePhase {
    Lobby,
    Score,
    Question,
    GameOver,
    GameClosed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AdminExtraInfo {
    pub upcoming_questions: Vec<GameQuestion>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum GameUpdate {
    /// A lightweight acknowledgement of connection.
    Connected {
        player_id: Uuid,
        name: Arc<str>,
        round_duration: u64,
    },
    /// A partial (delta) state update.
    StateDelta {
        phase: Option<GamePhase>,
        question_type: Option<Arc<str>>,
        question_text: Option<Arc<str>>,
        alternatives: Option<Vec<Arc<str>>>,
        question_time_remaining_ms: Option<u64>,
        answered_player_names: Option<Vec<Arc<str>>>,
        scoreboard: Option<Vec<(Arc<str>, i32)>>,
        round_scores: Option<Vec<(Arc<str>, i32)>>,
        consecutive_misses: Option<Vec<(Arc<str>, u32)>>,
        // Optional extra info for admin
        admin_extra: Option<AdminExtraInfo>,
    },
    PlayerLeft {
        name: Arc<str>,
    },
    PlayerKicked {
        reason: Arc<str>,
    },
    Answered {
        name: Arc<str>,
        score: i32,
    },
    GameOver {
        final_scores: Vec<(Arc<str>, i32)>,
        reason: Arc<str>,
    },
    GameClosed {
        reason: Arc<str>,
    },
    Error {
        message: Arc<str>,
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
    KickPlayer { player_name: Arc<str> },
    EndGame { reason: Arc<str> },
    CloseGame { reason: Arc<str> },
}

impl GameAction {
    /// Returns the variant name without any payload data (safe for logging)
    pub fn kind(&self) -> &'static str {
        match self {
            GameAction::Connect => "Connect",
            GameAction::Leave => "Leave",
            GameAction::Answer { .. } => "Answer",
            GameAction::StartGame => "StartGame",
            GameAction::StartRound => "StartRound",
            GameAction::EndRound => "EndRound",
            GameAction::SkipQuestion => "SkipQuestion",
            GameAction::KickPlayer { .. } => "KickPlayer",
            GameAction::EndGame { .. } => "EndGame",
            GameAction::CloseGame { .. } => "CloseGame",
        }
    }
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
    pub current_alternatives: Vec<Arc<str>>,
    pub correct_answers: Option<Vec<Arc<str>>>,
    pub current_question: Option<GameQuestion>,
    pub all_questions: Arc<Vec<GameQuestion>>,
    pub shuffled_question_indices: Vec<usize>,
    pub current_question_index: usize,
    pub last_lobby_message: Option<Instant>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PlayerState {
    pub name: Arc<str>,
    pub score: i32,
    pub round_score: i32,
    pub has_answered: bool,
    pub answer: Option<Arc<str>>,
    pub consecutive_misses: u32,
    #[serde(skip)]
    pub tx: Option<Sender<Utf8Bytes>>,
    #[serde(skip)]
    pub connection_id: Option<Uuid>,
}

impl PlayerState {
    pub fn new(name: Arc<str>) -> Self {
        Self {
            name,
            score: 0,
            round_score: 0,
            has_answered: false,
            answer: None,
            consecutive_misses: 0,
            tx: None,
            connection_id: None,
        }
    }
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
        }
    }

    pub fn update_player_connection(
        &mut self,
        player_id: Uuid,
        tx: Sender<Utf8Bytes>,
        connection_id: Uuid,
    ) {
        if let Some(player) = self.state.players.get_mut(&player_id) {
            player.tx = Some(tx);
            player.connection_id = Some(connection_id);
        }
    }

    pub fn clear_player_connection(&mut self, player_id: Uuid, connection_id: Uuid) {
        if let Some(player) = self.state.players.get_mut(&player_id)
            && player.connection_id == Some(connection_id)
        {
            player.tx = None;
            player.connection_id = None;
        }
    }

    pub fn add_player(&mut self, player_id: Uuid, name: String) -> Result<(), NameValidationError> {
        let trimmed = name.trim();
        let existing_names = self.state.players.values().map(|p| p.name.as_ref());
        validate_player_name(trimmed, existing_names)?;
        self.state
            .players
            .insert(player_id, PlayerState::new(Arc::from(trimmed)));
        Ok(())
    }

    pub fn last_update(&self) -> Option<Instant> {
        self.state.last_lobby_message
    }

    pub fn is_finished(&self) -> bool {
        if self.state.phase == GamePhase::GameClosed {
            return true;
        }
        if let Some(last_msg) = self.state.last_lobby_message
            && Instant::now().duration_since(last_msg) > Duration::from_secs(3600)
        {
            self.push_update(
                Recipients::All,
                GameUpdate::GameClosed {
                    reason: "Lobby closed due to inactivity".into(),
                },
            );
            return true;
        }
        false
    }

    pub fn is_full(&self) -> bool {
        self.state.players.len() >= 1024
    }

    pub fn get_lobby_stats(&self) -> (usize, usize, Vec<(Arc<str>, i32)>) {
        let total_players = self.state.players.len();
        let questions_played = self.state.current_question_index;
        let player_scores: Vec<(Arc<str>, i32)> = self
            .state
            .players
            .values()
            .map(|p| (p.name.clone(), p.score))
            .collect();
        (total_players, questions_played, player_scores)
    }

    pub fn get_consecutive_misses(&self) -> Vec<(Arc<str>, u32)> {
        self.state
            .players
            .values()
            .map(|p| (p.name.clone(), p.consecutive_misses))
            .collect()
    }

    pub fn get_round_scores(&self) -> Vec<(Arc<str>, i32)> {
        self.state
            .players
            .values()
            .map(|p| (p.name.clone(), p.round_score))
            .collect()
    }

    fn get_question_time_remaining_ms(&self, now: Instant) -> Option<u64> {
        let start = self.state.round_start_time?;
        let elapsed_ms = now.duration_since(start).as_millis();
        let total_ms = self.state.round_duration as u128 * 1000;
        Some(total_ms.saturating_sub(elapsed_ms) as u64)
    }

    fn get_answered_player_names(&self) -> Vec<Arc<str>> {
        self.state
            .players
            .iter()
            .filter(|(_, p)| p.has_answered)
            .map(|(_, p)| p.name.clone())
            .collect()
    }

    #[cfg(test)]
    pub fn get_admin_id(&self) -> Uuid {
        self.state.admin_id
    }

    #[cfg(test)]
    pub fn get_round_duration(&self) -> u64 {
        self.state.round_duration
    }

    #[cfg(test)]
    pub fn get_player_count(&self) -> usize {
        self.state.players.len()
    }

    pub fn has_player(&self, player_id: &Uuid) -> bool {
        self.state.players.contains_key(player_id)
    }

    fn get_scoreboard(&self) -> Vec<(Arc<str>, i32)> {
        self.state
            .players
            .iter()
            .filter(|(id, _)| **id != self.state.admin_id)
            .map(|(_, p)| (p.name.clone(), p.score))
            .collect()
    }

    fn push_update(&self, recipients: Recipients, update: GameUpdate) {
        let mut writer = BytesMut::with_capacity(2048).writer();

        if let Err(e) = serde_json::to_writer(&mut writer, &update) {
            error!("Failed to serialize game update to writer: {}", e);
            return;
        }

        let bytes = writer.into_inner().freeze();
        let payload = match Utf8Bytes::try_from(bytes) {
            Ok(payload) => payload,
            Err(e) => {
                error!("Failed to convert bytes to Utf8Bytes: {}", e);
                return;
            }
        };

        match recipients {
            Recipients::Single(target) => {
                if let Some(player) = self.state.players.get(&target)
                    && let Some(tx) = &player.tx
                {
                    let _ = tx.try_send(payload.clone());
                }
            }
            Recipients::Multiple(targets) => {
                for target in targets {
                    if let Some(player) = self.state.players.get(&target)
                        && let Some(tx) = &player.tx
                    {
                        let _ = tx.try_send(payload.clone());
                    }
                }
            }
            Recipients::_AllExcept(exclusions) => {
                for (player_id, player) in self.state.players.iter() {
                    if !exclusions.contains(player_id)
                        && let Some(tx) = &player.tx
                    {
                        let _ = tx.try_send(payload.clone());
                    }
                }
            }
            Recipients::All => {
                for player in self.state.players.values() {
                    if let Some(tx) = &player.tx {
                        let _ = tx.try_send(payload.clone());
                    }
                }
            }
        }
    }

    #[instrument(
        level = "debug",
        skip(self, event),
        fields(
            sender_id = %event.context.sender_id,
            action = %event.action.kind(),
            phase = ?self.state.phase,
        )
    )]
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
                    debug!(
                        sender_id = %event.context.sender_id,
                        action = %event.action.kind(),
                        "Admin action denied: sender is not admin"
                    );
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
                    .map(|q| Arc::from(q.get_question_type())),
                question_text: self
                    .state
                    .current_question
                    .as_ref()
                    .and_then(|q| q.question_text.clone()),
                alternatives: Some(self.state.current_alternatives.clone()),
                question_time_remaining_ms: if self.state.phase == GamePhase::Question {
                    self.get_question_time_remaining_ms(ctx.timestamp)
                } else {
                    None
                },
                answered_player_names: if self.state.phase == GamePhase::Question {
                    Some(self.get_answered_player_names())
                } else {
                    None
                },
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
                        question_time_remaining_ms: None,
                        answered_player_names: None,
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
                && let Some(question) = &self.state.current_question
            {
                self.push_update(
                    Recipients::Single(self.state.admin_id),
                    GameUpdate::AdminInfo {
                        current_question: question.clone(),
                    },
                );
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
                self.state.phase = GamePhase::GameClosed;
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
            debug!(
                sender_id = %ctx.sender_id,
                current_phase = ?self.state.phase,
                "Answer rejected: wrong phase"
            );
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    message: "Not in Question phase".into(),
                },
            );
            return;
        }
        let (player_name, score) = {
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
                .is_some_and(|answers| answers.iter().any(|a| a.as_ref() == answer));
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
            player.answer = Some(Arc::from(answer));
            (player.name.clone(), score_delta)
        };
        self.push_update(
            Recipients::All,
            GameUpdate::Answered {
                name: player_name,
                score,
            },
        );
    }

    fn handle_start_game(&mut self, ctx: EventContext) {
        if self.state.phase != GamePhase::Lobby && self.state.phase != GamePhase::GameOver {
            debug!(
                sender_id = %ctx.sender_id,
                current_phase = ?self.state.phase,
                "StartGame rejected: wrong phase"
            );
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    message: "Can only start game from lobby or after a finished game".into(),
                },
            );
            return;
        }

        let from_phase = self.state.phase;

        // if we came from a finished game, zero everything out
        if self.state.phase == GamePhase::GameOver {
            self.reset_for_new_game();
        }

        self.state.phase = GamePhase::Score;
        debug!(from = ?from_phase, to = ?self.state.phase, "Phase transition");
        self.push_update(
            Recipients::All,
            GameUpdate::StateDelta {
                phase: Some(GamePhase::Score),
                question_type: None,
                question_text: None,
                alternatives: None,
                question_time_remaining_ms: None,
                answered_player_names: None,
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
            debug!(
                sender_id = %ctx.sender_id,
                current_phase = ?self.state.phase,
                "StartRound rejected: wrong phase"
            );
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    message: "Can only start round from score phase".into(),
                },
            );
            return;
        }
        if self.state.current_question_index >= self.state.all_questions.len() {
            debug!(
                question_index = self.state.current_question_index,
                total_questions = self.state.all_questions.len(),
                "StartRound rejected: no more questions"
            );
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
                debug!(
                    from = ?GamePhase::Score,
                    to = ?GamePhase::Question,
                    question_index = self.state.current_question_index,
                    "Phase transition"
                );
                self.push_update(
                    Recipients::All,
                    GameUpdate::StateDelta {
                        phase: Some(GamePhase::Question),
                        question_type: Some(Arc::from(question.get_question_type())),
                        question_text: question.question_text.clone(),
                        alternatives: Some(self.state.current_alternatives.clone()),
                        question_time_remaining_ms: Some(self.state.round_duration * 1000),
                        answered_player_names: Some(Vec::new()),
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
                    GameUpdate::Error {
                        message: Arc::from(msg),
                    },
                );
            }
        }
    }

    fn handle_end_round(&mut self, ctx: EventContext) {
        if self.state.phase != GamePhase::Question {
            debug!(
                sender_id = %ctx.sender_id,
                current_phase = ?self.state.phase,
                "EndRound rejected: wrong phase"
            );
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
        debug!(from = ?GamePhase::Question, to = ?GamePhase::Score, "Phase transition");
        self.push_update(
            Recipients::All,
            GameUpdate::StateDelta {
                phase: Some(GamePhase::Score),
                question_type: None,
                question_text: None,
                alternatives: None,
                question_time_remaining_ms: None,
                answered_player_names: None,
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

    fn handle_kick_player(&mut self, ctx: EventContext, target_player_name: Arc<str>) {
        // Find the player ID based on the name
        let target_player_id = self
            .state
            .players
            .iter()
            .find(|(_, p)| p.name.as_ref() == target_player_name.as_ref())
            .map(|(id, _)| *id);
        let target_player_id = match target_player_id {
            Some(id) => id,
            None => {
                self.push_update(
                    Recipients::Single(ctx.sender_id), // Send error to admin
                    GameUpdate::Error {
                        message: Arc::from(format!("Player '{}' not found.", target_player_name)),
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
                    reason: Arc::from("Kicked by admin"),
                },
            );

            // Now we can safely remove the player
            self.state.players.remove(&target_player_id);

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
                    question_time_remaining_ms: None,
                    answered_player_names: None,
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
                    message: Arc::from(format!(
                        "Failed to remove player '{}' internally.",
                        target_player_name
                    )),
                },
            );
        }
    }

    fn handle_end_game(&mut self, _ctx: EventContext, reason: Arc<str>) {
        let from_phase = self.state.phase;
        self.state.phase = GamePhase::GameOver;
        debug!(from = ?from_phase, to = ?GamePhase::GameOver, "Phase transition");
        self.push_update(
            Recipients::All,
            GameUpdate::GameOver {
                final_scores: self.get_scoreboard(),
                reason,
            },
        );
    }

    fn handle_close_game(&mut self, _ctx: EventContext, reason: Arc<str>) {
        let from_phase = self.state.phase;
        self.state.phase = GamePhase::GameClosed;
        debug!(from = ?from_phase, to = ?GamePhase::GameClosed, "Phase transition");
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
    use serde::Deserialize;
    use std::time::Duration;
    use tokio::sync::mpsc::Receiver;

    async fn receive_and_deserialize<T>(rx: &mut Receiver<Utf8Bytes>) -> T
    where
        T: for<'de> Deserialize<'de> + std::fmt::Debug,
    {
        let payload = rx
            .recv()
            .await
            .expect("Test failed: Channel closed unexpectedly or failed to receive message.");
        let json = payload.as_str();
        serde_json::from_str::<T>(json).unwrap_or_else(|_| {
            panic!(
                "Test failed: Failed to deserialize received JSON: '{}'",
                json
            )
        })
    }

    fn create_test_questions() -> Vec<GameQuestion> {
        vec![
            // Color question
            GameQuestion {
                id: 1,
                question_type: QuestionType::Color,
                question_text: None,
                title: Arc::from("What color is predominantly used in this video?"),
                artist: Some(Arc::from("Test Artist")),
                youtube_id: Arc::from("test123"),
                options: vec![
                    GameQuestionOption {
                        option: Arc::from("Red"),
                        is_correct: true,
                    },
                    GameQuestionOption {
                        option: Arc::from("Blue"),
                        is_correct: false,
                    },
                ],
            },
            // Text question
            GameQuestion {
                id: 2,
                question_type: QuestionType::Text,
                question_text: None,
                title: Arc::from("What is the main theme of this video?"),
                artist: Some(Arc::from("Test Artist")),
                youtube_id: Arc::from("test456"),
                options: vec![
                    GameQuestionOption {
                        option: Arc::from("Love"),
                        is_correct: true,
                    },
                    GameQuestionOption {
                        option: Arc::from("War"),
                        is_correct: false,
                    },
                    GameQuestionOption {
                        option: Arc::from("Peace"),
                        is_correct: false,
                    },
                ],
            },
            // Year question
            GameQuestion {
                id: 3,
                question_type: QuestionType::Year,
                question_text: None,
                title: Arc::from("When was this video released?"),
                artist: Some(Arc::from("Test Artist")),
                youtube_id: Arc::from("test789"),
                options: vec![GameQuestionOption {
                    option: Arc::from("2020"),
                    is_correct: true,
                }],
            },
        ]
    }

    fn setup_test_game() -> (GameEngine, Uuid) {
        let admin_id = Uuid::new_v4();
        let questions = Arc::new(create_test_questions());
        let (tx, _rx) = tokio::sync::mpsc::channel(128);
        let admin_conn_id = Uuid::new_v4();
        let mut engine = GameEngine::new(admin_id, questions, None, 30);
        engine.add_player(admin_id, "Admin".into()).unwrap();
        engine.update_player_connection(admin_id, tx, admin_conn_id);

        (engine, admin_id)
    }

    fn add_test_player(engine: &mut GameEngine, name: &str) -> Uuid {
        let player_id = Uuid::new_v4();
        let (tx, _rx) = tokio::sync::mpsc::channel(128);
        let conn_id = Uuid::new_v4();
        engine.add_player(player_id, name.to_string()).unwrap();
        engine.update_player_connection(player_id, tx, conn_id);
        player_id
    }

    fn add_test_player_with_channel(
        engine: &mut GameEngine,
        name: &str,
    ) -> (Uuid, Receiver<Utf8Bytes>) {
        let player_id = add_test_player(engine, name);
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let conn_id = Uuid::new_v4();
        engine.update_player_connection(player_id, tx, conn_id);
        (player_id, rx)
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
                    answer: correct_answer.to_string(),
                },
            });

            // Player 2 answers correctly but slower
            engine.process_event(GameEvent {
                context: EventContext {
                    sender_id: player2_id,
                    timestamp: now + Duration::from_secs(5),
                },
                action: GameAction::Answer {
                    answer: correct_answer.to_string(),
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
                reason: Arc::from("Game completed"),
            },
        });
        assert_eq!(engine.state.phase, GamePhase::GameOver);
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
                    assert!(
                        engine
                            .state
                            .current_alternatives
                            .iter()
                            .all(|c| c.parse::<Color>().is_ok())
                    );
                }
                QuestionType::Year => {
                    assert_eq!(engine.state.current_alternatives.len(), 5);
                    assert!(
                        engine
                            .state
                            .current_alternatives
                            .iter()
                            .all(|y| y.parse::<i32>().is_ok())
                    );
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
    }

    #[tokio::test]
    async fn test_error_conditions() {
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
                answer: answer.to_string(),
            },
        });
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Answer {
                answer: answer.to_string(),
            },
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
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::CloseGame {
                reason: Arc::from("Test close"),
            },
        });
    }

    #[tokio::test]
    async fn test_player_reconnect_lobby() {
        let (mut engine, admin_id) = setup_test_game();
        let player_id = add_test_player(&mut engine, "Player1");
        // Stay in lobby; do not start the game
        let player_initial_state = engine.state.players.get(&player_id).unwrap().clone();
        let initial_conn_id = player_initial_state
            .connection_id
            .expect("connection id should be set");

        // Simulate disconnection (remove the connection)
        engine.clear_player_connection(player_id, initial_conn_id);

        // Re-add the player with a new connection
        let (player_tx, mut player_rx) = tokio::sync::mpsc::channel(128);
        let reconnect_conn_id = Uuid::new_v4();
        engine.update_player_connection(player_id, player_tx, reconnect_conn_id);

        // Reconnect
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Connect,
        });

        // Verify Connected message
        match receive_and_deserialize(&mut player_rx).await {
            GameUpdate::Connected { player_id: pid, .. } => {
                assert_eq!(pid, player_id, "Correct player ID in Connected");
            }
            other => panic!("Expected Connected, got {:?}", other),
        }
        // Verify StateDelta message
        match receive_and_deserialize(&mut player_rx).await {
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
            }
            other => panic!("Expected StateDelta, got {:?}", other),
        }

        player_rx.close();

        // Ensure admin connection is unaffected
        assert!(
            engine
                .state
                .players
                .get(&admin_id)
                .and_then(|p| p.tx.clone())
                .is_some(),
            "Admin connection should remain intact"
        );
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
        let initial_conn_id = player_initial_state
            .connection_id
            .expect("connection id should be set");

        // Simulate disconnection (remove the connection)
        engine.clear_player_connection(player_id, initial_conn_id);

        // Re-add the player with a new connection
        let (player_tx, mut player_rx) = tokio::sync::mpsc::channel(128);
        let reconnection_id = Uuid::new_v4();
        engine.update_player_connection(player_id, player_tx, reconnection_id);

        // Reconnect
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Connect,
        });

        // Verify Connected message
        match receive_and_deserialize(&mut player_rx).await {
            GameUpdate::Connected { player_id: pid, .. } => {
                assert_eq!(pid, player_id, "Correct player ID in Connected");
            }
            other => panic!("Expected Connected, got {:?}", other),
        }

        // Verify StateDelta message
        match receive_and_deserialize(&mut player_rx).await {
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
        let existing_names = ["TestName"];
        assert!(matches!(
            validate_player_name("TestName", existing_names.iter().copied()),
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
                reason: Arc::from("Test close"),
            },
        });
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
    }

    #[test]
    fn test_question_set_handling() {
        let admin_id = Uuid::new_v4();
        let questions = Arc::new(create_test_questions());

        let question_set = QuestionSet {
            id: 1,
            question_ids: vec![1, 2], // Only include first two questions
            name: Arc::from("Test Set"),
        };

        let mut engine = GameEngine::new(admin_id, questions, Some(&question_set), 30);
        engine.add_player(admin_id, "Admin".into()).unwrap();

        assert_eq!(engine.state.shuffled_question_indices.len(), 2);
    }

    #[tokio::test]
    async fn test_admin_reconnect() {
        let (mut engine, admin_id) = setup_test_game();

        // Create channel to capture admin messages
        let (admin_tx, mut admin_rx) = tokio::sync::mpsc::channel(128);
        let admin_conn_id = Uuid::new_v4();
        engine.update_player_connection(admin_id, admin_tx, admin_conn_id);

        // First test: admin reconnects in lobby, should get upcoming questions
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Connect,
        });

        // Should receive Connected message first
        match receive_and_deserialize(&mut admin_rx).await {
            GameUpdate::Connected { player_id, .. } => {
                assert_eq!(player_id, admin_id);
            }
            other => panic!("Expected Connected message, got {:?}", other),
        }

        // Should receive StateDelta with admin_extra next
        match receive_and_deserialize(&mut admin_rx).await {
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
        let (admin_tx2, mut admin_rx2) = tokio::sync::mpsc::channel(128);
        let admin_conn_id2 = Uuid::new_v4();
        engine.update_player_connection(admin_id, admin_tx2, admin_conn_id2);

        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Connect,
        });

        // Should receive Connected message first
        match receive_and_deserialize(&mut admin_rx2).await {
            GameUpdate::Connected { player_id, .. } => {
                assert_eq!(player_id, admin_id);
            }
            other => panic!("Expected Connected message, got {:?}", other),
        }

        // Should receive StateDelta next
        match receive_and_deserialize(&mut admin_rx2).await {
            GameUpdate::StateDelta { admin_extra, .. } => {
                assert!(admin_extra.is_some(), "Admin should receive admin_extra");
            }
            other => panic!("Expected StateDelta message, got {:?}", other),
        }

        // Should receive current question info
        match receive_and_deserialize(&mut admin_rx2).await {
            GameUpdate::AdminInfo { current_question } => {
                assert_eq!(current_question.id, test_question.id);
            }
            other => panic!("Expected AdminInfo message, got {:?}", other),
        }
        admin_rx2.close();
        admin_rx.close();
    }

    #[tokio::test]
    async fn test_admin_leave_message() {
        let (mut engine, admin_id) = setup_test_game();

        // Add a player
        let (_player_id, mut player_rx) = add_test_player_with_channel(&mut engine, "TestPlayer");

        // Admin leaves
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Leave,
        });

        // Use tokio::spawn to handle message receiving in parallel
        let message: GameUpdate = receive_and_deserialize(&mut player_rx).await;

        // Verify the message
        match message {
            GameUpdate::GameClosed { reason } => {
                assert_eq!(reason.as_ref(), "Host left the game");
            }
            other => panic!("Expected GameClosed message, got {:?}", other),
        }

        // State verifications
        assert!(!engine.state.players.contains_key(&admin_id));
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
        let conn_id = engine.state.players[&player_id]
            .connection_id
            .expect("connection id should be present");
        engine.clear_player_connection(player_id, conn_id);
        assert!(
            engine
                .state
                .players
                .get(&player_id)
                .and_then(|p| p.tx.clone())
                .is_none(),
            "Connection should be cleared"
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

        assert!(
            engine
                .state
                .players
                .get(&player_id)
                .and_then(|p| p.tx.clone())
                .is_none(),
            "Connection should remain cleared"
        );
    }

    #[tokio::test]
    async fn test_regular_player_connect() {
        let (mut engine, _admin_id) = setup_test_game();

        // Add a player with message capture
        let (player_id, mut player_rx) = add_test_player_with_channel(&mut engine, "Player1");

        // Test connect
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Connect,
        });

        // Verify player receives correct messages
        match receive_and_deserialize(&mut player_rx).await {
            GameUpdate::Connected {
                player_id: pid,
                name,
                ..
            } => {
                assert_eq!(pid, player_id);
                assert_eq!(name.as_ref(), "Player1");
            }
            other => panic!("Expected Connected message, got {:?}", other),
        }

        match receive_and_deserialize(&mut player_rx).await {
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
        player_rx.close();
    }

    #[tokio::test]
    async fn test_admin_connect_during_question() {
        let (mut engine, admin_id) = setup_test_game();
        let (admin_tx, mut admin_rx) = tokio::sync::mpsc::channel(128);
        let admin_conn_id = Uuid::new_v4();
        engine.update_player_connection(admin_id, admin_tx, admin_conn_id);

        // Set up question phase
        engine.state.phase = GamePhase::Question;
        let question = engine.state.all_questions[0].clone();
        engine.state.current_question = Some(question.clone());

        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Connect,
        });

        // Verify admin receives all expected messages
        match receive_and_deserialize(&mut admin_rx).await {
            GameUpdate::Connected {
                player_id: pid,
                name,
                ..
            } => {
                assert_eq!(pid, admin_id);
                assert_eq!(name.as_ref(), "Admin");
            }
            other => panic!("Expected Connected message, got {:?}", other),
        }

        match receive_and_deserialize(&mut admin_rx).await {
            GameUpdate::StateDelta {
                phase, admin_extra, ..
            } => {
                assert_eq!(phase, Some(GamePhase::Question));
                assert!(admin_extra.is_some(), "Admin should get admin_extra");
            }
            other => panic!("Expected StateDelta, got {:?}", other),
        }

        match receive_and_deserialize(&mut admin_rx).await {
            GameUpdate::AdminInfo {
                current_question: q,
            } => {
                assert_eq!(q.id, question.id);
            }
            other => panic!("Expected AdminInfo, got {:?}", other),
        }
        admin_rx.close();
    }

    #[tokio::test]
    async fn test_player_reconnect_during_question_receives_timer_and_answers() {
        let (mut engine, admin_id) = setup_test_game();
        let player1_id = add_test_player(&mut engine, "Player1");
        let (player2_id, mut player2_rx) = add_test_player_with_channel(&mut engine, "Player2");

        let start_time = Instant::now();

        // Move into question phase
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: start_time,
            },
            action: GameAction::StartGame,
        });
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: start_time,
            },
            action: GameAction::StartRound,
        });

        // Player 1 answers so they appear in the answered list
        let answer = engine.state.current_alternatives[0].clone();
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player1_id,
                timestamp: start_time + Duration::from_secs(3),
            },
            action: GameAction::Answer {
                answer: answer.to_string(),
            },
        });

        // Simulate player 2 reconnecting mid-question
        let initial_conn_id = engine.state.players[&player2_id]
            .connection_id
            .expect("connection id should be set");
        engine.clear_player_connection(player2_id, initial_conn_id);
        let (reconnect_tx, mut reconnect_rx) = tokio::sync::mpsc::channel(128);
        let reconnect_conn_id = Uuid::new_v4();
        engine.update_player_connection(player2_id, reconnect_tx, reconnect_conn_id);

        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player2_id,
                timestamp: start_time + Duration::from_secs(10),
            },
            action: GameAction::Connect,
        });

        // Skip Connected message
        let _ = receive_and_deserialize::<GameUpdate>(&mut reconnect_rx).await;

        match receive_and_deserialize(&mut reconnect_rx).await {
            GameUpdate::StateDelta {
                question_time_remaining_ms,
                answered_player_names,
                ..
            } => {
                let remaining = question_time_remaining_ms
                    .expect("question_time_remaining_ms should be present");
                assert!(
                    (19_000..=30_000).contains(&remaining),
                    "remaining time should be within expected window, got {}",
                    remaining
                );
                let answered = answered_player_names
                    .expect("answered_player_names should be present in question");
                assert!(
                    answered.contains(&engine.state.players[&player1_id].name),
                    "answered list should include player1"
                );
                assert!(
                    !answered.contains(&engine.state.players[&player2_id].name),
                    "answered list should not include reconnecting player who has not answered"
                );
            }
            other => panic!("Expected StateDelta, got {:?}", other),
        }
        player2_rx.close();
        reconnect_rx.close();
    }

    #[tokio::test]
    async fn test_connect_unregistered_player() {
        let (mut engine, admin_id) = setup_test_game();
        let initial_player_count = engine.get_player_count();

        // Attempt to connect with an unknown player id
        let invalid_id = Uuid::new_v4();
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: invalid_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Connect,
        });

        // Ensure state is unchanged and admin remains present
        assert_eq!(engine.get_player_count(), initial_player_count);
        assert!(engine.state.players.contains_key(&admin_id));
        assert!(!engine.state.players.contains_key(&invalid_id));
    }

    #[tokio::test]
    async fn test_admin_leave() {
        let (mut engine, admin_id) = setup_test_game();

        // Add a player and capture their messages
        let (_player_id, mut player_rx) = add_test_player_with_channel(&mut engine, "Player1");

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
        match receive_and_deserialize(&mut player_rx).await {
            GameUpdate::GameClosed { reason } => {
                assert_eq!(reason.as_ref(), "Host left the game");
            }
            other => panic!("Expected GameClosed message, got {:?}", other),
        }
        player_rx.close();
    }

    #[tokio::test]
    async fn test_start_game_twice() {
        let (mut engine, admin_id) = setup_test_game();
        // Create channel to capture admin messages
        let (admin_tx, mut admin_rx) = tokio::sync::mpsc::channel(128);
        let admin_conn_id = Uuid::new_v4();
        engine.update_player_connection(admin_id, admin_tx, admin_conn_id);
        // First start - should succeed
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartGame,
        });

        // Collect all messages from the first start
        let mut found_state_delta = false;
        let mut found_admin_next_questions = false;

        // Use a timeout to prevent infinite loop
        let timeout = tokio::time::Duration::from_millis(100);

        loop {
            // Use timeout to prevent infinite waiting
            let msg = tokio::time::timeout(
                timeout,
                receive_and_deserialize::<GameUpdate>(&mut admin_rx),
            )
            .await;

            // Break if timeout occurs (no more messages)
            if msg.is_err() {
                break;
            }

            let msg = msg.unwrap();
            match msg {
                GameUpdate::StateDelta { phase, .. } => {
                    assert_eq!(
                        phase,
                        Some(GamePhase::Score),
                        "First start should move to Score phase"
                    );
                    found_state_delta = true;
                }
                GameUpdate::AdminNextQuestions { .. } => {
                    // Expected message
                    found_admin_next_questions = true;
                }
                other => panic!("Unexpected message during first start: {:?}", other),
            }

            // Break if we've found both expected message types
            if found_state_delta && found_admin_next_questions {
                break;
            }
        }

        // Ensure we found both message types
        assert!(found_state_delta, "Did not receive StateDelta message");
        assert!(
            found_admin_next_questions,
            "Did not receive AdminNextQuestions message"
        );

        // Second start - should fail
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action: GameAction::StartGame,
        });
        // Should receive error message
        match receive_and_deserialize(&mut admin_rx).await {
            GameUpdate::Error { message } => {
                assert_eq!(
                    message.as_ref(),
                    "Can only start game from lobby or after a finished game"
                );
            }
            other => panic!("Expected Error message, got {:?}", other),
        }
        // Verify game is still in Score phase
        assert_eq!(engine.state.phase, GamePhase::Score);
    }

    #[test]
    fn test_inactivity_timeout() {
        let (mut engine, _) = setup_test_game();

        // Test case where last_lobby_message is None
        engine.state.last_lobby_message = None;
        assert!(!engine.is_finished());
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
                reason: Arc::from("finished"),
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
        assert!(
            engine
                .state
                .players
                .values()
                .all(|p| p.score == 0 && p.round_score == 0)
        );

        // fresh question sequence
        assert_eq!(engine.state.current_question_index, 0);
    }

    #[tokio::test]
    async fn test_admin_kick_player() {
        let (mut engine, admin_id) = setup_test_game();
        let (player1_id, mut player1_rx) = add_test_player_with_channel(&mut engine, "Player1");
        let (player2_id, mut player2_rx) = add_test_player_with_channel(&mut engine, "Player2");

        let now = Instant::now();

        // Admin kicks Player1
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: now,
            },
            action: GameAction::KickPlayer {
                player_name: Arc::from("Player1"),
            },
        });

        // Verify Player1 is removed; Player2 remains
        assert!(!engine.state.players.contains_key(&player1_id));
        assert!(engine.state.players.contains_key(&player2_id));

        // Verify Player1 received the Kicked message
        match receive_and_deserialize(&mut player1_rx).await {
            GameUpdate::PlayerKicked { reason } => {
                assert_eq!(reason.as_ref(), "Kicked by admin");
            }
            other => panic!("Player1 expected PlayerKicked, got {:?}", other),
        }

        // Player2 should be notified that Player1 left and receive updated scoreboard
        match receive_and_deserialize(&mut player2_rx).await {
            GameUpdate::PlayerLeft { name } => {
                assert_eq!(name.as_ref(), "Player1");
            }
            other => panic!("Player2 expected PlayerLeft, got {:?}", other),
        }
        match receive_and_deserialize(&mut player2_rx).await {
            GameUpdate::StateDelta { scoreboard, .. } => {
                assert!(scoreboard.is_some(), "Scoreboard update should be sent");
            }
            other => panic!("Player2 expected StateDelta, got {:?}", other),
        }

        player1_rx.close();
        player2_rx.close();
    }

    #[tokio::test]
    async fn test_admin_kick_nonexistent_player() {
        let (mut engine, admin_id) = setup_test_game();
        add_test_player(&mut engine, "Player1");

        // Capture messages for admin
        let (admin_tx, mut admin_rx) = tokio::sync::mpsc::channel(128);
        let admin_conn_id = Uuid::new_v4();
        engine.update_player_connection(admin_id, admin_tx, admin_conn_id);

        let initial_player_count = engine.state.players.len();
        let now = Instant::now();

        // Admin tries to kick a player who doesn't exist
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: now,
            },
            action: GameAction::KickPlayer {
                player_name: Arc::from("Ghost"),
            },
        });

        // Verify no players were removed
        assert_eq!(engine.state.players.len(), initial_player_count);

        // Verify admin received an error message
        match receive_and_deserialize(&mut admin_rx).await {
            GameUpdate::Error { message } => {
                assert!(message.contains("Player 'Ghost' not found"));
            }
            other => panic!("Admin expected Error, got {:?}", other),
        }
        admin_rx.close();
    }

    #[tokio::test]
    async fn test_admin_kick_self() {
        let (mut engine, admin_id) = setup_test_game();

        // Capture messages for admin
        let (admin_tx, mut admin_rx) = tokio::sync::mpsc::channel(128);
        let admin_conn_id = Uuid::new_v4();
        engine.update_player_connection(admin_id, admin_tx, admin_conn_id);

        let initial_player_count = engine.state.players.len();
        let now = Instant::now();

        // Admin tries to kick themselves
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: now,
            },
            action: GameAction::KickPlayer {
                player_name: Arc::from("Admin"),
            },
        });

        // Verify admin was not removed
        assert!(engine.state.players.contains_key(&admin_id));
        assert_eq!(engine.state.players.len(), initial_player_count);

        // Verify admin received an error message
        match receive_and_deserialize(&mut admin_rx).await {
            GameUpdate::Error { message } => {
                assert!(message.contains("Admin cannot kick themselves"));
            }
            other => panic!("Admin expected Error, got {:?}", other),
        }

        admin_rx.close();
    }
}
