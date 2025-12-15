use crate::db::{DbError, StoredData};
use crate::game::{
    EventContext, GameAction, GameEngine, GameEvent, GameUpdate, NameValidationError,
};
use crate::question::{QuestionError, QuestionStore};
use crate::uuid::Uuid;
use axum::extract::Path;
use axum::extract::ws::Utf8Bytes;
use axum::http::StatusCode;
use axum::{
    Json,
    extract::Multipart,
    extract::State,
    extract::WebSocketUpgrade,
    extract::ws::{Message, WebSocket},
    response::IntoResponse,
};
use bytes::Bytes;
use chrono::{DateTime, SecondsFormat, Utc};
use dashmap::DashMap;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use thiserror::Error;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::task::JoinHandle;
use tracing::{Instrument, Span, debug, error, info, info_span, trace, warn};

const HEARTBEAT_BYTE: u8 = 0x42;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Database error: {0}")]
    Database(String),
    #[error("Lobby error: {0}")]
    Lobby(String),
    #[error("No more join codes error")]
    OutOfJoinCodes,
    #[error("Unsupported media type")]
    UnsupportedMediaType,
    #[error("Bad request: {0}")]
    BadRequest(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    details: Option<String>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error, details) = match self {
            ApiError::Validation(message) => {
                (StatusCode::BAD_REQUEST, "Validation error", Some(message))
            }
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized", None),
            ApiError::Database(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
                Some(message),
            ),
            ApiError::Lobby(message) => (StatusCode::BAD_REQUEST, "Lobby error", Some(message)),
            ApiError::OutOfJoinCodes => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "No more join codes error",
                None,
            ),
            ApiError::UnsupportedMediaType => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Unsupported media type",
                None,
            ),
            ApiError::BadRequest(message) => {
                (StatusCode::BAD_REQUEST, "Bad request", Some(message))
            }
        };

        let body = Json(ErrorResponse {
            error: error.into(),
            details,
        });
        (status, body).into_response()
    }
}

impl From<DbError> for ApiError {
    fn from(err: DbError) -> Self {
        ApiError::Database(err.to_string())
    }
}

impl From<NameValidationError> for ApiError {
    fn from(err: NameValidationError) -> Self {
        ApiError::Validation(err.to_string())
    }
}

impl From<QuestionError> for ApiError {
    fn from(err: QuestionError) -> Self {
        ApiError::Database(err.to_string())
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Connect { session_token: String },
    Leave,
    Answer { answer: String },
    AdminAction { action: AdminAction },
}

impl ClientMessage {
    /// Returns the variant name without any payload data (safe for logging)
    pub fn kind(&self) -> &'static str {
        match self {
            ClientMessage::Connect { .. } => "Connect",
            ClientMessage::Leave => "Leave",
            ClientMessage::Answer { .. } => "Answer",
            ClientMessage::AdminAction { .. } => "AdminAction",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum AdminAction {
    StartGame,
    StartRound,
    EndRound,
    SkipQuestion,
    KickPlayer { player_name: String },
    EndGame { reason: String },
    CloseGame { reason: String },
}

impl AdminAction {
    /// Returns the variant name without any payload data (safe for logging)
    pub fn kind(&self) -> &'static str {
        match self {
            AdminAction::StartGame => "StartGame",
            AdminAction::StartRound => "StartRound",
            AdminAction::EndRound => "EndRound",
            AdminAction::SkipQuestion => "SkipQuestion",
            AdminAction::KickPlayer { .. } => "KickPlayer",
            AdminAction::EndGame { .. } => "EndGame",
            AdminAction::CloseGame { .. } => "CloseGame",
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub lobbies: Arc<DashMap<String, GameEngine>>,
    pub store: Arc<QuestionStore>,
    pub admin_passwords: Vec<String>,
}

impl AppState {
    pub fn new(question_manager: QuestionStore, admin_passwords: Vec<String>) -> Self {
        let state = Self {
            lobbies: Arc::new(DashMap::new()),
            store: Arc::new(question_manager),
            admin_passwords,
        };

        {
            let lobbies = state.lobbies.clone();
            tokio::spawn(
                async move {
                    cleanup_lobbies(lobbies).await;
                }
                .instrument(info_span!(target: "maintenance", "lobby_cleanup")),
            );
        }

        state
    }

    fn generate_join_code(&self) -> Result<String, ApiError> {
        // First try 6-digit codes
        for _ in 0..10_000 {
            let code = format!("{:06}", fastrand::u32(0..1_000_000));
            if !self.lobbies.contains_key(&code) {
                return Ok(code);
            }
        }

        // If many collisions, escalate to 7 digits
        for _ in 0..1_000_000 {
            let code = format!("{:07}", fastrand::u32(0..10_000_000));
            if !self.lobbies.contains_key(&code) {
                return Ok(code);
            }
        }

        Err(ApiError::OutOfJoinCodes)
    }
}

#[derive(Debug, Serialize)]
pub struct SetInfo {
    pub id: i64,
    pub name: Arc<str>,
    pub question_count: usize,
}

#[derive(Debug, Serialize)]
pub struct ListSetsResponse {
    pub num_questions: usize,
    pub sets: Vec<SetInfo>,
}

pub async fn list_sets(state: &AppState) -> Result<ListSetsResponse, ApiError> {
    let snap = state.store.snapshot();
    let num_questions = snap.questions.len();
    let sets = &*snap.sets;

    let sets_info: Vec<SetInfo> = sets
        .iter()
        .map(|set| SetInfo {
            id: set.id,
            name: set.name.clone(),
            question_count: set.question_ids.len(),
        })
        .collect();

    Ok(ListSetsResponse {
        num_questions,
        sets: sets_info,
    })
}

#[derive(Debug, Deserialize)]
pub struct CreateLobbyRequest {
    pub round_duration: Option<u64>,
    pub set_id: Option<i64>,
}

#[derive(Debug, serde::Serialize, PartialEq)]
pub struct CreateLobbyResponse {
    pub player_id: Uuid,
    pub join_code: String,
    pub session_token: String,
}

pub async fn create_lobby(
    state: &AppState,
    req: CreateLobbyRequest,
) -> Result<CreateLobbyResponse, ApiError> {
    let round_duration = req.round_duration.unwrap_or(60);
    if round_duration < 10 {
        return Err(ApiError::Validation(
            "Round duration must be at least 10 seconds".into(),
        ));
    }

    let snap = state.store.snapshot();
    let questions = snap.questions.clone();
    let sets = &*snap.sets;
    let selected_set = if let Some(set_id) = req.set_id {
        Some(
            sets.iter()
                .find(|set| set.id == set_id)
                .ok_or_else(|| ApiError::Validation(format!("Set with id {} not found", set_id)))?,
        )
    } else {
        None
    };

    let admin_id = Uuid::new_v4();
    let join_code = state.generate_join_code()?;

    let mut engine = GameEngine::new(
        admin_id,
        questions,
        snap.color_weights,
        selected_set,
        round_duration,
    );
    engine.add_player(admin_id, "Admin".into())?;
    trace!("Creating new lobby {}", join_code);

    state.lobbies.insert(join_code.clone(), engine);
    let session_token = format!("{}:{}", join_code, admin_id);

    info!(
        "Lobby created with join code: {} (round_duration: {}s, set: {})",
        join_code,
        round_duration,
        selected_set
            .map(|s| s.name.as_ref())
            .unwrap_or("all questions")
    );

    Ok(CreateLobbyResponse {
        player_id: admin_id,
        join_code,
        session_token,
    })
}

#[derive(Debug, Deserialize)]
pub struct JoinLobbyRequest {
    pub join_code: String,
    pub name: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct JoinLobbyResponse {
    pub player_id: Uuid,
    pub session_token: String,
}

pub async fn join_lobby(
    state: &AppState,
    req: JoinLobbyRequest,
) -> Result<JoinLobbyResponse, ApiError> {
    let join_code = req.join_code.trim();

    debug!(target: "lock", lobby_key = %join_code, "acquiring_lobby_lock");
    let t0 = Instant::now();

    let mut engine = match state.lobbies.get_mut(join_code) {
        Some(engine) => {
            let dt = t0.elapsed();
            debug!(
                target: "lock",
                lobby_key = %join_code,
                duration_us = dt.as_micros() as u64,
                "lobby_lock_acquired"
            );
            engine
        }
        None => {
            debug!(target: "lock", lobby_key = %join_code, "lobby_not_found");
            return Err(ApiError::Lobby("Invalid join code.".into()));
        }
    };

    if engine.is_full() {
        return Err(ApiError::Lobby("Lobby is full.".into()));
    }

    let new_player_id = Uuid::new_v4();
    engine.add_player(new_player_id, req.name)?;
    Ok(JoinLobbyResponse {
        player_id: new_player_id,
        session_token: format!("{}:{}", join_code, new_player_id),
    })
}

#[derive(Debug, Deserialize)]
pub struct GetStoredDataRequest {
    password: String,
}

pub async fn get_stored_data(
    state: &AppState,
    req: GetStoredDataRequest,
) -> Result<StoredData, ApiError> {
    if !state.admin_passwords.contains(&req.password) {
        return Err(ApiError::Unauthorized);
    }
    let stored_data = state.store.get_stored_data().await?;
    Ok(stored_data)
}

#[derive(Debug, Deserialize)]
pub struct SetStoredDataRequest {
    password: String,
    stored_data: StoredData,
}

pub async fn set_stored_data(
    state: &AppState,
    req: SetStoredDataRequest,
) -> Result<StoredData, ApiError> {
    if !state.admin_passwords.contains(&req.password) {
        return Err(ApiError::Unauthorized);
    }
    state.store.backup_stored_data().await?;
    state.store.set_stored_data(req.stored_data).await?;
    state.store.reload().await?;
    let data = state.store.get_stored_data().await?;
    Ok(data)
}

#[derive(Debug, Serialize, PartialEq)]
pub struct UploadCharacterImageResponse {
    image_url: String,
}

pub async fn upload_character_image(
    state: &AppState,
    character_name: String,
    password: Option<String>,
    image_data: Option<Bytes>,
) -> Result<UploadCharacterImageResponse, ApiError> {
    let password = password.ok_or(ApiError::Unauthorized)?;
    if !state.admin_passwords.contains(&password) {
        return Err(ApiError::Unauthorized);
    }
    let image_data = image_data.ok_or(ApiError::BadRequest("Missing image file".to_string()))?;
    let url = state
        .store
        .store_character_image(&character_name, &image_data)
        .await?;
    Ok(UploadCharacterImageResponse { image_url: url })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub player_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CheckSessionsRequest {
    pub sessions: Vec<SessionInfo>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ValidSessionInfo {
    pub player_id: Uuid,
    pub last_update: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CheckSessionsResponse {
    pub valid_sessions: Vec<ValidSessionInfo>,
}

pub async fn check_sessions(
    state: &AppState,
    req: CheckSessionsRequest,
) -> Result<CheckSessionsResponse, ApiError> {
    let mono_now = Instant::now();
    let sys_now = SystemTime::now();

    let valid_sessions: Vec<ValidSessionInfo> = req
        .sessions
        .into_iter()
        .filter_map(|session| {
            let lobby = state
                .lobbies
                .iter()
                .find(|entry| entry.value().has_player(&session.player_id))?;

            if lobby.value().is_finished() {
                return None;
            }
            let last_update = lobby.value().last_update()?;
            let duration = mono_now.duration_since(last_update);
            let system_time = sys_now.checked_sub(duration)?;
            let last_update_iso =
                DateTime::<Utc>::from(system_time).to_rfc3339_opts(SecondsFormat::Millis, true);

            Some(ValidSessionInfo {
                player_id: session.player_id,
                last_update: last_update_iso,
            })
        })
        .collect();

    Ok(CheckSessionsResponse { valid_sessions })
}

pub async fn list_sets_handler(
    State(state): State<AppState>,
) -> Result<Json<ListSetsResponse>, ApiError> {
    let response = list_sets(&state).await?;
    Ok(Json(response))
}

pub async fn create_lobby_handler(
    State(state): State<AppState>,
    Json(req): Json<CreateLobbyRequest>,
) -> Result<Json<CreateLobbyResponse>, ApiError> {
    let response = create_lobby(&state, req).await?;
    Ok(Json(response))
}

pub async fn join_lobby_handler(
    State(state): State<AppState>,
    Json(req): Json<JoinLobbyRequest>,
) -> Result<Json<JoinLobbyResponse>, ApiError> {
    let response = join_lobby(&state, req).await?;
    Ok(Json(response))
}

pub async fn get_stored_data_handler(
    State(state): State<AppState>,
    Json(req): Json<GetStoredDataRequest>,
) -> Result<Json<StoredData>, ApiError> {
    let response = get_stored_data(&state, req).await?;
    Ok(Json(response))
}

pub async fn set_stored_data_handler(
    State(state): State<AppState>,
    Json(req): Json<SetStoredDataRequest>,
) -> Result<Json<StoredData>, ApiError> {
    let response = set_stored_data(&state, req).await?;
    Ok(Json(response))
}

pub async fn upload_character_image_handler(
    State(state): State<AppState>,
    Path(character_name): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<UploadCharacterImageResponse>, ApiError> {
    let mut password = None;
    let mut image_data = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
    {
        match field.name().unwrap_or_default() {
            "password" => {
                password = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(e.to_string()))?,
                )
            }
            "image" => {
                if !field
                    .content_type()
                    .unwrap_or("")
                    .eq_ignore_ascii_case("image/avif")
                {
                    return Err(ApiError::UnsupportedMediaType);
                }
                image_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| ApiError::BadRequest(e.to_string()))?,
                );
            }
            _ => continue,
        }
    }
    let response = upload_character_image(&state, character_name, password, image_data).await?;
    Ok(Json(response))
}

pub async fn check_sessions_handler(
    State(state): State<AppState>,
    Json(req): Json<CheckSessionsRequest>,
) -> Result<Json<CheckSessionsResponse>, ApiError> {
    let response = check_sessions(&state, req).await?;
    Ok(Json(response))
}

struct WsConnection {
    player_id: Option<Uuid>,
    lobby_key: Option<String>,
    recent_message_count: usize,
    count_reset_time: Instant,
    connection_id: Uuid,
    /// The connection-level tracing span, stored for explicit field recording
    conn_span: Span,
}

impl WsConnection {
    fn new(upgrade_request_id: Option<u64>) -> Self {
        let connection_id = Uuid::new_v4();
        let conn_span = info_span!(
            target: "ws",
            "ws_connection",
            connection_id = %connection_id,
            upgrade_request_id = upgrade_request_id,
            player_id = tracing::field::Empty,
            lobby_key = tracing::field::Empty,
        );
        Self {
            player_id: None,
            lobby_key: None,
            recent_message_count: 0,
            count_reset_time: Instant::now(),
            connection_id,
            conn_span,
        }
    }
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    // Generate a unique ID for this upgrade request that links HTTP → WebSocket
    // This will appear in both the HTTP request span and the WS connection span
    let upgrade_request_id = fastrand::u64(..);
    debug!(
        target: "ws",
        upgrade_request_id = upgrade_request_id,
        "WebSocket upgrade requested"
    );

    ws.on_upgrade(move |socket| handle_socket(socket, state, Some(upgrade_request_id)))
}

async fn handle_socket(socket: WebSocket, state: AppState, upgrade_request_id: Option<u64>) {
    let (ws_tx, mut ws_rx) = socket.split();

    let (text_tx, text_rx) = channel::<Utf8Bytes>(128);
    let (bin_tx, bin_rx) = channel::<Bytes>(128);

    let mut conn = WsConnection::new(upgrade_request_id);
    // Clone the span for use in .instrument() - conn retains ownership for field recording
    let conn_span = conn.conn_span.clone();

    // Ensure the sender task inherits the connection span as its parent
    let send_task = {
        let _guard = conn_span.enter();
        spawn_sender_task(ws_tx, text_rx, bin_rx, conn.connection_id)
    };

    async {
        while let Some(Ok(msg)) = ws_rx.next().await {
            let (msg_kind, size_bytes) = get_message_info(&msg);
            let msg_span = info_span!(
                target: "ws",
                "ws_message",
                msg_kind = %msg_kind,
                size_bytes = size_bytes,
            );

            let result = async { handle_message(msg, &mut conn, &state, &text_tx, &bin_tx).await }
                .instrument(msg_span)
                .await;

            if result.is_err() {
                break;
            }
        }

        handle_disconnect(&conn, &state).await;
        send_task.abort();
    }
    .instrument(conn_span)
    .await;
}

/// Returns (message_kind, size_bytes) for logging purposes
fn get_message_info(msg: &Message) -> (&'static str, usize) {
    match msg {
        Message::Text(text) => ("text", text.len()),
        Message::Binary(data) => {
            if data.as_ref() == [HEARTBEAT_BYTE] {
                ("binary_heartbeat", data.len())
            } else {
                ("binary", data.len())
            }
        }
        Message::Ping(data) => ("ping", data.len()),
        Message::Pong(data) => ("pong", data.len()),
        Message::Close(_) => ("close", 0),
    }
}

fn spawn_sender_task(
    mut ws_tx: SplitSink<WebSocket, Message>,
    mut text_rx: Receiver<Utf8Bytes>,
    mut bin_rx: Receiver<Bytes>,
    connection_id: Uuid,
) -> JoinHandle<()> {
    let sender_span = info_span!(
        target: "ws",
        "ws_sender_task",
        connection_id = %connection_id,
        channel = "ws_tx",
    );

    tokio::spawn(
        async move {
            let mut ping_interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                tokio::select! {
                    _ = ping_interval.tick() => {
                        if ws_tx.send(Message::Ping(vec![].into())).await.is_err() { break; }
                    }
                    Some(msg) = text_rx.recv() => {
                        if ws_tx.send(Message::Text(msg)).await.is_err() { break; }
                    }
                    Some(msg) = bin_rx.recv() => {
                        if ws_tx.send(Message::Binary(msg)).await.is_err() { break; }
                    }
                    else => break,
                }
            }
        }
        .instrument(sender_span),
    )
}

async fn handle_message(
    msg: Message,
    conn: &mut WsConnection,
    state: &AppState,
    text_tx: &Sender<Utf8Bytes>,
    bin_tx: &Sender<Bytes>,
) -> Result<(), ()> {
    // Rate limit check
    let now = Instant::now();

    if now.duration_since(conn.count_reset_time) > Duration::from_secs(1) {
        conn.recent_message_count = 0;
        conn.count_reset_time = now;
    }

    conn.recent_message_count += 1;
    if conn.recent_message_count > 30 {
        error!(
            target: "ws",
            player_id = ?conn.player_id,
            connection_id = %conn.connection_id,
            "Rate limit exceeded, closing connection"
        );
        send_error_to_client(
            text_tx,
            "Rate limit exceeded. Closing connection".to_string(),
            "rate_limit",
        );
        return Err(());
    }

    // Handle Message
    match msg {
        Message::Text(text) => {
            let client_msg: ClientMessage = match serde_json::from_str(&text) {
                Ok(msg) => msg,
                Err(e) => {
                    // Log parse error without raw payload (security: avoid logging user data)
                    debug!(
                        target: "ws",
                        connection_id = %conn.connection_id,
                        error = %e,
                        size_bytes = text.len(),
                        "Failed to parse client message"
                    );
                    send_error_to_client(
                        text_tx,
                        format!("Invalid message format: {}", e),
                        "json_parse",
                    );
                    return Ok(());
                }
            };

            // Log message type without payload (safe for logging)
            trace!(
                target: "ws",
                client_msg_type = %client_msg.kind(),
                "Processing client message"
            );

            if let ClientMessage::Connect { session_token } = client_msg {
                handle_connect(session_token, conn, state, text_tx).await;
            } else if conn.player_id.is_some() {
                dispatch_game_action(client_msg, conn, state).await;
            } else {
                send_error_to_client(text_tx, "Must connect first.".to_string(), "not_connected");
            }
        }
        Message::Binary(payload) if payload.as_ref() == [HEARTBEAT_BYTE] => {
            if bin_tx.try_send(payload).is_err() {
                warn!("Heartbeat channel full");
            }
        }
        Message::Pong(_) => trace!("Received pong from client"),
        Message::Close(_) => {
            if let Some(pid) = conn.player_id {
                trace!("Client initiated close for player {}", pid);
            }
            return Err(());
        }
        _ => {}
    }
    Ok(())
}

async fn handle_connect(
    session_token: String,
    conn: &mut WsConnection,
    state: &AppState,
    tx: &Sender<Utf8Bytes>,
) {
    let (code, pid_str) = match session_token.split_once(':') {
        Some(parts) => parts,
        None => {
            send_error_to_client(
                tx,
                "Invalid session token format.".to_string(),
                "connect_token_parse",
            );
            return;
        }
    };

    let player_id = match pid_str.parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => {
            send_error_to_client(
                tx,
                "Invalid player id in session token.".to_string(),
                "connect_token_invalid",
            );
            return;
        }
    };

    debug!(target: "lock", lobby_key = %code, "acquiring_lobby_lock");
    let t0 = Instant::now();

    let mut engine = match state.lobbies.get_mut(code) {
        Some(engine) => {
            let dt = t0.elapsed();
            debug!(
                target: "lock",
                lobby_key = %code,
                duration_us = dt.as_micros() as u64,
                "lobby_lock_acquired"
            );
            engine
        }
        None => {
            debug!(target: "lock", lobby_key = %code, "lobby_not_found");
            send_error_to_client(
                tx,
                "Lobby not found for session token.".to_string(),
                "connect_lobby_not_found",
            );
            return;
        }
    };

    if !engine.has_player(&player_id) {
        send_error_to_client(
            tx,
            "Player not found in lobby. Please join again.".to_string(),
            "connect_player_not_found",
        );
        return;
    }

    engine.update_player_connection(player_id, tx.clone(), conn.connection_id);
    conn.player_id = Some(player_id);
    conn.lobby_key = Some(code.to_string());

    // Record player_id and lobby_key in the connection span (explicit, future-proof)
    conn.conn_span
        .record("player_id", tracing::field::display(&player_id));
    conn.conn_span.record("lobby_key", code);

    debug!(
        target: "ws",
        %player_id,
        lobby_key = %code,
        "Player connected to lobby"
    );

    let event = GameEvent {
        context: EventContext {
            sender_id: player_id,
            timestamp: Instant::now(),
        },
        action: GameAction::Connect,
    };
    engine.process_event(event);
}

async fn dispatch_game_action(msg: ClientMessage, conn: &WsConnection, state: &AppState) {
    let Some(player_id) = conn.player_id else {
        error!("dispatch_game_action called without player_id");
        return;
    };
    let Some(lobby_key) = conn.lobby_key.as_ref() else {
        error!("dispatch_game_action called without lobby_key");
        return;
    };

    debug!(target: "lock", %lobby_key, "acquiring_lobby_lock");
    let t0 = Instant::now();

    let Some(mut engine) = state.lobbies.get_mut(lobby_key) else {
        debug!(target: "lock", %lobby_key, "lobby_not_found");
        return;
    };

    let dt = t0.elapsed();
    debug!(
        target: "lock",
        %lobby_key,
        duration_us = dt.as_micros() as u64,
        "lobby_lock_acquired"
    );

    let action = match msg {
        ClientMessage::Leave => GameAction::Leave,
        ClientMessage::Answer { answer } => GameAction::Answer { answer },
        ClientMessage::AdminAction { action } => {
            debug!(
                target: "ws",
                admin_action_type = %action.kind(),
                %player_id,
                %lobby_key,
                "Processing admin action"
            );
            match action {
                AdminAction::StartGame => GameAction::StartGame,
                AdminAction::StartRound => GameAction::StartRound,
                AdminAction::EndRound => GameAction::EndRound,
                AdminAction::SkipQuestion => GameAction::SkipQuestion,
                AdminAction::KickPlayer { player_name } => GameAction::KickPlayer {
                    player_name: Arc::from(player_name),
                },
                AdminAction::EndGame { reason } => GameAction::EndGame {
                    reason: Arc::from(reason),
                },
                AdminAction::CloseGame { reason } => GameAction::CloseGame {
                    reason: Arc::from(reason),
                },
            }
        }
        _ => return, // Connect is handled separately
    };
    let event = GameEvent {
        context: EventContext {
            sender_id: player_id,
            timestamp: Instant::now(),
        },
        action,
    };
    engine.process_event(event);
}

async fn handle_disconnect(conn: &WsConnection, state: &AppState) {
    if let (Some(player_id), Some(lobby_key)) = (conn.player_id, conn.lobby_key.as_ref()) {
        trace!(
            "Disconnecting player {} from lobby {}",
            player_id, lobby_key
        );

        debug!(target: "lock", %lobby_key, "acquiring_lobby_lock");
        let t0 = Instant::now();

        if let Some(mut engine) = state.lobbies.get_mut(lobby_key) {
            let dt = t0.elapsed();
            debug!(
                target: "lock",
                %lobby_key,
                duration_us = dt.as_micros() as u64,
                "lobby_lock_acquired"
            );
            engine.clear_player_connection(player_id, conn.connection_id);
        } else {
            debug!(target: "lock", %lobby_key, "lobby_not_found");
        }
    }
}

fn send_error_to_client(tx: &Sender<Utf8Bytes>, message: String, context: &str) {
    let error_update = GameUpdate::Error {
        message: Arc::from(message),
    };
    if let Ok(json) = serde_json::to_string(&error_update)
        && tx.try_send(Utf8Bytes::from(json)).is_err()
    {
        error!("Failed to send '{}' error to client channel", context);
    }
}

async fn cleanup_lobbies(lobbies: Arc<DashMap<String, GameEngine>>) {
    let mut tick = tokio::time::interval(Duration::from_secs(60));
    loop {
        tick.tick().await;

        // Close inactive lobbies and notify players before cleanup
        for mut entry in lobbies.iter_mut() {
            entry.value_mut().close_if_inactive();
        }

        let finished_lobby_ids: Vec<String> = lobbies
            .iter()
            .filter(|entry| entry.value().is_finished())
            .map(|entry| entry.key().clone())
            .collect();

        for lobby_id in &finished_lobby_ids {
            if let Some((_, engine)) = lobbies.remove(lobby_id) {
                let (total_players, questions_played, player_scores) = engine.get_lobby_stats();
                let players_info: Vec<String> = player_scores
                    .iter()
                    .map(|(name, score)| format!("{}:{}", name, score))
                    .collect();
                info!(
                    "Lobby closed: {} with {} players, {} questions played, players [{}]",
                    lobby_id,
                    total_players,
                    questions_played,
                    players_info.join(", ")
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageConfig;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    async fn setup_test_state() -> (AppState, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("questions.json");

        // Create a dummy questions file with minimal data to avoid NoQuestions error
        let dummy_json = r#"{
            "media": [{"id": 1, "title": "Test Song", "artist": "Test Artist", "release_year": null, "spotify_uri": null, "youtube_id": "test123"}],
            "characters": [],
            "questions": [{"id": 1, "media_id": 1, "question_type": "color", "question_text": null, "image_url": null, "is_active": true}],
            "options": [{"id": 1, "question_id": 1, "option_text": "Red", "is_correct": true}],
            "sets": []
        }"#;
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "{}", dummy_json).unwrap();

        let storage_config = StorageConfig::Filesystem {
            base_path: dir.path().to_path_buf(),
            file_path: "questions.json".to_string(),
        };

        let store = QuestionStore::new(&storage_config).await.unwrap();
        let state = AppState::new(store, vec!["password".to_string()]);
        (state, dir)
    }

    #[tokio::test]
    async fn test_create_lobby_logic() {
        let (state, _dir) = setup_test_state().await;
        let req = CreateLobbyRequest {
            round_duration: Some(120),
            set_id: None,
        };

        let res = create_lobby(&state, req).await.unwrap();

        assert_eq!(state.lobbies.len(), 1);
        let lobby = state.lobbies.get(&res.join_code).unwrap();

        assert_eq!(lobby.get_admin_id(), res.player_id);
        assert_eq!(lobby.get_round_duration(), 120);
        assert_eq!(
            res.session_token,
            format!("{}:{}", res.join_code, res.player_id)
        );
    }

    #[tokio::test]
    async fn test_create_lobby_invalid_duration() {
        let (state, _dir) = setup_test_state().await;
        let req = CreateLobbyRequest {
            round_duration: Some(5), // Too short
            set_id: None,
        };

        let res = create_lobby(&state, req).await;
        assert!(matches!(res, Err(ApiError::Validation(_))));
    }

    #[tokio::test]
    async fn test_join_lobby_logic() {
        let (state, _dir) = setup_test_state().await;
        // First, create a lobby to get a join code
        let create_req = CreateLobbyRequest {
            round_duration: None,
            set_id: None,
        };
        let create_res = create_lobby(&state, create_req).await.unwrap();

        // Now, try to join it
        let join_code = create_res.join_code.clone();
        let join_req = JoinLobbyRequest {
            join_code: create_res.join_code,
            name: "Player1".to_string(),
        };
        let join_res = join_lobby(&state, join_req).await.unwrap();

        let lobby = state.lobbies.get(&join_code).unwrap();
        assert_eq!(lobby.get_player_count(), 2); // Admin + Player1
        assert!(lobby.has_player(&join_res.player_id));
        assert_eq!(
            join_res.session_token,
            format!("{}:{}", join_code, join_res.player_id)
        );
    }

    #[tokio::test]
    async fn test_join_lobby_invalid_code() {
        let (state, _dir) = setup_test_state().await;
        let join_req = JoinLobbyRequest {
            join_code: "123456".to_string(),
            name: "Player1".to_string(),
        };

        let res = join_lobby(&state, join_req).await;
        assert!(matches!(res, Err(ApiError::Lobby(msg)) if msg == "Invalid join code."));
    }

    #[tokio::test]
    async fn test_join_lobby_invalid_name() {
        let (state, _dir) = setup_test_state().await;
        let create_req = CreateLobbyRequest {
            round_duration: None,
            set_id: None,
        };
        let create_res = create_lobby(&state, create_req).await.unwrap();

        // Try to join with a name that is too short
        let join_req = JoinLobbyRequest {
            join_code: create_res.join_code.clone(),
            name: "a".to_string(),
        };

        let res = join_lobby(&state, join_req).await;
        assert!(matches!(res, Err(ApiError::Validation(_))));
    }

    #[tokio::test]
    async fn test_check_sessions_logic() {
        let (state, _dir) = setup_test_state().await;
        let create_res = create_lobby(
            &state,
            CreateLobbyRequest {
                round_duration: None,
                set_id: None,
            },
        )
        .await
        .unwrap();

        // Check a valid session (the admin)
        let check_req = CheckSessionsRequest {
            sessions: vec![SessionInfo {
                player_id: create_res.player_id,
            }],
        };

        let res = check_sessions(&state, check_req).await.unwrap();
        assert_eq!(res.valid_sessions.len(), 1);
        assert_eq!(res.valid_sessions[0].player_id, create_res.player_id);

        // Check an invalid session
        let check_req_invalid = CheckSessionsRequest {
            sessions: vec![SessionInfo {
                player_id: Uuid::new_v4(),
            }],
        };
        let res_invalid = check_sessions(&state, check_req_invalid).await.unwrap();
        assert_eq!(res_invalid.valid_sessions.len(), 0);
    }
}
