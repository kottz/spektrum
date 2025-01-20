use crate::db::StoredData;
use crate::game::{
    EventContext, GameAction, GameEvent, GamePhase, GameResponse, Recipients, ResponsePayload,
};
use crate::game_manager::{GameLobby, GameManager};
use crate::messages::GameState;
use crate::messages::{convert_to_server_message, AdminAction, ClientMessage, ServerMessage};
use crate::question::QuestionStore;
use axum::{
    extract::ws::{Message, WebSocket},
    extract::State,
    extract::WebSocketUpgrade,
    response::IntoResponse,
    Json,
};
use bytes::Bytes;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use thiserror::Error;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::Duration;
use tracing::*;
use uuid::Uuid;

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

    #[error("Lock acquisition failed: {0}")]
    Lock(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match &self {
            ApiError::Validation(_) => (axum::http::StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::Unauthorized => (axum::http::StatusCode::UNAUTHORIZED, self.to_string()),
            ApiError::Database(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                self.to_string(),
            ),
            ApiError::Lobby(_) => (axum::http::StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::Lock(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                self.to_string(),
            ),
        };

        let body = Json(serde_json::json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}

#[derive(Clone)]
pub struct AppState {
    pub manager: Arc<Mutex<GameManager>>,
    //pub questions: Arc<Vec<GameQuestion>>,
    pub store: Arc<QuestionStore>,
    admin_password: String,
}

impl AppState {
    pub fn new(question_manager: QuestionStore, admin_password: String) -> Self {
        let state = Self {
            manager: Arc::new(Mutex::new(GameManager::new())),
            store: Arc::new(question_manager),
            admin_password
        };

        let manager = state.manager.clone();

        // Spawn cleanup task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Run every hour
            loop {
                interval.tick().await;
                if let Ok(mgr) = manager.lock() {
                    if let Err(e) = mgr.cleanup_inactive_lobbies() {
                        error!("Error during periodic lobby cleanup: {}", e);
                    } else {
                        info!("Periodic cleanup of empty lobbies complete");
                    }
                }
            }
        });

        state
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateLobbyRequest {
    round_duration: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct CreateLobbyResponse {
    lobby_id: Uuid,
    join_code: String,
    admin_id: Uuid,
}

pub async fn create_lobby_handler(
    State(state): State<AppState>,
    Json(req): Json<CreateLobbyRequest>,
) -> Result<Json<CreateLobbyResponse>, ApiError> {
    let round_duration = req.round_duration.unwrap_or(60);
    if round_duration < 10 {
        return Err(ApiError::Validation(
            "Round duration must be at least 10 seconds".into(),
        ));
    }

    let questions = state.store.questions.read().await;
    let mgr = state
        .manager
        .lock()
        .map_err(|e| ApiError::Lock(e.to_string()))?;

    let (lobby_id, join_code, admin_id) = mgr
        .create_lobby(questions.clone(), round_duration)
        .map_err(|e| ApiError::Lobby(e.to_string()))?;

    Ok(Json(CreateLobbyResponse {
        lobby_id,
        join_code,
        admin_id,
    }))
}

#[derive(Debug, Deserialize)]
pub struct GetStoredDataRequest {
    password: String,
}

pub async fn get_stored_data_handler(
    State(state): State<AppState>,
    Json(req): Json<GetStoredDataRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if req.password != state.admin_password {
        return Err(ApiError::Unauthorized);
    }

    let stored_data = state
        .store
        .get_stored_data()
        .map_err(|e| ApiError::Database(e.to_string()))?;

    Ok(Json(stored_data))
}

#[derive(Debug, Deserialize)]
pub struct SetStoredDataRequest {
    password: String,
    stored_data: StoredData,
}

pub async fn set_stored_data_handler(
    State(state): State<AppState>,
    Json(req): Json<SetStoredDataRequest>,
) -> Result<impl IntoResponse, ApiError> {
    info!("password: {:?}", req.password);
    if req.password != state.admin_password {
        return Err(ApiError::Unauthorized);
    }
    info!("data: {:?}", req.stored_data);
    // here we want to take this data and store it in the database
    if let Err(e) = state.store.set_stored_data(req.stored_data) {
        return Err(ApiError::Database(e.to_string()));
    }

    Ok(Json(serde_json::json!({
        "message": "Data stored successfully"
    })))
}
#[derive(Debug, Deserialize)]
pub struct CheckSessionsRequest {
    pub sessions: Vec<SessionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub player_id: Uuid,
    pub lobby_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct CheckSessionsResponse {
    pub valid_sessions: Vec<SessionInfo>,
}

pub async fn check_sessions_handler(
    State(state): State<AppState>,
    Json(req): Json<CheckSessionsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let mut valid_sessions = Vec::new();
    let mgr = state
        .manager
        .lock()
        .map_err(|e| ApiError::Lock(e.to_string()))?;

    for session in req.sessions.iter() {
        if let Some(lobby) = mgr
            .get_lobby(&session.lobby_id)
            .map_err(|e| ApiError::Lobby(e.to_string()))?
        {
            if lobby
                .is_empty()
                .map_err(|e| ApiError::Lobby(e.to_string()))?
            {
                continue;
            }
            valid_sessions.push(session.clone());
        }
    }

    Ok(Json(CheckSessionsResponse { valid_sessions }))
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Holds the state for a single WebSocket connection
struct WSConnectionState {
    tx: UnboundedSender<ServerMessage>,
    lobby_ref: Option<Arc<GameLobby>>,
    player_id: Option<Uuid>,
}

impl WSConnectionState {
    fn new(tx: UnboundedSender<ServerMessage>) -> Self {
        Self {
            tx,
            lobby_ref: None,
            player_id: None,
        }
    }
}

pub async fn handle_socket(socket: WebSocket, state: AppState) {
    let (ws_tx, mut ws_rx) = socket.split();
    let (tx, rx) = unbounded_channel::<ServerMessage>();
    let (pong_tx, pong_rx) = unbounded_channel::<Bytes>();

    let conn_state = Arc::new(RwLock::new(WSConnectionState::new(tx.clone())));

    // Spawn the sender task
    let send_task = spawn_sender_task(ws_tx, rx, pong_rx);

    // Process incoming messages
    process_incoming_messages(&mut ws_rx, conn_state.clone(), &state, tx.clone(), pong_tx).await;

    // Cleanup
    handle_disconnect(conn_state, &state).await;
    send_task.abort();
}

/// Spawns a task to handle sending messages to the client
fn spawn_sender_task(
    mut ws_tx: SplitSink<WebSocket, Message>,
    mut rx: UnboundedReceiver<ServerMessage>,
    mut pong_rx: UnboundedReceiver<Bytes>,
) -> JoinHandle<()> {
    let mut ping_interval = tokio::time::interval(Duration::from_secs(30));

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = ping_interval.tick() => {
                    let ping_payload: Bytes = Bytes::new();
                    if let Err(e) = ws_tx.send(Message::Ping(ping_payload)).await {
                        error!("Failed to send ping: {}", e);
                        break;
                    }
                }
                Some(msg) = rx.recv() => {
                    if let Err(e) = send_server_message(&mut ws_tx, msg).await {
                        error!("Failed to send message: {}", e);
                        break;
                    }
                }
                Some(payload) = pong_rx.recv() => {
                    if let Err(e) = ws_tx.send(Message::Pong(payload)).await {
                        error!("Failed to send pong: {}", e);
                        break;
                    }
                }
                else => break,
            }
        }
    })
}

/// Processes incoming messages from the client
async fn process_incoming_messages(
    ws_rx: &mut SplitStream<WebSocket>,
    conn_state: Arc<RwLock<WSConnectionState>>,
    state: &AppState,
    tx: UnboundedSender<ServerMessage>,
    pong_tx: UnboundedSender<Bytes>,
) {
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) => match msg {
                Message::Text(text) => {
                    handle_text_message(text.to_string(), &conn_state, state, &tx).await;
                }
                Message::Ping(payload) => {
                    if let Err(e) = pong_tx.send(payload) {
                        error!("Failed to send pong through channel: {}", e);
                        break;
                    }
                }
                Message::Pong(_) => {
                    trace!("Received pong from client");
                }
                Message::Close(_) => {
                    let state = conn_state.read().await;
                    if let Some(pid) = state.player_id {
                        info!("Client initiated close for player {}", pid);
                    }
                    break;
                }
                _ => {}
            },
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
        }
    }
}

/// Handles text messages received from the client
async fn handle_text_message(
    text: String,
    conn_state: &Arc<RwLock<WSConnectionState>>,
    state: &AppState,
    tx: &UnboundedSender<ServerMessage>,
) {
    match serde_json::from_str::<ClientMessage>(&text) {
        Ok(client_msg) => match &client_msg {
            ClientMessage::Reconnect {
                lobby_id,
                player_id,
            } => {
                handle_reconnect(
                    *lobby_id, // These are now Uuid already
                    *player_id, conn_state, state, tx,
                )
                .await;
            }
            ClientMessage::JoinLobby {
                join_code,
                admin_id,
                name,
            } => {
                handle_join_lobby(join_code, admin_id.as_ref(), name, conn_state, state, tx).await;
            }
            _ => {
                handle_game_message(client_msg, conn_state, Instant::now()).await;
            }
        },
        Err(e) => {
            warn!("Failed to parse client message: {}", e);
            let _ = tx.send(ServerMessage::Error {
                message: format!("Invalid message format: {}", e),
            });
        }
    }
}

async fn handle_reconnect(
    lobby_id: Uuid,
    player_id: Uuid,
    conn_state: &Arc<RwLock<WSConnectionState>>,
    state: &AppState,
    tx: &UnboundedSender<ServerMessage>,
) {
    // Get the lobby reference in a separate scope
    let lobby = {
        let manager = match state.manager.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to lock manager: {}", e);
                return;
            }
        };

        match manager.get_lobby(&lobby_id) {
            Ok(Some(lobby)) => Some(lobby.clone()),
            _ => None,
        }
    }; // MutexGuard is dropped here

    // Now we can safely use await since we no longer hold the lock
    if let Some(lobby) = lobby {
        if let Ok(()) = lobby.reconnect_player(player_id, tx.clone()) {
            let mut state = conn_state.write().await;
            state.player_id = Some(player_id);
            state.lobby_ref = Some(lobby.clone());

            if let Ok(game_response) = lobby.get_game_state() {
                if let ResponsePayload::StateChanged {
                    phase,
                    question_type,
                    alternatives,
                    scoreboard,
                } = game_response.payload
                {
                    let phase_str = match phase {
                        GamePhase::Lobby => "lobby".to_string(),
                        GamePhase::Score => "score".to_string(),
                        GamePhase::Question => "question".to_string(),
                        GamePhase::GameOver => "gameover".to_string(),
                    };

                    let _ = tx.send(ServerMessage::ReconnectSuccess {
                        game_state: GameState {
                            phase: phase_str,
                            question_type,
                            alternatives,
                            scoreboard,
                            current_song: None,
                        },
                    });
                }
            }
        }
    }
}

/// Handles initial lobby join
async fn handle_join_lobby(
    join_code: &str,
    admin_id: Option<&Uuid>,
    name: &str,
    conn_state: &Arc<RwLock<WSConnectionState>>,
    state: &AppState,
    tx: &UnboundedSender<ServerMessage>,
) {
    info!("Joining lobby with code: {}", join_code);
    // Get what we need from the manager immediately in a separate scope
    let lobby = {
        let manager = match state.manager.lock() {
            Ok(manager) => manager,
            Err(e) => {
                error!("Failed to acquire manager lock: {}", e);
                return;
            }
        };

        let lobby_id = match manager.get_lobby_id_from_join_code(join_code) {
            Ok(Some(id)) => id,
            Ok(None) => {
                warn!("No lobby found for join code: {}", join_code);
                return;
            }
            Err(e) => {
                error!("Failed to get lobby ID: {}", e);
                return;
            }
        };

        match manager.get_lobby(&lobby_id) {
            Ok(Some(lobby)) => Some(lobby.clone()),
            _ => None,
        }
    }; // Manager lock is dropped here

    if let Some(lobby) = lobby {
        let player_id = admin_id.copied().unwrap_or_else(Uuid::new_v4);

        // Add the connection
        if lobby.add_connection(player_id, tx.clone()).is_ok() {
            // Keep track of the connection state
            let mut state_lock = conn_state.write().await;
            state_lock.player_id = Some(player_id);
            state_lock.lobby_ref = Some(lobby.clone());
            drop(state_lock); // Release the write lock before processing events

            // Restore the join event logic so the server broadcasts updates
            let now = Instant::now();
            let event = create_game_event(
                ClientMessage::JoinLobby {
                    join_code: join_code.to_string(),
                    admin_id: admin_id.copied(),
                    name: name.to_string(),
                },
                lobby.id(),
                player_id,
                now,
            );

            if let Ok(responses) = lobby.process_event(event) {
                send_game_responses(responses, &lobby).await;
            }
        }
    }
}

async fn send_game_responses(responses: Vec<GameResponse>, lobby: &Arc<GameLobby>) {
    for response in responses {
        // Convert the response to a server message
        let server_msg = convert_to_server_message(&response.payload);

        // Get active connections and send to appropriate recipients
        if let Ok(active_conns) = lobby.get_active_connections() {
            let recipient_ids = match response.recipients {
                Recipients::Single(id) => vec![id],
                Recipients::Multiple(ids) => ids,
                Recipients::AllExcept(exclude_ids) => active_conns
                    .iter()
                    .filter(|(id, _)| !exclude_ids.contains(id))
                    .map(|(id, _)| *id)
                    .collect(),
                Recipients::All => active_conns.iter().map(|(id, _)| *id).collect(),
            };

            for id in recipient_ids {
                if let Some(sender) = active_conns.iter().find(|(conn_id, _)| *conn_id == id) {
                    let _ = sender.1.send(server_msg.clone());
                }
            }
        }
    }
}

/// Handles game-specific messages
async fn handle_game_message(
    msg: ClientMessage,
    conn_state: &Arc<RwLock<WSConnectionState>>,
    now: Instant,
) {
    let state = conn_state.read().await;
    if let (Some(lobby), Some(player_id)) = (&state.lobby_ref, state.player_id) {
        let event = create_game_event(msg, lobby.id(), player_id, now);
        if let Ok(responses) = lobby.process_event(event) {
            send_game_responses(responses, lobby).await;
        }
    }
}

/// Creates a game event from a client message
fn create_game_event(
    msg: ClientMessage,
    lobby_id: Uuid,
    player_id: Uuid,
    now: Instant,
) -> GameEvent {
    let context = EventContext {
        lobby_id,
        sender_id: player_id,
        timestamp: now,
    };

    GameEvent {
        context,
        action: match msg {
            ClientMessage::JoinLobby { name, .. } => GameAction::Join { name },
            ClientMessage::Leave { .. } => GameAction::Leave,
            ClientMessage::Answer { answer, .. } => GameAction::Answer { answer },
            ClientMessage::AdminAction { action, .. } => convert_admin_action(action),
            _ => GameAction::Leave, // Default case
        },
    }
}

/// Converts an admin action to a game action
fn convert_admin_action(action: AdminAction) -> GameAction {
    match action {
        AdminAction::StartGame => GameAction::StartGame,
        AdminAction::StartRound {
            specified_alternatives,
        } => GameAction::StartRound {
            specified_alternatives,
        },
        AdminAction::EndRound => GameAction::EndRound,
        AdminAction::SkipQuestion => GameAction::SkipQuestion,
        AdminAction::EndGame { reason } => GameAction::EndGame { reason },
        AdminAction::CloseGame { reason } => GameAction::CloseGame { reason },
    }
}

/// Sends a server message through the WebSocket
async fn send_server_message(
    ws_tx: &mut SplitSink<WebSocket, Message>,
    msg: ServerMessage,
) -> Result<(), Box<dyn std::error::Error>> {
    let text = serde_json::to_string(&msg)?;
    ws_tx.send(Message::Text(text.into())).await?;
    Ok(())
}

/// Handles cleanup when a connection is closed
async fn handle_disconnect(conn_state: Arc<RwLock<WSConnectionState>>, state: &AppState) {
    let state_data = conn_state.read().await;
    if let (Some(lobby), Some(player_id)) = (&state_data.lobby_ref, state_data.player_id) {
        // Mark player as disconnected
        if let Err(e) = lobby.mark_player_disconnected(&player_id) {
            error!("Failed to mark player as disconnected: {}", e);
        }

        // Check cleanup in a separate block to drop the first lock
        let should_remove = lobby.is_empty_for_cleanup().unwrap_or(false);
        if should_remove {
            if let Ok(manager) = state.manager.lock() {
                if let Err(e) = manager.remove_lobby(&lobby.id()) {
                    error!("Failed to remove empty lobby: {}", e);
                }
            }
        }
    }
}
