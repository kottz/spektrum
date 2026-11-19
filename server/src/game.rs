use crate::db::QuestionSet;
use crate::question::{Color, GameQuestion, QuestionType};
use crate::uuid::Uuid;
use axum::extract::ws::Utf8Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, instrument, warn};
use ts_rs::TS;

#[derive(Debug)]
pub enum NameValidationError {
    TooShort,
    TooLong,
    InvalidCharacters,
    AlreadyTaken,
}

impl NameValidationError {
    fn to_message(&self) -> &'static str {
        match self {
            Self::TooShort => "Name must be at least 2 characters long.",
            Self::TooLong => "Name cannot be longer than 16 characters.",
            Self::InvalidCharacters => {
                "Name can only contain letters, numbers, spaces, and the symbols: _ - ."
            }
            Self::AlreadyTaken => "This name is already taken.",
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

    let char_count = name.chars().count();
    if char_count < 2 {
        return Err(NameValidationError::TooShort);
    }
    if char_count > 16 {
        return Err(NameValidationError::TooLong);
    }

    if !name.chars().all(is_allowed_name_char) {
        return Err(NameValidationError::InvalidCharacters);
    }

    if existing_names.any(|existing_name| existing_name == name) {
        return Err(NameValidationError::AlreadyTaken);
    }

    Ok(())
}

fn is_allowed_name_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ' ')
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum GamePhase {
    Lobby,
    Score,
    Question,
    GameOver,
    GameClosed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, TS)]
pub struct AdminExtraInfo {
    pub upcoming_questions: Vec<GameQuestion>,
}

/// Machine-readable reason attached to every error sent to a client, so clients
/// react to the kind of failure instead of parsing prose.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The session no longer maps to a player in a live lobby; the client must forget it.
    SessionInvalid,
    /// Another connection took over this session; the client must not reconnect.
    SessionSuperseded,
    /// The action is not permitted for this sender.
    NotAllowed,
    /// The action is not valid in the current game phase.
    InvalidPhase,
    AlreadyAnswered,
    TimeExpired,
    NoMoreQuestions,
    PlayerNotFound,
    /// The message could not be understood, or arrived before `Connect`.
    InvalidMessage,
    RateLimited,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, TS)]
#[serde(tag = "type")]
pub enum GameUpdate {
    /// A lightweight acknowledgement of connection.
    Connected {
        player_id: Uuid,
        name: Arc<str>,
        round_duration: u64,
    },
    /// A partial (delta) state update.
    /// All fields are optional; absent fields (None) are omitted from the JSON so
    /// the client can distinguish "not updated" from an explicit null.
    StateDelta {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        phase: Option<GamePhase>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        question_type: Option<QuestionType>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        question_text: Option<Arc<str>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        alternatives: Option<Vec<Arc<str>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        question_time_remaining_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        answered_player_names: Option<Vec<Arc<str>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        scoreboard: Option<Vec<(Arc<str>, i32)>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        round_scores: Option<Vec<(Arc<str>, i32)>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        consecutive_misses: Option<Vec<(Arc<str>, u32)>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        admin_extra: Option<AdminExtraInfo>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        lobby_locked: Option<bool>,
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
        code: ErrorCode,
        message: Arc<str>,
    },
    AdminInfo {
        current_question: GameQuestion,
    },
    AdminNextQuestions {
        upcoming_questions: Vec<GameQuestion>,
    },
}

#[derive(Clone, Debug)]
pub enum Recipients {
    Single(Uuid),
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
    LockLobby { locked: bool },
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
            GameAction::LockLobby { .. } => "LockLobby",
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
    pub admin: AdminConnection,
    pub join_code: Arc<str>,
    /// Present exactly while the phase is `Question`.
    pub round: Option<Round>,
    pub round_duration: u64,
    pub all_questions: Arc<Vec<GameQuestion>>,
    pub color_weights: [f64; Color::COUNT],
    pub shuffled_question_indices: Vec<usize>,
    pub current_question_index: usize,
    pub last_lobby_message: Option<Instant>,
    pub locked: bool,
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
    pub connection: Option<ClientConnection>,
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
            connection: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AdminConnection {
    pub name: Arc<str>,
    pub connection: Option<ClientConnection>,
}

/// The engine's handle to one live client socket.
///
/// Delivery is lossless or nothing: when a message cannot be queued (the client
/// stopped reading, or the socket is gone) the connection is closed rather than
/// silently skipped, so the client reconnects and resynchronises from a full
/// snapshot instead of rendering a state that no longer matches the server.
#[derive(Clone, Debug)]
pub struct ClientConnection {
    pub id: Uuid,
    tx: Sender<Utf8Bytes>,
    close: Arc<Notify>,
}

impl ClientConnection {
    pub fn new(id: Uuid, tx: Sender<Utf8Bytes>, close: Arc<Notify>) -> Self {
        Self { id, tx, close }
    }

    /// Queues `payload`; on failure closes the connection and returns `false`.
    fn send(&self, payload: Utf8Bytes) -> bool {
        match self.tx.try_send(payload) {
            Ok(()) => true,
            Err(e) => {
                warn!(connection_id = %self.id, reason = %e, "Dropping client connection: update not deliverable");
                self.close();
                false
            }
        }
    }

    /// Asks the socket task to shut this connection down.
    fn close(&self) {
        self.close.notify_one();
    }
}

/// The question being played, with this round's answer options.
#[derive(Clone, Debug)]
pub struct Round {
    pub question: GameQuestion,
    pub correct_answers: Vec<Arc<str>>,
    pub alternatives: Vec<Arc<str>>,
    pub started_at: Instant,
}

pub struct GameEngine {
    state: GameState,
}

impl GameEngine {
    pub fn new(
        admin_id: Uuid,
        join_code: Arc<str>,
        questions: Arc<Vec<GameQuestion>>,
        color_weights: [f64; Color::COUNT],
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
                    .map(|(idx, q)| (q.id, idx))
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
                admin: AdminConnection {
                    name: Arc::from("Admin"),
                    connection: None,
                },
                join_code,
                round: None,
                round_duration,
                all_questions: questions,
                color_weights,
                shuffled_question_indices: indices,
                current_question_index: 0,
                last_lobby_message: Some(Instant::now()),
                locked: false,
            },
        }
    }

    fn connection_slot(&mut self, player_id: Uuid) -> Option<&mut Option<ClientConnection>> {
        if player_id == self.state.admin_id {
            Some(&mut self.state.admin.connection)
        } else {
            self.state
                .players
                .get_mut(&player_id)
                .map(|player| &mut player.connection)
        }
    }

    /// Binds `connection` to the player. A session has at most one live
    /// connection: a previous one is told it was superseded and closed, so two
    /// tabs never both believe they are the player.
    pub fn update_player_connection(&mut self, player_id: Uuid, connection: ClientConnection) {
        let Some(slot) = self.connection_slot(player_id) else {
            return;
        };
        if let Some(previous) = slot.replace(connection)
            && let Ok(json) = serde_json::to_string(&GameUpdate::Error {
                code: ErrorCode::SessionSuperseded,
                message: "This session was opened in another window.".into(),
            })
        {
            previous.send(Utf8Bytes::from(json));
            previous.close();
        }
    }

    pub fn clear_player_connection(&mut self, player_id: Uuid, connection_id: Uuid) {
        if let Some(slot) = self.connection_slot(player_id)
            && slot.as_ref().is_some_and(|c| c.id == connection_id)
        {
            *slot = None;
        }
    }

    pub fn add_player(&mut self, player_id: Uuid, name: String) -> Result<(), NameValidationError> {
        let trimmed = name.trim();
        let existing_names = self
            .state
            .players
            .values()
            .map(|p| p.name.as_ref())
            .chain(std::iter::once(self.state.admin.name.as_ref()));
        validate_player_name(trimmed, existing_names)?;
        self.state
            .players
            .insert(player_id, PlayerState::new(Arc::from(trimmed)));
        // Everyone learns about a new player once, whatever the phase
        self.push_update(
            Recipients::All,
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
                lobby_locked: None,
            },
        );
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
            return true;
        }
        false
    }

    pub fn close_if_inactive(&mut self) {
        if self.state.phase != GamePhase::GameClosed
            && let Some(last_msg) = self.state.last_lobby_message
            && Instant::now().duration_since(last_msg) > Duration::from_secs(3600)
        {
            self.close("Lobby closed due to inactivity".into());
        }
    }

    pub fn is_full(&self) -> bool {
        self.state.players.len() >= 1024
    }

    pub fn is_closed(&self) -> bool {
        self.state.phase == GamePhase::GameClosed
    }

    pub fn is_locked(&self) -> bool {
        self.state.locked
    }

    pub fn get_lobby_stats(&self) -> (usize, usize) {
        (self.state.players.len(), self.state.current_question_index)
    }

    #[allow(clippy::type_complexity)]
    fn get_player_summary(
        &self,
    ) -> (
        Vec<(Arc<str>, i32)>,
        Vec<(Arc<str>, i32)>,
        Vec<(Arc<str>, u32)>,
    ) {
        let player_count = self.state.players.len();
        let mut scoreboard = Vec::with_capacity(player_count);
        let mut round_scores = Vec::with_capacity(player_count);
        let mut consecutive_misses = Vec::with_capacity(player_count);
        for p in self.state.players.values() {
            let name = p.name.clone();
            scoreboard.push((name.clone(), p.score));
            round_scores.push((name.clone(), p.round_score));
            consecutive_misses.push((name, p.consecutive_misses));
        }
        (scoreboard, round_scores, consecutive_misses)
    }

    fn round_length(&self) -> Duration {
        Duration::from_secs(self.state.round_duration)
    }

    /// When the current round ends on its own, if one is being played.
    pub fn round_deadline(&self) -> Option<Instant> {
        let round = self.state.round.as_ref()?;
        Some(round.started_at + self.round_length())
    }

    fn get_question_time_remaining_ms(&self, now: Instant) -> Option<u64> {
        let remaining = self.round_deadline()?.saturating_duration_since(now);
        Some(remaining.as_millis() as u64)
    }

    fn get_answered_player_names(&self) -> Vec<Arc<str>> {
        self.state
            .players
            .iter()
            .filter(|(_, p)| p.has_answered)
            .map(|(_, p)| p.name.clone())
            .collect()
    }

    fn log_game_state(&self, action: &str) {
        use std::fmt::Write;
        let total_players = self.state.players.len();
        let questions_played = self.state.current_question_index;
        let mut players_info = String::new();
        for (i, p) in self.state.players.values().enumerate() {
            if i > 0 {
                players_info.push_str(", ");
            }
            let _ = write!(players_info, "{}:{}", p.name, p.score);
        }
        info!(
            "Lobby {}: {} - {} players, {} questions played, players [{}]",
            self.state.join_code, action, total_players, questions_played, players_info
        );
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
        *player_id == self.state.admin_id || self.state.players.contains_key(player_id)
    }

    fn get_scoreboard(&self) -> Vec<(Arc<str>, i32)> {
        self.state
            .players
            .values()
            .map(|p| (p.name.clone(), p.score))
            .collect()
    }

    /// Delivers to one connection slot, dropping the connection if delivery fails.
    fn deliver(slot: &mut Option<ClientConnection>, payload: Utf8Bytes) {
        if let Some(connection) = slot
            && !connection.send(payload)
        {
            *slot = None;
        }
    }

    fn push_update(&mut self, recipients: Recipients, update: GameUpdate) {
        let json = match serde_json::to_string(&update) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize game update: {}", e);
                return;
            }
        };
        let payload = Utf8Bytes::from(json);

        match recipients {
            Recipients::Single(target) => {
                if let Some(slot) = self.connection_slot(target) {
                    Self::deliver(slot, payload);
                }
            }
            Recipients::All => {
                Self::deliver(&mut self.state.admin.connection, payload.clone());
                for player in self.state.players.values_mut() {
                    Self::deliver(&mut player.connection, payload.clone());
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
        if self.state.phase == GamePhase::GameClosed {
            // Terminal: accept nothing, and tell the sender so it forgets the session
            self.push_update(
                Recipients::Single(event.context.sender_id),
                GameUpdate::GameClosed {
                    reason: "This game has ended".into(),
                },
            );
            return;
        }
        self.state.last_lobby_message = Some(Instant::now());
        // Check admin-only actions:
        match &event.action {
            GameAction::StartGame
            | GameAction::StartRound
            | GameAction::EndRound
            | GameAction::SkipQuestion
            | GameAction::KickPlayer { .. }
            | GameAction::EndGame { .. }
            | GameAction::CloseGame { .. }
            | GameAction::LockLobby { .. }
                if event.context.sender_id != self.state.admin_id =>
            {
                debug!(
                    sender_id = %event.context.sender_id,
                    action = %event.action.kind(),
                    "Admin action denied: sender is not admin"
                );
                self.push_update(
                    Recipients::Single(event.context.sender_id),
                    GameUpdate::Error {
                        code: ErrorCode::NotAllowed,
                        message: "Admin action requires authorization".into(),
                    },
                );
                return;
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
            GameAction::LockLobby { locked } => self.handle_lock_lobby(event.context, locked),
        }
    }

    fn handle_connect(&mut self, ctx: EventContext) {
        let is_admin = ctx.sender_id == self.state.admin_id;
        let name = if is_admin {
            self.state.admin.name.clone()
        } else {
            match self.state.players.get(&ctx.sender_id) {
                Some(player) => player.name.clone(),
                None => {
                    self.push_update(
                        Recipients::Single(ctx.sender_id),
                        GameUpdate::Error {
                            code: ErrorCode::SessionInvalid,
                            message: "Player not found. Please register before connecting.".into(),
                        },
                    );
                    return;
                }
            }
        };

        // First send the initial connection acknowledgment
        self.push_update(
            Recipients::Single(ctx.sender_id),
            GameUpdate::Connected {
                player_id: ctx.sender_id,
                name,
                round_duration: self.state.round_duration,
            },
        );

        // Then send the complete current state
        let (scoreboard, round_scores, consecutive_misses) = self.get_player_summary();
        let state_update = GameUpdate::StateDelta {
            phase: Some(self.state.phase),
            question_type: self.state.round.as_ref().map(|r| r.question.question_type),
            question_text: self
                .state
                .round
                .as_ref()
                .and_then(|r| r.question.question_text.clone()),
            alternatives: self.state.round.as_ref().map(|r| r.alternatives.clone()),
            question_time_remaining_ms: self.get_question_time_remaining_ms(ctx.timestamp),
            answered_player_names: if self.state.phase == GamePhase::Question {
                Some(self.get_answered_player_names())
            } else {
                None
            },
            scoreboard: Some(scoreboard),
            round_scores: Some(round_scores),
            consecutive_misses: Some(consecutive_misses),
            admin_extra: if is_admin {
                Some(AdminExtraInfo {
                    upcoming_questions: self.get_upcoming_questions(3),
                })
            } else {
                None
            },
            lobby_locked: if is_admin {
                Some(self.state.locked)
            } else {
                None
            },
        };

        self.push_update(Recipients::Single(ctx.sender_id), state_update);

        // The admin also sees the answer to the question being played
        if is_admin && let Some(round) = &self.state.round {
            self.push_update(
                Recipients::Single(self.state.admin_id),
                GameUpdate::AdminInfo {
                    current_question: round.question.clone(),
                },
            );
        }
    }

    fn handle_leave(&mut self, ctx: EventContext) {
        if ctx.sender_id == self.state.admin_id {
            self.close("Host left the game".into());
        } else if let Some(player) = self.state.players.remove(&ctx.sender_id) {
            self.push_update(
                Recipients::All,
                GameUpdate::PlayerLeft { name: player.name },
            );
        } else {
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    code: ErrorCode::PlayerNotFound,
                    message: "Player not found".into(),
                },
            );
        }
    }

    fn handle_answer(&mut self, ctx: EventContext, answer: String) {
        if ctx.sender_id == self.state.admin_id {
            return;
        }
        let Some(round) = &self.state.round else {
            debug!(
                sender_id = %ctx.sender_id,
                current_phase = ?self.state.phase,
                "Answer rejected: wrong phase"
            );
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    code: ErrorCode::InvalidPhase,
                    message: "Not in Question phase".into(),
                },
            );
            return;
        };
        let elapsed = ctx.timestamp.saturating_duration_since(round.started_at);
        let round_length = self.round_length();
        let correct = round.correct_answers.iter().any(|a| a.as_ref() == answer);
        let (player_name, score) = {
            let player = match self.state.players.get_mut(&ctx.sender_id) {
                Some(p) => p,
                None => {
                    self.push_update(
                        Recipients::Single(ctx.sender_id),
                        GameUpdate::Error {
                            code: ErrorCode::PlayerNotFound,
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
                        code: ErrorCode::AlreadyAnswered,
                        message: "Already answered this round".into(),
                    },
                );
                return;
            }
            if elapsed > round_length {
                self.push_update(
                    Recipients::Single(ctx.sender_id),
                    GameUpdate::Error {
                        code: ErrorCode::TimeExpired,
                        message: "Time expired for this round".into(),
                    },
                );
                return;
            }
            let score_delta = if correct {
                ((5000.0 * (round_length.as_secs_f64() - elapsed.as_secs_f64())
                    / round_length.as_secs_f64())
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
                    code: ErrorCode::InvalidPhase,
                    message: "Can only start game from lobby or after a finished game".into(),
                },
            );
            return;
        }

        let from_phase = self.state.phase;

        // if we came from a finished game, zero everything out
        if self.state.phase == GamePhase::GameOver {
            info!(
                "Lobby {}: Game restarted in same lobby",
                self.state.join_code
            );
            self.reset_for_new_game();
        }

        self.state.phase = GamePhase::Score;
        debug!(from = ?from_phase, to = ?self.state.phase, "Phase transition");
        let (scoreboard, round_scores, consecutive_misses) = self.get_player_summary();
        self.push_update(
            Recipients::All,
            GameUpdate::StateDelta {
                phase: Some(GamePhase::Score),
                question_type: None,
                question_text: None,
                alternatives: None,
                question_time_remaining_ms: None,
                answered_player_names: None,
                scoreboard: Some(scoreboard),
                round_scores: Some(round_scores),
                consecutive_misses: Some(consecutive_misses),
                admin_extra: None,
                lobby_locked: None,
            },
        );
        self.send_upcoming_questions();
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
                    code: ErrorCode::InvalidPhase,
                    message: "Can only start round from score phase".into(),
                },
            );
            return;
        }
        let Some(round) = self.prepare_next_round(ctx.timestamp) else {
            debug!(
                question_index = self.state.current_question_index,
                "StartRound rejected: no more questions"
            );
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    code: ErrorCode::NoMoreQuestions,
                    message: "No more questions available. Please end the game.".into(),
                },
            );
            return;
        };

        for player in self.state.players.values_mut() {
            player.has_answered = false;
            player.answer = None;
            player.round_score = 0;
        }
        let question = round.question.clone();
        let question_type = question.question_type;
        let question_text = question.question_text.clone();
        let alternatives = round.alternatives.clone();
        self.state.round = Some(round);
        self.state.phase = GamePhase::Question;
        debug!(
            from = ?GamePhase::Score,
            to = ?GamePhase::Question,
            question_index = self.state.current_question_index,
            "Phase transition"
        );
        let (scoreboard, round_scores, consecutive_misses) = self.get_player_summary();
        self.push_update(
            Recipients::All,
            GameUpdate::StateDelta {
                phase: Some(GamePhase::Question),
                question_type: Some(question_type),
                question_text,
                alternatives: Some(alternatives),
                question_time_remaining_ms: Some(self.state.round_duration * 1000),
                answered_player_names: Some(Vec::new()),
                scoreboard: Some(scoreboard),
                round_scores: Some(round_scores),
                consecutive_misses: Some(consecutive_misses),
                admin_extra: None,
                lobby_locked: None,
            },
        );
        self.push_update(
            Recipients::Single(self.state.admin_id),
            GameUpdate::AdminInfo {
                current_question: question,
            },
        );
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
                    code: ErrorCode::InvalidPhase,
                    message: "Can only end round from question phase".into(),
                },
            );
            return;
        }
        self.end_round();
    }

    /// Ends the round once its time is up; a no-op before then or outside a round,
    /// so a timer that outlived its round cannot end a later one early.
    pub fn end_round_if_expired(&mut self, now: Instant) {
        if self
            .round_deadline()
            .is_some_and(|deadline| now >= deadline)
        {
            debug!("Round time is up");
            self.end_round();
        }
    }

    fn end_round(&mut self) {
        // AFK check: if a player has not answered, increment their consecutive misses
        for player in self.state.players.values_mut() {
            if !player.has_answered {
                player.consecutive_misses += 1;
            } else {
                player.consecutive_misses = 0;
            }
        }

        self.clear_round();
        self.state.current_question_index += 1;
        self.state.phase = GamePhase::Score;
        debug!(from = ?GamePhase::Question, to = ?GamePhase::Score, "Phase transition");
        let (scoreboard, round_scores, consecutive_misses) = self.get_player_summary();
        self.push_update(
            Recipients::All,
            GameUpdate::StateDelta {
                phase: Some(GamePhase::Score),
                question_type: None,
                question_text: None,
                alternatives: None,
                question_time_remaining_ms: None,
                answered_player_names: None,
                scoreboard: Some(scoreboard),
                round_scores: Some(round_scores),
                consecutive_misses: Some(consecutive_misses),
                admin_extra: None,
                lobby_locked: None,
            },
        );
        self.send_upcoming_questions();
    }

    fn handle_skip_question(&mut self, ctx: EventContext) {
        if self.state.phase != GamePhase::Score {
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    code: ErrorCode::InvalidPhase,
                    message: "Can only skip question during score phase".into(),
                },
            );
            return;
        }
        if self.next_question().is_none() {
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    code: ErrorCode::NoMoreQuestions,
                    message: "No more questions to skip.".into(),
                },
            );
            return;
        }
        self.state.current_question_index += 1;
        self.send_upcoming_questions();
    }

    fn handle_kick_player(&mut self, ctx: EventContext, target_player_name: Arc<str>) {
        let target_player_id = self
            .state
            .players
            .iter()
            .find(|(_, p)| p.name.as_ref() == target_player_name.as_ref())
            .map(|(id, _)| *id);
        let Some(target_player_id) = target_player_id else {
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    code: ErrorCode::PlayerNotFound,
                    message: Arc::from(format!("Player '{}' not found.", target_player_name)),
                },
            );
            return;
        };

        // Tell the player before removing them, while their connection is still bound
        self.push_update(
            Recipients::Single(target_player_id),
            GameUpdate::PlayerKicked {
                reason: Arc::from("Kicked by admin"),
            },
        );
        let Some(kicked) = self.state.players.remove(&target_player_id) else {
            return;
        };
        self.push_update(
            Recipients::All,
            GameUpdate::PlayerLeft { name: kicked.name },
        );

        let (scoreboard, round_scores, consecutive_misses) = self.get_player_summary();
        self.push_update(
            Recipients::All,
            GameUpdate::StateDelta {
                phase: None,
                question_type: None,
                question_text: None,
                alternatives: None,
                question_time_remaining_ms: None,
                answered_player_names: None,
                scoreboard: Some(scoreboard),
                round_scores: Some(round_scores),
                consecutive_misses: Some(consecutive_misses),
                admin_extra: None,
                lobby_locked: None,
            },
        );
    }

    fn handle_end_game(&mut self, ctx: EventContext, reason: Arc<str>) {
        if !matches!(
            self.state.phase,
            GamePhase::Lobby | GamePhase::Score | GamePhase::Question
        ) {
            self.push_update(
                Recipients::Single(ctx.sender_id),
                GameUpdate::Error {
                    code: ErrorCode::InvalidPhase,
                    message: "The game has already ended".into(),
                },
            );
            return;
        }
        let from_phase = self.state.phase;
        // Points are awarded as answers arrive, so ending mid-round keeps them
        self.clear_round();
        self.state.phase = GamePhase::GameOver;
        debug!(from = ?from_phase, to = ?GamePhase::GameOver, "Phase transition");
        self.log_game_state(&format!("Game ended: {}", reason));
        self.push_update(
            Recipients::All,
            GameUpdate::GameOver {
                final_scores: self.get_scoreboard(),
                reason,
            },
        );
    }

    fn handle_close_game(&mut self, _ctx: EventContext, reason: Arc<str>) {
        self.close(reason);
    }

    /// The single transition into the terminal `GameClosed` phase.
    fn close(&mut self, reason: Arc<str>) {
        let from_phase = self.state.phase;
        self.clear_round();
        self.state.phase = GamePhase::GameClosed;
        debug!(from = ?from_phase, to = ?GamePhase::GameClosed, "Phase transition");
        self.log_game_state(&format!("Game closed: {}", reason));
        self.push_update(Recipients::All, GameUpdate::GameClosed { reason });
    }

    fn clear_round(&mut self) {
        self.state.round = None;
    }

    fn send_upcoming_questions(&mut self) {
        let upcoming_questions = self.get_upcoming_questions(3);
        self.push_update(
            Recipients::Single(self.state.admin_id),
            GameUpdate::AdminNextQuestions { upcoming_questions },
        );
    }

    fn handle_lock_lobby(&mut self, ctx: EventContext, locked: bool) {
        if self.state.locked == locked {
            return;
        }
        self.state.locked = locked;
        info!(
            "Lobby {}: {} by admin",
            self.state.join_code,
            if locked { "locked" } else { "unlocked" }
        );
        self.push_update(
            Recipients::Single(ctx.sender_id),
            GameUpdate::StateDelta {
                phase: None,
                question_type: None,
                question_text: None,
                alternatives: None,
                question_time_remaining_ms: None,
                answered_player_names: None,
                scoreboard: None,
                round_scores: None,
                consecutive_misses: None,
                admin_extra: None,
                lobby_locked: Some(locked),
            },
        );
    }

    /// Picks the next question and its alternatives without touching game state,
    /// or `None` when this game's questions are used up.
    fn prepare_next_round(&self, started_at: Instant) -> Option<Round> {
        let question = self.next_question()?.clone();
        let correct_answers = question.get_correct_answer();
        let mut alternatives = question.generate_round_alternatives(&self.state.color_weights);
        for answer in &correct_answers {
            if !alternatives.contains(answer) {
                alternatives.push(answer.clone());
            }
        }
        alternatives.sort();
        alternatives.dedup();
        fastrand::shuffle(&mut alternatives);
        Some(Round {
            question,
            correct_answers,
            alternatives,
            started_at,
        })
    }

    /// The question the next round would play, if any remain.
    fn next_question(&self) -> Option<&GameQuestion> {
        self.state
            .shuffled_question_indices
            .get(self.state.current_question_index)
            .and_then(|&idx| self.state.all_questions.get(idx))
    }

    fn reset_for_new_game(&mut self) {
        // Reshuffle this lobby's own question pool, which may be a set
        fastrand::shuffle(&mut self.state.shuffled_question_indices);
        self.state.current_question_index = 0;
        self.clear_round();

        for p in self.state.players.values_mut() {
            p.score = 0;
            p.round_score = 0;
            p.has_answered = false;
            p.answer = None;
            p.consecutive_misses = 0;
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
    use crate::question::{
        Color, GameQuestion, GameQuestionOption, QuestionType, baseline_weights,
    };
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

    fn test_connection(id: Uuid, tx: tokio::sync::mpsc::Sender<Utf8Bytes>) -> ClientConnection {
        ClientConnection::new(id, tx, Arc::default())
    }

    fn setup_test_game() -> (GameEngine, Uuid) {
        let admin_id = Uuid::new_v4();
        let questions = Arc::new(create_test_questions());
        let color_weights = baseline_weights();
        let mut engine = GameEngine::new(
            admin_id,
            Arc::from("TEST"),
            questions,
            color_weights,
            None,
            30,
        );
        engine.update_player_connection(admin_id, sink_connection());

        (engine, admin_id)
    }

    /// A live connection whose messages the test does not inspect. The receiver is
    /// leaked so it outlives the engine: a dropped receiver is a dead client, and
    /// the engine (correctly) disconnects dead clients.
    /// Puts the engine mid-round on its next question, however it got here.
    fn force_round(engine: &mut GameEngine) {
        engine.state.round = engine.prepare_next_round(Instant::now());
        engine.state.phase = GamePhase::Question;
    }

    fn sink_connection() -> ClientConnection {
        let (tx, rx) = tokio::sync::mpsc::channel(1024);
        Box::leak(Box::new(rx));
        test_connection(Uuid::new_v4(), tx)
    }

    fn drain(rx: &mut Receiver<Utf8Bytes>) {
        while rx.try_recv().is_ok() {}
    }

    fn add_test_player(engine: &mut GameEngine, name: &str) -> Uuid {
        let player_id = Uuid::new_v4();
        engine.add_player(player_id, name.to_string()).unwrap();
        engine.update_player_connection(player_id, sink_connection());
        player_id
    }

    fn add_test_player_with_channel(
        engine: &mut GameEngine,
        name: &str,
    ) -> (Uuid, Receiver<Utf8Bytes>) {
        let player_id = add_test_player(engine, name);
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let conn_id = Uuid::new_v4();
        engine.update_player_connection(player_id, test_connection(conn_id, tx));
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
            let correct_answer = engine.state.round.as_ref().unwrap().correct_answers[0].clone();

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
            let round = engine.state.round.as_ref().unwrap();
            match round.question.question_type {
                QuestionType::Color => {
                    assert_eq!(round.alternatives.len(), 6);
                    assert!(
                        round
                            .alternatives
                            .iter()
                            .all(|c| c.parse::<Color>().is_ok())
                    );
                }
                QuestionType::Year => {
                    assert_eq!(round.alternatives.len(), 5);
                    assert!(round.alternatives.iter().all(|y| y.parse::<i32>().is_ok()));
                }
                _ => {
                    assert!(!round.alternatives.is_empty());
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
        let answer = engine.state.round.as_ref().unwrap().alternatives[0].clone();
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
        // Keep admin receiver alive so broadcasts don't fail
        let (admin_tx, _admin_rx) = tokio::sync::mpsc::channel(128);
        let admin_conn_id = Uuid::new_v4();
        engine.update_player_connection(admin_id, test_connection(admin_conn_id, admin_tx));

        let player_id = add_test_player(&mut engine, "Player1");
        // Stay in lobby; do not start the game
        let player_initial_state = engine.state.players.get(&player_id).unwrap().clone();
        let initial_conn_id = player_initial_state
            .connection
            .as_ref()
            .map(|c| c.id)
            .expect("connection id should be set");

        // Simulate disconnection (remove the connection)
        engine.clear_player_connection(player_id, initial_conn_id);

        // Re-add the player with a new connection
        let (player_tx, mut player_rx) = tokio::sync::mpsc::channel(128);
        let reconnect_conn_id = Uuid::new_v4();
        engine.update_player_connection(player_id, test_connection(reconnect_conn_id, player_tx));

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
            engine.state.admin.connection.is_some(),
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
            .connection
            .as_ref()
            .map(|c| c.id)
            .expect("connection id should be set");

        // Simulate disconnection (remove the connection)
        engine.clear_player_connection(player_id, initial_conn_id);

        // Re-add the player with a new connection
        let (player_tx, mut player_rx) = tokio::sync::mpsc::channel(128);
        let reconnection_id = Uuid::new_v4();
        engine.update_player_connection(player_id, test_connection(reconnection_id, player_tx));

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
        assert!(matches!(
            validate_player_name("Tab\tName", empty_names.clone()),
            Err(NameValidationError::InvalidCharacters)
        ));

        // Test allowed characters: unicode letters, digits, and `_ - .` with spaces
        for name in ["Åsa_Öberg", "José-2.0", "名前 7", "Zoë.k"] {
            assert!(
                validate_player_name(name, empty_names.clone()).is_ok(),
                "{name} should be valid"
            );
        }

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

        let engine = GameEngine::new(
            admin_id,
            Arc::from("TEST"),
            questions,
            baseline_weights(),
            Some(&question_set),
            30,
        );

        assert_eq!(engine.state.shuffled_question_indices.len(), 2);
    }

    #[tokio::test]
    async fn test_admin_reconnect() {
        let (mut engine, admin_id) = setup_test_game();

        // Create channel to capture admin messages
        let (admin_tx, mut admin_rx) = tokio::sync::mpsc::channel(128);
        let admin_conn_id = Uuid::new_v4();
        engine.update_player_connection(admin_id, test_connection(admin_conn_id, admin_tx));

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
        force_round(&mut engine);
        let test_question = engine.state.round.as_ref().unwrap().question.clone();

        // Create new channel for clean message capture
        let (admin_tx2, mut admin_rx2) = tokio::sync::mpsc::channel(128);
        let admin_conn_id2 = Uuid::new_v4();
        engine.update_player_connection(admin_id, test_connection(admin_conn_id2, admin_tx2));

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
        assert_eq!(engine.state.phase, GamePhase::GameClosed);
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
        // 3. No round should be set
        assert!(engine.state.round.is_none());
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
        assert!(engine.state.round.is_none(), "Round should be cleared");
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
        force_round(&mut engine);

        // Store state before attempting skip
        let initial_index = engine.state.current_question_index;
        let initial_question = engine.state.round.as_ref().map(|r| r.question.clone());

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
            engine.state.round.as_ref().map(|r| r.question.clone()),
            initial_question,
            "Current question should not change"
        );
    }

    #[test]
    fn test_prepare_next_round_when_exhausted() {
        let (mut engine, _) = setup_test_game();

        engine.state.current_question_index = engine.state.shuffled_question_indices.len();
        assert!(engine.prepare_next_round(Instant::now()).is_none());
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

        // Assert game was closed after admin left
        assert_eq!(
            engine.state.phase,
            GamePhase::GameClosed,
            "Game should be closed after admin leaves"
        );

        // Test answer with non-existent player
        let invalid_id = Uuid::new_v4();
        force_round(&mut engine);

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
            .connection
            .as_ref()
            .map(|c| c.id)
            .expect("connection id should be present");
        engine.clear_player_connection(player_id, conn_id);
        assert!(
            engine
                .state
                .players
                .get(&player_id)
                .and_then(|p| p.connection.clone())
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
                code: ErrorCode::InvalidMessage,
                message: "Test error".into(),
            },
        );
        assert_eq!(
            engine.state.phase, pre_update_state.phase,
            "Game state should not change after failed message"
        );

        // Test All
        engine.push_update(
            Recipients::All,
            GameUpdate::Error {
                code: ErrorCode::InvalidMessage,
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
                .and_then(|p| p.connection.clone())
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
        engine.update_player_connection(admin_id, test_connection(admin_conn_id, admin_tx));

        // Set up question phase
        force_round(&mut engine);
        let question = engine.state.round.as_ref().unwrap().question.clone();

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
        let answer = engine.state.round.as_ref().unwrap().alternatives[0].clone();
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
            .connection
            .as_ref()
            .map(|c| c.id)
            .expect("connection id should be set");
        engine.clear_player_connection(player2_id, initial_conn_id);
        let (reconnect_tx, mut reconnect_rx) = tokio::sync::mpsc::channel(128);
        let reconnect_conn_id = Uuid::new_v4();
        engine
            .update_player_connection(player2_id, test_connection(reconnect_conn_id, reconnect_tx));

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
        let (mut engine, _admin_id) = setup_test_game();
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

        // Ensure state is unchanged
        assert_eq!(engine.get_player_count(), initial_player_count);
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

        // Verify game was closed
        assert_eq!(engine.state.phase, GamePhase::GameClosed);

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
        engine.update_player_connection(admin_id, test_connection(admin_conn_id, admin_tx));
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
            GameUpdate::Error { message, .. } => {
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
        drain(&mut player1_rx);
        drain(&mut player2_rx);

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
        engine.update_player_connection(admin_id, test_connection(admin_conn_id, admin_tx));

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
            GameUpdate::Error { message, .. } => {
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
        engine.update_player_connection(admin_id, test_connection(admin_conn_id, admin_tx));

        let initial_player_count = engine.state.players.len();
        let now = Instant::now();

        // Admin tries to kick "Admin" — not found in players since admin is separate
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: now,
            },
            action: GameAction::KickPlayer {
                player_name: Arc::from("Admin"),
            },
        });

        // Verify player count unchanged
        assert_eq!(engine.state.players.len(), initial_player_count);

        // Verify admin received a "not found" error message
        match receive_and_deserialize(&mut admin_rx).await {
            GameUpdate::Error { message, .. } => {
                assert!(message.contains("not found"));
            }
            other => panic!("Admin expected Error, got {:?}", other),
        }

        admin_rx.close();
    }

    // ---- Regression tests for state-machine bugs (see docs/state-machines.md) ----

    fn admin_event(admin_id: Uuid, action: GameAction) -> GameEvent {
        GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp: Instant::now(),
            },
            action,
        }
    }

    async fn close_signalled(close: &Notify) -> bool {
        tokio::time::timeout(Duration::from_millis(50), close.notified())
            .await
            .is_ok()
    }

    #[tokio::test]
    async fn undeliverable_update_closes_the_connection() {
        let (mut engine, _admin_id) = setup_test_game();
        let player_id = Uuid::new_v4();
        engine.add_player(player_id, "Slow".into()).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let close = Arc::new(Notify::new());
        engine.update_player_connection(
            player_id,
            ClientConnection::new(Uuid::new_v4(), tx, close.clone()),
        );

        // The client never reads: the second message cannot be queued
        engine.add_player(Uuid::new_v4(), "Other1".into()).unwrap();
        engine.add_player(Uuid::new_v4(), "Other2".into()).unwrap();

        assert!(engine.state.players[&player_id].connection.is_none());
        assert!(
            close_signalled(&close).await,
            "a client that missed an update must be disconnected so it resyncs"
        );
    }

    #[tokio::test]
    async fn newer_connection_supersedes_and_closes_the_older_one() {
        let (mut engine, _admin_id) = setup_test_game();
        let player_id = Uuid::new_v4();
        engine.add_player(player_id, "Twin".into()).unwrap();
        let (old_tx, mut old_rx) = tokio::sync::mpsc::channel(16);
        let old_close = Arc::new(Notify::new());
        engine.update_player_connection(
            player_id,
            ClientConnection::new(Uuid::new_v4(), old_tx, old_close.clone()),
        );

        let new_id = Uuid::new_v4();
        engine.update_player_connection(player_id, sink_connection_with_id(new_id));

        match receive_and_deserialize(&mut old_rx).await {
            GameUpdate::Error { code, .. } => assert_eq!(code, ErrorCode::SessionSuperseded),
            other => panic!("expected SessionSuperseded, got {other:?}"),
        }
        assert!(close_signalled(&old_close).await);
        assert_eq!(
            engine.state.players[&player_id]
                .connection
                .as_ref()
                .map(|c| c.id),
            Some(new_id)
        );
    }

    fn sink_connection_with_id(id: Uuid) -> ClientConnection {
        let (tx, rx) = tokio::sync::mpsc::channel(1024);
        Box::leak(Box::new(rx));
        test_connection(id, tx)
    }

    #[test]
    fn closed_game_is_terminal() {
        let (mut engine, admin_id) = setup_test_game();
        engine.process_event(admin_event(
            admin_id,
            GameAction::CloseGame {
                reason: "done".into(),
            },
        ));
        for action in [
            GameAction::EndGame {
                reason: "revive".into(),
            },
            GameAction::StartGame,
            GameAction::StartRound,
        ] {
            engine.process_event(admin_event(admin_id, action));
            assert_eq!(engine.state.phase, GamePhase::GameClosed);
        }
    }

    #[tokio::test]
    async fn connecting_to_a_closed_game_reports_it_closed() {
        let (mut engine, admin_id) = setup_test_game();
        let (player_id, mut rx) = add_test_player_with_channel(&mut engine, "Late");
        engine.process_event(admin_event(
            admin_id,
            GameAction::CloseGame {
                reason: "done".into(),
            },
        ));
        drain(&mut rx);
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Connect,
        });
        assert!(matches!(
            receive_and_deserialize(&mut rx).await,
            GameUpdate::GameClosed { .. }
        ));
    }

    #[test]
    fn end_game_is_rejected_after_the_game_ended() {
        let (mut engine, admin_id) = setup_test_game();
        engine.process_event(admin_event(admin_id, GameAction::StartGame));
        engine.process_event(admin_event(
            admin_id,
            GameAction::EndGame {
                reason: "first".into(),
            },
        ));
        assert_eq!(engine.state.phase, GamePhase::GameOver);
        engine.process_event(admin_event(
            admin_id,
            GameAction::EndGame {
                reason: "again".into(),
            },
        ));
        assert_eq!(engine.state.phase, GamePhase::GameOver);
    }

    #[test]
    fn ending_mid_round_clears_the_round() {
        let (mut engine, admin_id) = setup_test_game();
        engine.process_event(admin_event(admin_id, GameAction::StartGame));
        engine.process_event(admin_event(admin_id, GameAction::StartRound));
        assert_eq!(engine.state.phase, GamePhase::Question);
        engine.process_event(admin_event(
            admin_id,
            GameAction::EndGame {
                reason: "stop".into(),
            },
        ));
        assert_eq!(engine.state.phase, GamePhase::GameOver);
        assert!(engine.state.round.is_none());
    }

    fn set_game(question_ids: Vec<i64>) -> (GameEngine, Uuid) {
        let admin_id = Uuid::new_v4();
        let set = QuestionSet {
            id: 1,
            question_ids,
            name: Arc::from("Set"),
        };
        let mut engine = GameEngine::new(
            admin_id,
            Arc::from("TEST"),
            Arc::new(create_test_questions()),
            baseline_weights(),
            Some(&set),
            30,
        );
        engine.update_player_connection(admin_id, sink_connection());
        (engine, admin_id)
    }

    #[test]
    fn restarting_a_set_lobby_keeps_the_set() {
        let (mut engine, admin_id) = set_game(vec![1, 2]);
        let mut pool = engine.state.shuffled_question_indices.clone();
        pool.sort_unstable();
        engine.process_event(admin_event(admin_id, GameAction::StartGame));
        engine.process_event(admin_event(
            admin_id,
            GameAction::EndGame {
                reason: "next".into(),
            },
        ));
        engine.process_event(admin_event(admin_id, GameAction::StartGame));

        let mut after = engine.state.shuffled_question_indices.clone();
        after.sort_unstable();
        assert_eq!(after, pool, "restart must reuse the lobby's question set");
    }

    #[test]
    fn exhausted_set_rejects_start_round_without_touching_round_scores() {
        let (mut engine, admin_id) = set_game(vec![1]);
        let player_id = add_test_player(&mut engine, "Player");
        engine.process_event(admin_event(admin_id, GameAction::StartGame));
        engine.process_event(admin_event(admin_id, GameAction::StartRound));
        let answer = engine.state.round.as_ref().unwrap().correct_answers[0].to_string();
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player_id,
                timestamp: Instant::now(),
            },
            action: GameAction::Answer { answer },
        });
        engine.process_event(admin_event(admin_id, GameAction::EndRound));
        let round_score = engine.state.players[&player_id].round_score;
        assert!(round_score > 0);

        engine.process_event(admin_event(admin_id, GameAction::StartRound));
        assert_eq!(engine.state.phase, GamePhase::Score);
        assert_eq!(engine.state.players[&player_id].round_score, round_score);
    }

    #[test]
    fn skip_is_rejected_when_no_questions_remain() {
        let (mut engine, admin_id) = set_game(vec![1]);
        engine.process_event(admin_event(admin_id, GameAction::StartGame));
        engine.process_event(admin_event(admin_id, GameAction::SkipQuestion));
        assert_eq!(engine.state.current_question_index, 1);
        engine.process_event(admin_event(admin_id, GameAction::SkipQuestion));
        assert_eq!(engine.state.current_question_index, 1);
    }

    #[tokio::test]
    async fn admin_is_told_when_questions_run_out() {
        let (mut engine, admin_id) = set_game(vec![1]);
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        engine.update_player_connection(admin_id, test_connection(Uuid::new_v4(), tx));
        engine.process_event(admin_event(admin_id, GameAction::StartGame));
        engine.process_event(admin_event(admin_id, GameAction::SkipQuestion));
        let mut last_upcoming = None;
        while let Ok(payload) = rx.try_recv() {
            if let Ok(GameUpdate::AdminNextQuestions { upcoming_questions }) =
                serde_json::from_str(payload.as_str())
            {
                last_upcoming = Some(upcoming_questions);
            }
        }
        assert_eq!(last_upcoming, Some(Vec::new()));
    }

    #[tokio::test]
    async fn kick_keeps_round_scores_in_the_update() {
        let (mut engine, admin_id) = setup_test_game();
        let _ = add_test_player(&mut engine, "Victim");
        let (_id, mut rx) = add_test_player_with_channel(&mut engine, "Watcher");
        drain(&mut rx);
        engine.process_event(admin_event(
            admin_id,
            GameAction::KickPlayer {
                player_name: "Victim".into(),
            },
        ));
        let mut saw_round_scores = false;
        while let Ok(payload) = rx.try_recv() {
            if let Ok(GameUpdate::StateDelta { round_scores, .. }) =
                serde_json::from_str(payload.as_str())
            {
                saw_round_scores = round_scores.is_some();
            }
        }
        assert!(
            saw_round_scores,
            "a kick must not erase everyone's round scores"
        );
    }

    #[tokio::test]
    async fn players_learn_about_mid_game_joiners() {
        let (mut engine, admin_id) = setup_test_game();
        let (_id, mut rx) = add_test_player_with_channel(&mut engine, "Early");
        engine.process_event(admin_event(admin_id, GameAction::StartGame));
        engine.process_event(admin_event(admin_id, GameAction::StartRound));
        drain(&mut rx);
        engine.add_player(Uuid::new_v4(), "Late".into()).unwrap();
        match receive_and_deserialize(&mut rx).await {
            GameUpdate::StateDelta {
                scoreboard: Some(scoreboard),
                ..
            } => assert!(scoreboard.iter().any(|(name, _)| name.as_ref() == "Late")),
            other => panic!("expected scoreboard update, got {other:?}"),
        }
    }

    #[test]
    fn answer_just_past_the_deadline_is_rejected() {
        let (mut engine, admin_id) = setup_test_game();
        let player_id = add_test_player(&mut engine, "Slowpoke");
        engine.process_event(admin_event(admin_id, GameAction::StartGame));
        engine.process_event(admin_event(admin_id, GameAction::StartRound));
        let start = engine.state.round.as_ref().unwrap().started_at;
        let answer = engine.state.round.as_ref().unwrap().correct_answers[0].to_string();
        engine.process_event(GameEvent {
            context: EventContext {
                sender_id: player_id,
                timestamp: start + Duration::from_millis(30_500),
            },
            action: GameAction::Answer { answer },
        });
        assert!(!engine.state.players[&player_id].has_answered);
    }

    fn admin_event_at(admin_id: Uuid, action: GameAction, timestamp: Instant) -> GameEvent {
        GameEvent {
            context: EventContext {
                sender_id: admin_id,
                timestamp,
            },
            action,
        }
    }

    #[test]
    fn round_ends_by_itself_when_time_is_up() {
        let (mut engine, admin_id) = setup_test_game();
        let player_id = add_test_player(&mut engine, "Sleepy");
        engine.process_event(admin_event(admin_id, GameAction::StartGame));
        engine.process_event(admin_event(admin_id, GameAction::StartRound));
        let deadline = engine.round_deadline().unwrap();

        engine.end_round_if_expired(deadline - Duration::from_millis(1));
        assert_eq!(engine.state.phase, GamePhase::Question);

        engine.end_round_if_expired(deadline);
        assert_eq!(engine.state.phase, GamePhase::Score);
        assert!(engine.state.round.is_none());
        assert_eq!(engine.state.current_question_index, 1);
        assert_eq!(engine.state.players[&player_id].consecutive_misses, 1);
    }

    #[test]
    fn a_timer_that_outlived_its_round_cannot_end_the_next_one() {
        let (mut engine, admin_id) = setup_test_game();
        let t0 = Instant::now();
        engine.process_event(admin_event_at(admin_id, GameAction::StartGame, t0));
        engine.process_event(admin_event_at(admin_id, GameAction::StartRound, t0));
        let first_deadline = engine.round_deadline().unwrap();
        engine.process_event(admin_event_at(admin_id, GameAction::EndRound, t0));
        let later = t0 + Duration::from_secs(5);
        engine.process_event(admin_event_at(admin_id, GameAction::StartRound, later));

        engine.end_round_if_expired(first_deadline);
        assert_eq!(engine.state.phase, GamePhase::Question);
        assert_eq!(engine.state.current_question_index, 1);
    }

    #[test]
    fn expiry_outside_a_round_does_nothing() {
        let (mut engine, admin_id) = setup_test_game();
        engine.process_event(admin_event(admin_id, GameAction::StartGame));
        engine.end_round_if_expired(Instant::now() + Duration::from_secs(3600));
        assert_eq!(engine.state.phase, GamePhase::Score);
        assert_eq!(engine.state.current_question_index, 0);
    }
}
