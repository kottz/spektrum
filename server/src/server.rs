use crate::db::{DbError, StoredData};
use crate::game::{
    EventContext, GameAction, GameEngine, GameEvent, GameUpdate, NameValidationError,
};
use crate::question::{QuestionError, QuestionStore};
use crate::uuid::Uuid;
use axum::extract::ws::Utf8Bytes;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::{
    extract::ws::{Message, WebSocket},
    extract::Multipart,
    extract::State,
    extract::WebSocketUpgrade,
    response::IntoResponse,
    Json,
};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use thiserror::Error;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;
use tracing::{error, info, trace};

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
    Connect { player_id: Uuid },
    Leave,
    Answer { answer: String },
    AdminAction { action: AdminAction },
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

pub struct Connection {
    pub lobby_id: Uuid,
    pub tx: Option<Sender<Utf8Bytes>>,
    pub connection_id: Option<Uuid>,
}

#[derive(Clone)]
pub struct AppState {
    pub lobbies: Arc<DashMap<Uuid, GameEngine>>,
    pub join_codes: Arc<DashMap<String, Uuid>>,
    pub connections: Arc<DashMap<Uuid, Connection>>,
    pub store: Arc<QuestionStore>,
    pub admin_passwords: Vec<String>,
}

impl AppState {
    pub fn new(question_manager: QuestionStore, admin_passwords: Vec<String>) -> Self {
        let state = Self {
            lobbies: Arc::new(DashMap::new()),
            join_codes: Arc::new(DashMap::new()),
            connections: Arc::new(DashMap::new()),
            store: Arc::new(question_manager),
            admin_passwords,
        };

        {
            let lobbies = state.lobbies.clone();
            let connections = state.connections.clone();
            let join_codes = state.join_codes.clone();
            tokio::spawn(async move {
                cleanup_lobbies(lobbies, connections, join_codes).await;
            });
        }

        state
    }

    fn generate_join_code(&self, lobby_id: Uuid) -> Result<String, ApiError> {
        // First try 6-digit codes
        for _ in 0..10_000 {
            let code = format!("{:06}", fastrand::u32(0..1_000_000));
            if !self.join_codes.contains_key(&code) {
                self.join_codes.insert(code.clone(), lobby_id);
                return Ok(code);
            }
        }

        // If many collisions, escalate to 7 digits
        for _ in 0..1_000_000 {
            let code = format!("{:07}", fastrand::u32(0..10_000_000));
            if !self.join_codes.contains_key(&code) {
                self.join_codes.insert(code.clone(), lobby_id);
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
    let num_questions = state.store.questions.read().await.len();
    let sets = state.store.sets.read().await;

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

    let questions = state.store.questions.read().await;
    let sets = state.store.sets.read().await;
    let selected_set = if let Some(set_id) = req.set_id {
        Some(
            sets.iter()
                .find(|set| set.id == set_id)
                .ok_or_else(|| ApiError::Validation(format!("Set with id {} not found", set_id)))?,
        )
    } else {
        None
    };

    let lobby_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    let join_code = state.generate_join_code(lobby_id)?;

    let mut engine = GameEngine::new(
        admin_id,
        Arc::clone(&questions),
        selected_set,
        round_duration,
        state.connections.clone(),
    );
    engine.add_player(admin_id, "Admin".into())?;
    trace!("Creating new lobby {}", lobby_id);

    state.connections.insert(
        admin_id,
        Connection {
            lobby_id,
            tx: None,
            connection_id: None,
        },
    );
    state.lobbies.insert(lobby_id, engine);

    info!(
        "Lobby created: {} with join code: {} (round_duration: {}s, set: {})",
        lobby_id,
        join_code,
        round_duration,
        selected_set
            .map(|s| s.name.as_ref())
            .unwrap_or("all questions")
    );

    Ok(CreateLobbyResponse {
        player_id: admin_id,
        join_code,
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
}

pub async fn join_lobby(
    state: &AppState,
    req: JoinLobbyRequest,
) -> Result<JoinLobbyResponse, ApiError> {
    let lobby_id = *state
        .join_codes
        .get(req.join_code.trim())
        .ok_or_else(|| ApiError::Lobby("Invalid join code.".into()))?
        .value();

    let mut engine = state
        .lobbies
        .get_mut(&lobby_id)
        .ok_or_else(|| ApiError::Lobby("Lobby not found. It may have been closed.".into()))?;

    if engine.is_full() {
        return Err(ApiError::Lobby("Lobby is full.".into()));
    }

    let new_player_id = Uuid::new_v4();
    engine.add_player(new_player_id, req.name)?;
    state.connections.insert(
        new_player_id,
        Connection {
            lobby_id,
            tx: None,
            connection_id: None,
        },
    );
    Ok(JoinLobbyResponse {
        player_id: new_player_id,
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
    state.store.load_questions().await?;
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
    let mut valid_sessions = Vec::new();
    let now = Instant::now();
    for session in req.sessions.iter() {
        if let Some(conn) = state.connections.get(&session.player_id) {
            if let Some(lobby) = state.lobbies.get(&conn.lobby_id) {
                if !lobby.is_finished() {
                    if let Some(last_update) = lobby.last_update() {
                        let duration = now.duration_since(last_update);
                        if let Some(system_time) = SystemTime::now().checked_sub(duration) {
                            let dt = DateTime::<Utc>::from(system_time);
                            valid_sessions.push(ValidSessionInfo {
                                player_id: session.player_id,
                                last_update: dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
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
    connection_id: Uuid,
}

impl WsConnection {
    fn new() -> Self {
        Self {
            player_id: None,
            connection_id: Uuid::new_v4(),
        }
    }
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (ws_tx, mut ws_rx) = socket.split();

    let (text_tx, text_rx) = channel::<Utf8Bytes>(128);
    let (bin_tx, bin_rx) = channel::<Bytes>(128);

    let mut conn = WsConnection::new();

    let send_task = spawn_sender_task(ws_tx, text_rx, bin_rx);

    while let Some(Ok(msg)) = ws_rx.next().await {
        let result = handle_message(msg, &mut conn, &state, &text_tx, &bin_tx).await;
        if result.is_err() {
            break;
        }
    }

    handle_disconnect(&conn, &state).await;
    send_task.abort();
}

fn spawn_sender_task(
    mut ws_tx: SplitSink<WebSocket, Message>,
    mut text_rx: Receiver<Utf8Bytes>,
    mut bin_rx: Receiver<Bytes>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
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
    })
}

async fn handle_message(
    msg: Message,
    conn: &mut WsConnection,
    state: &AppState,
    text_tx: &Sender<Utf8Bytes>,
    bin_tx: &Sender<Bytes>,
) -> Result<(), ()> {
    match msg {
        Message::Text(text) => {
            let client_msg: ClientMessage = match serde_json::from_str(&text) {
                Ok(msg) => msg,
                Err(e) => {
                    send_error_to_client(
                        text_tx,
                        format!("Invalid message format: {}", e),
                        "json_parse",
                    );
                    return Ok(());
                }
            };

            if let ClientMessage::Connect { player_id } = client_msg {
                handle_connect(player_id, conn, state, text_tx).await;
            } else if conn.player_id.is_some() {
                dispatch_game_action(client_msg, conn, state).await;
            } else {
                send_error_to_client(text_tx, "Must connect first.".to_string(), "not_connected");
            }
        }
        Message::Binary(payload) if payload.as_ref() == [HEARTBEAT_BYTE] => {
            let _ = bin_tx.try_send(payload);
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
    player_id: Uuid,
    conn: &mut WsConnection,
    state: &AppState,
    tx: &Sender<Utf8Bytes>,
) {
    if let Some(mut c) = state.connections.get_mut(&player_id) {
        let lobby_id = c.lobby_id;
        conn.player_id = Some(player_id);
        c.tx = Some(tx.clone());
        c.connection_id = Some(conn.connection_id);
        drop(c);

        if let Some(mut engine) = state.lobbies.get_mut(&lobby_id) {
            let event = GameEvent {
                context: EventContext {
                    sender_id: player_id,
                    timestamp: Instant::now(),
                },
                action: GameAction::Connect,
            };
            engine.process_event(event);
        }
    } else {
        send_error_to_client(
            tx,
            "ID not in lobby. Must join lobby first.".to_string(),
            "connect_id_not_found",
        );
    }
}

async fn dispatch_game_action(msg: ClientMessage, conn: &WsConnection, state: &AppState) {
    let Some(player_id) = conn.player_id else {
        error!("dispatch_game_action called without player_id");
        return;
    };
    if let Some(c) = state.connections.get(&player_id) {
        let lobby_id = c.lobby_id;
        drop(c);
        if let Some(mut engine) = state.lobbies.get_mut(&lobby_id) {
            let action = match msg {
                ClientMessage::Leave => GameAction::Leave,
                ClientMessage::Answer { answer } => GameAction::Answer { answer },
                ClientMessage::AdminAction { action } => match action {
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
                },
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
    }
}

async fn handle_disconnect(conn: &WsConnection, state: &AppState) {
    if let Some(player_id) = conn.player_id {
        if let Some(mut c) = state.connections.get_mut(&player_id) {
            if c.connection_id == Some(conn.connection_id) {
                trace!("Disconnect: Nullifying tx for player {}", player_id);
                c.tx = None;
                c.connection_id = None;
            } else {
                trace!("Disconnect: Stale disconnect for player {}", player_id);
            }
        }
    }
}

fn send_error_to_client(tx: &Sender<Utf8Bytes>, message: String, context: &str) {
    let error_update = GameUpdate::Error {
        message: Arc::from(message),
    };
    if let Ok(json) = serde_json::to_string(&error_update) {
        if tx.try_send(Utf8Bytes::from(json)).is_err() {
            error!("Failed to send '{}' error to client channel", context);
        }
    }
}

async fn cleanup_lobbies(
    lobbies: Arc<DashMap<Uuid, GameEngine>>,
    connections: Arc<DashMap<Uuid, Connection>>,
    join_codes: Arc<DashMap<String, Uuid>>,
) {
    let mut tick = tokio::time::interval(Duration::from_secs(60));
    loop {
        tick.tick().await;

        let finished_lobby_ids: Vec<Uuid> = lobbies
            .iter()
            .filter(|entry| entry.value().is_finished())
            .map(|entry| *entry.key())
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

        join_codes.retain(|_, v| !finished_lobby_ids.contains(v));
        connections.retain(|_, v| !finished_lobby_ids.contains(&v.lobby_id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::QuestionDatabase;
    use crate::StorageConfig;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    async fn setup_test_state() -> (AppState, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("questions.json");

        // Create a dummy questions file
        let dummy_data = StoredData {
            media: vec![],
            characters: vec![],
            questions: vec![],
            options: vec![],
            sets: vec![],
        };
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "{}", serde_json::to_string(&dummy_data).unwrap()).unwrap();

        let storage_config = StorageConfig::Filesystem {
            base_path: dir.path().to_path_buf(),
            file_path: "questions.json".to_string(),
        };
        let db = QuestionDatabase::new(&storage_config).unwrap();
        // Manually load empty data to avoid NoQuestions error on init
        db.set_stored_data(dummy_data).await.unwrap();

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
        assert_eq!(state.join_codes.len(), 1);
        assert!(state.join_codes.contains_key(&res.join_code));

        let lobby_id = *state.join_codes.get(&res.join_code).unwrap();
        let lobby = state.lobbies.get(&lobby_id).unwrap();

        assert_eq!(lobby.state.admin_id, res.player_id);
        assert_eq!(lobby.state.round_duration, 120);
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
        let join_req = JoinLobbyRequest {
            join_code: create_res.join_code,
            name: "Player1".to_string(),
        };
        let join_res = join_lobby(&state, join_req).await.unwrap();

        assert_eq!(state.connections.len(), 2); // Admin + Player1
        assert!(state.connections.contains_key(&join_res.player_id));

        let lobby_id = *state.join_codes.get(&create_res.join_code).unwrap();
        let lobby = state.lobbies.get(&lobby_id).unwrap();
        assert_eq!(lobby.state.players.len(), 2); // Admin + Player1
        assert!(lobby.state.players.contains_key(&join_res.player_id));
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
