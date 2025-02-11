use crate::db::{DbError, StoredData};
use crate::game::{
    EventContext, GameAction, GameEngine, GameEvent, GamePhase, GameUpdate, GameUpdatePacket,
    Recipients,
};
use crate::messages::{AdminAction, ClientMessage};
use tokio::time::{interval, Duration};
//use crate::game_manager::{GameLobby, GameManager};
//use crate::messages::GameState;
//use crate::messages::{convert_to_server_message, AdminAction, ClientMessage, ServerMessage};
use crate::question::QuestionStore;
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
use dashmap::DashMap;
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

    #[error("No more join codes error")]
    OutOfJoinCodes,

    #[error("Lock acquisition failed: {0}")]
    Lock(String),

    #[error("Unsupported media type")]
    UnsupportedMediaType,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal server error: {0}")]
    InternalServerError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match &self {
            ApiError::Validation(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            ApiError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::Lobby(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::OutOfJoinCodes => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::Lock(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::UnsupportedMediaType => {
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, self.to_string())
            }
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::InternalServerError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
        };

        let body = Json(serde_json::json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}

impl From<DbError> for ApiError {
    fn from(err: DbError) -> Self {
        ApiError::Database(err.to_string())
    }
}

pub struct Connection {
    pub lobby_id: Uuid,
    pub tx: Option<UnboundedSender<GameUpdate>>,
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
        // TODO spawn cleanup task
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
    pub name: String,
    pub question_count: usize,
}

#[derive(Debug, Serialize)]
pub struct ListSetsResponse {
    pub num_questions: usize,
    pub sets: Vec<SetInfo>,
}

pub async fn list_sets_handler(
    State(state): State<AppState>,
) -> Result<Json<ListSetsResponse>, ApiError> {
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

    Ok(Json(ListSetsResponse {
        num_questions,
        sets: sets_info,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateLobbyRequest {
    pub round_duration: Option<u64>,
    pub set_id: Option<i64>,
}

#[derive(Debug, serde::Serialize)]
pub struct CreateLobbyResponse {
    pub player_id: Uuid,
    pub join_code: String,
}

pub async fn create_lobby_handler(
    State(state): State<crate::server::AppState>,
    Json(req): Json<CreateLobbyRequest>,
) -> Result<Json<CreateLobbyResponse>, ApiError> {
    let round_duration = req.round_duration.unwrap_or(60);
    if round_duration < 10 {
        return Err(ApiError::Validation(
            "Round duration must be at least 10 seconds".into(),
        ));
    }

    // Read questions and sets from the QuestionStore.
    let questions = state.store.questions.read().await;
    let sets = state.store.sets.read().await;

    // Validate and select a set if requested.
    let selected_set = if let Some(set_id) = req.set_id {
        Some(
            sets.iter()
                .find(|set| set.id == set_id)
                .ok_or_else(|| ApiError::Validation(format!("Set with id {} not found", set_id)))?,
        )
    } else {
        None
    };

    // Generate new IDs and a join code.
    let lobby_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    let join_code = format!("{:06}", fastrand::u32(0..1_000_000));

    // Create a new game engine.
    let engine = GameEngine::new(
        admin_id,
        Arc::clone(&questions),
        selected_set,
        round_duration,
        state.connections.clone(),
    );

    // Insert the new engine into the global lobbies map.
    state.lobbies.insert(lobby_id, engine);

    Ok(Json(CreateLobbyResponse {
        player_id: admin_id,
        join_code,
    }))
}

#[derive(Debug, Deserialize)]
pub struct JoinLobbyRequest {
    pub join_code: String,
    pub name: String,
}

// New response type that returns the new player’s UUID.
#[derive(Debug, Serialize)]
pub struct JoinLobbyResponse {
    pub player_id: Uuid,
}

/// This handler accepts a join code and a name. It uses the join_codes map
/// to look up the lobby ID, retrieves the GameEngine for that lobby, creates a new
/// player ID, tells the engine to add the player, and returns the new player ID.
pub async fn join_lobby_handler(
    State(state): State<AppState>,
    Json(req): Json<JoinLobbyRequest>,
) -> Result<Json<JoinLobbyResponse>, ApiError> {
    // Look up the lobby ID using the join code.
    let lobby_id = state
        .join_codes
        .get(&req.join_code)
        .ok_or_else(|| ApiError::Lobby("Lobby not found".into()))?;

    // Retrieve the game engine corresponding to that lobby.
    let mut engine = state
        .lobbies
        .get_mut(lobby_id.value())
        .ok_or_else(|| ApiError::Lobby("Lobby engine not found".into()))?;

    // Create a new player id for the joining player.
    let new_player_id = Uuid::new_v4();

    engine.add_player(new_player_id, req.name).map_err(|e| ApiError::Lobby(e.to_string()))?;

    // Return the new player's id.
    Ok(Json(JoinLobbyResponse {
        player_id: new_player_id,
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
    if !state.admin_passwords.contains(&req.password) {
        return Err(ApiError::Unauthorized);
    }

    let stored_data = state
        .store
        .get_stored_data()
        .await
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
    if !state.admin_passwords.contains(&req.password) {
        return Err(ApiError::Unauthorized);
    }

    if let Err(e) = state.store.backup_stored_data().await {
        return Err(ApiError::Database(e.to_string()));
    }

    if let Err(e) = state.store.set_stored_data(req.stored_data).await {
        return Err(ApiError::Database(e.to_string()));
    }

    state
        .store
        .load_questions()
        .await
        .map_err(|e| ApiError::Database(e.to_string()))?;
    match state.store.get_stored_data().await {
        Ok(data) => Ok(Json(data)),
        Err(e) => Err(ApiError::Database(e.to_string())),
    }
}

#[derive(Debug, Serialize)]
pub struct UploadCharacterImageResponse {
    image_url: String,
}

pub async fn upload_character_image_handler(
    State(state): State<AppState>,
    Path(character_name): Path<String>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut password = None;
    let mut image_data = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read multipart field: {}", e)))?
    {
        match field.name().unwrap_or_default() {
            "password" => {
                password =
                    Some(field.text().await.map_err(|e| {
                        ApiError::BadRequest(format!("Failed to read password: {}", e))
                    })?)
            }
            "image" => {
                if !field
                    .content_type()
                    .unwrap_or("")
                    .eq_ignore_ascii_case("image/avif")
                {
                    return Err(ApiError::UnsupportedMediaType);
                }
                image_data = Some(field.bytes().await.map_err(|e| {
                    ApiError::BadRequest(format!("Failed to read image data: {}", e))
                })?);
            }
            _ => continue,
        }
    }
    let password = password.ok_or(ApiError::Unauthorized)?;
    if !state.admin_passwords.contains(&password) {
        return Err(ApiError::Unauthorized);
    }
    let image_data = image_data.ok_or(ApiError::BadRequest("Missing image file".to_string()))?;

    // Get the store from state and use it to store the image
    let url = state
        .store
        .store_character_image(&character_name, &image_data)
        .await?;

    Ok(Json(UploadCharacterImageResponse { image_url: url }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub player_id: Uuid,
    pub lobby_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CheckSessionsRequest {
    pub sessions: Vec<SessionInfo>,
}

#[derive(Debug, Serialize)]
pub struct CheckSessionsResponse {
    pub valid_sessions: Vec<SessionInfo>,
}

pub async fn check_sessions_handler(
    State(state): State<AppState>,
    Json(req): Json<CheckSessionsRequest>,
) -> Result<Json<CheckSessionsResponse>, ApiError> {
    let mut valid_sessions = Vec::new();

    // Iterate over each session provided by the client.
    for session in req.sessions.iter() {
        // Look up the lobby in our global DashMap.
        if let Some(lobby_arc) = state.lobbies.get(&session.lobby_id) {
            let lobby = lobby_arc.value();
            if lobby.get_phase() == GamePhase::GameOver && lobby.is_empty() {
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

/// Simplified connection state - only tracks player ID
struct WSConnectionState {
    player_id: Option<Uuid>,
}

impl WSConnectionState {
    fn new() -> Self {
        Self { player_id: None }
    }
}

pub async fn handle_socket(socket: WebSocket, state: AppState) {
    let (ws_tx, mut ws_rx) = socket.split();
    let (tx, rx) = unbounded_channel::<GameUpdate>();
    let (pong_tx, pong_rx) = unbounded_channel::<Bytes>();

    let conn_state = Arc::new(RwLock::new(WSConnectionState::new()));

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
    mut rx: UnboundedReceiver<GameUpdate>,
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

async fn process_incoming_messages(
    ws_rx: &mut SplitStream<WebSocket>,
    conn_state: Arc<RwLock<WSConnectionState>>,
    state: &AppState,
    tx: UnboundedSender<GameUpdate>,
    pong_tx: UnboundedSender<Bytes>,
) {
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) => match msg {
                Message::Text(text) => {
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(ClientMessage::Connect { player_id }) => {
                            // Look up existing connection info
                            if let Some(conn) = state.connections.get(&player_id) {
                                // Store player_id in connection state
                                conn_state.write().await.player_id = Some(player_id);

                                // Update sender in connections map
                                state.connections.insert(
                                    player_id,
                                    Connection {
                                        lobby_id: conn.lobby_id,
                                        tx: Some(tx.clone()),
                                    },
                                );

                                // Get current game state by triggering GetState
                                if let Some(mut engine) = state.lobbies.get_mut(&conn.lobby_id) {
                                    let event = GameEvent {
                                        context: EventContext {
                                            sender_id: player_id,
                                            timestamp: Instant::now(),
                                        },
                                        action: GameAction::GetState,
                                    };
                                    engine.process_event(event);
                                }
                            } else {
                                let _ = tx.send(GameUpdate::Error {
                                    message: "ID not in lobby. Must join lobby first".into(),
                                });
                            }
                        }
                        Ok(msg) => {
                            // Handle all other messages
                            let state_read = conn_state.read().await;
                            if let Some(player_id) = state_read.player_id {
                                if let Some(conn) = state.connections.get(&player_id) {
                                    if let Some(mut engine) = state.lobbies.get_mut(&conn.lobby_id)
                                    {
                                        let event = GameEvent {
                                            context: EventContext {
                                                sender_id: player_id,
                                                timestamp: Instant::now(),
                                            },
                                            action: match msg {
                                                ClientMessage::Leave => GameAction::Leave,
                                                ClientMessage::Answer { answer } => {
                                                    GameAction::Answer { answer }
                                                }
                                                ClientMessage::AdminAction { action } => {
                                                    match action {
                                                        AdminAction::StartGame => {
                                                            GameAction::StartGame
                                                        }
                                                        AdminAction::StartRound => {
                                                            GameAction::StartRound
                                                        }
                                                        AdminAction::EndRound => {
                                                            GameAction::EndRound
                                                        }
                                                        AdminAction::SkipQuestion => {
                                                            GameAction::SkipQuestion
                                                        }
                                                        AdminAction::EndGame { reason } => {
                                                            GameAction::EndGame { reason }
                                                        }
                                                        AdminAction::CloseGame { reason } => {
                                                            GameAction::CloseGame { reason }
                                                        }
                                                    }
                                                }
                                                _ => {
                                                    error!(
                                                        "Unexpected message after join: {:?}",
                                                        msg
                                                    );
                                                    continue;
                                                }
                                            },
                                        };
                                        engine.process_event(event);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse client message: {}", e);
                            let _ = tx.send(GameUpdate::Error {
                                message: format!("Invalid message format: {}", e),
                            });
                        }
                    }
                }
                Message::Ping(payload) => {
                    if let Err(e) = pong_tx.send(payload) {
                        error!("Failed to send pong: {}", e);
                        break;
                    }
                }
                Message::Pong(_) => {
                    trace!("Received pong from client");
                }
                Message::Close(_) => {
                    let state_read = conn_state.read().await;
                    if let Some(pid) = state_read.player_id {
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
// async fn handle_text_message(
//     text: String,
//     conn_state: &Arc<RwLock<WSConnectionState>>,
//     state: &AppState,
//     tx: &tokio::sync::mpsc::UnboundedSender<GameUpdate>,
// ) {
//     match serde_json::from_str::<ClientMessage>(&text) {
//         Ok(client_msg) => {
//             match client_msg {
//                 ClientMessage::JoinLobby { player_id } => {
//                     if let Some(existing_uuid) = player_id {
//                         // Attempt to reconnect using the provided UUID.
//                         handle_reconnect(
//                             join_code,
//                             existing_uuid,
//                             name,
//                             admin_id,
//                             conn_state,
//                             state,
//                             tx,
//                         )
//                         .await;
//                     } else {
//                         // Proceed as a normal join.
//                         handle_join_lobby(
//                             &join_code,
//                             admin_id.as_ref(),
//                             &name,
//                             player_id,
//                             conn_state,
//                             state,
//                             tx,
//                         )
//                         .await;
//                     }
//                 }
//                 // Other variants are handled as usual.
//                 other => {
//                     handle_game_message(other, conn_state, Instant::now()).await;
//                 }
//             }
//         }
//         Err(e) => {
//             eprintln!("Failed to parse client message: {}", e);
//             let _ = tx.send(ServerMessage::Error {
//                 message: format!("Invalid message format: {}", e),
//             });
//         }
//     }
// }

// async fn handle_reconnect(
//     join_code: String,
//     player_id: Uuid,
//     name: String,
//     admin_id: Option<Uuid>,
//     conn_state: &Arc<RwLock<WSConnectionState>>,
//     state: &AppState,
//     tx: &tokio::sync::mpsc::UnboundedSender<ServerMessage>,
// ) {
//     // Look up the lobby ID from the join code.
//     let lobby_id = {
//         let manager = state.manager.lock().unwrap();
//         manager
//             .get_lobby_id_from_join_code(&join_code)
//             .unwrap_or(None)
//     };
//     if let Some(lobby_id) = lobby_id {
//         let lobby_option = {
//             let manager = state.manager.lock().unwrap();
//             manager.get_lobby(&lobby_id).unwrap_or(None)
//         };
//         if let Some(lobby) = lobby_option {
//             match lobby.reconnect_player(player_id, tx.clone()) {
//                 Ok(()) => {
//                     let mut cs = conn_state.write().await;
//                     cs.player_id = Some(player_id);
//                     cs.lobby_ref = Some(lobby.clone());
//                     if let Ok(game_response) = lobby.get_game_state() {
//                         let _ = tx.send(ServerMessage::ReconnectSuccess {
//                             game_state: crate::messages::convert_to_game_state(
//                                 &game_response.payload,
//                             ),
//                         });
//                     }
//                 }
//                 Err(e) => {
//                     eprintln!("Reconnect failed for player {}: {}", player_id, e);
//                     let _ = tx.send(ServerMessage::Error {
//                         message: format!("Reconnect failed: {}", e),
//                     });
//                 }
//             }
//         } else {
//             eprintln!("Lobby not found for join code: {}", join_code);
//             let _ = tx.send(ServerMessage::Error {
//                 message: "Lobby not found".into(),
//             });
//         }
//     } else {
//         // If no lobby is found for the join code, fall back to a normal join.
//         handle_join_lobby(
//             &join_code,
//             admin_id.as_ref(),
//             &name,
//             Some(player_id),
//             conn_state,
//             state,
//             tx,
//         )
//         .await;
//     }
// }

/// Handles initial lobby join (or reconnect, if a player_id is provided)
// async fn handle_join_lobby(
//     join_code: &str,
//     admin_id: Option<&Uuid>,
//     name: &str,
//     // New optional field: if provided, try to reconnect
//     player_id: Option<Uuid>,
//     conn_state: &Arc<RwLock<WSConnectionState>>,
//     state: &AppState,
//     tx: &UnboundedSender<ServerMessage>,
// ) {
//     info!("Joining lobby with code: {}", join_code);
//     // Look up the lobby from the manager.
//     let lobby = {
//         let manager = match state.manager.lock() {
//             Ok(manager) => manager,
//             Err(e) => {
//                 error!("Failed to acquire manager lock: {}", e);
//                 return;
//             }
//         };
//
//         let lobby_id = match manager.get_lobby_id_from_join_code(join_code) {
//             Ok(Some(id)) => id,
//             Ok(None) => {
//                 warn!("No lobby found for join code: {}", join_code);
//                 return;
//             }
//             Err(e) => {
//                 error!("Failed to get lobby ID: {}", e);
//                 return;
//             }
//         };
//
//         match manager.get_lobby(&lobby_id) {
//             Ok(Some(lobby)) => Some(lobby.clone()),
//             _ => None,
//         }
//     }; // manager lock released here
//
//     if let Some(lobby) = lobby {
//         // If the client provided a player_id, attempt to reconnect.
//         if let Some(existing_uuid) = player_id {
//             if lobby.reconnect_player(existing_uuid, tx.clone()).is_ok() {
//                 let mut state_lock = conn_state.write().await;
//                 state_lock.player_id = Some(existing_uuid);
//                 state_lock.lobby_ref = Some(lobby.clone());
//                 drop(state_lock);
//                 // Send back current game state to confirm reconnection.
//                 if let Ok(game_response) = lobby.get_game_state() {
//                     let _ = tx.send(ServerMessage::ReconnectSuccess {
//                         game_state: crate::messages::convert_to_game_state(&game_response.payload),
//                     });
//                 }
//                 return; // Reconnect successful; do not create a new join.
//             } else {
//                 info!(
//                     "Reconnect attempt for {} failed; falling back to a new join",
//                     existing_uuid
//                 );
//             }
//         }
//
//         // Otherwise (or if reconnect failed), create a new player_id.
//         let new_player_id = Uuid::new_v4();
//         if lobby.add_connection(new_player_id, tx.clone()).is_ok() {
//             let mut state_lock = conn_state.write().await;
//             state_lock.player_id = Some(new_player_id);
//             state_lock.lobby_ref = Some(lobby.clone());
//             drop(state_lock);
//
//             let now = Instant::now();
//             // Create a join event. When constructing the event, we set player_id to None
//             // (since this is a new join) even though the server will use new_player_id.
//             let event = create_game_event(
//                 ClientMessage::JoinLobby {
//                     join_code: join_code.to_string(),
//                     admin_id: admin_id.copied(),
//                     name: name.to_string(),
//                     player_id: None,
//                 },
//                 lobby.id(),
//                 new_player_id,
//                 now,
//             );
//
//             if let Ok(responses) = lobby.process_event(event) {
//                 send_game_responses(responses, &lobby).await;
//             }
//         }
//     }
// }

// async fn send_game_responses(responses: Vec<GameResponse>, lobby: &Arc<GameLobby>) {
//     for response in responses {
//         // Convert the response to a server message
//         let server_msg = convert_to_server_message(&response.payload);
//
//         // Get active connections and send to appropriate recipients
//         if let Ok(active_conns) = lobby.get_active_connections() {
//             let recipient_ids = match response.recipients {
//                 Recipients::Single(id) => vec![id],
//                 Recipients::Multiple(ids) => ids,
//                 Recipients::AllExcept(exclude_ids) => active_conns
//                     .iter()
//                     .filter(|(id, _)| !exclude_ids.contains(id))
//                     .map(|(id, _)| *id)
//                     .collect(),
//                 Recipients::All => active_conns.iter().map(|(id, _)| *id).collect(),
//             };
//
//             for id in recipient_ids {
//                 if let Some(sender) = active_conns.iter().find(|(conn_id, _)| *conn_id == id) {
//                     let _ = sender.1.send(server_msg.clone());
//                 }
//             }
//         }
//     }
// }
//
// /// Handles game-specific messages
// async fn handle_game_message(
//     msg: ClientMessage,
//     conn_state: &Arc<RwLock<WSConnectionState>>,
//     now: Instant,
// ) {
//     let state = conn_state.read().await;
//     if let (Some(lobby), Some(player_id)) = (&state.lobby_ref, state.player_id) {
//         let event = create_game_event(msg, lobby.id(), player_id, now);
//         if let Ok(responses) = lobby.process_event(event) {
//             send_game_responses(responses, lobby).await;
//         }
//     }
// }
//
// /// Creates a game event from a client message
// fn create_game_event(
//     msg: ClientMessage,
//     lobby_id: Uuid,
//     player_id: Uuid,
//     now: Instant,
// ) -> GameEvent {
//     let context = EventContext {
//         sender_id: player_id,
//         timestamp: now,
//     };
//
//     GameEvent {
//         context,
//         action: match msg {
//             ClientMessage::JoinLobby { name, .. } => GameAction::Join { name },
//             ClientMessage::Leave { .. } => GameAction::Leave,
//             ClientMessage::Answer { answer, .. } => GameAction::Answer { answer },
//             ClientMessage::AdminAction { action, .. } => convert_admin_action(action),
//             _ => GameAction::Leave, // Default case
//         },
//     }
// }
//
// /// Converts an admin action to a game action
// fn convert_admin_action(action: AdminAction) -> GameAction {
//     match action {
//         AdminAction::StartGame => GameAction::StartGame,
//         AdminAction::StartRound {
//             specified_alternatives,
//         } => GameAction::StartRound {
//             specified_alternatives,
//         },
//         AdminAction::EndRound => GameAction::EndRound,
//         AdminAction::SkipQuestion => GameAction::SkipQuestion,
//         AdminAction::EndGame { reason } => GameAction::EndGame { reason },
//         AdminAction::CloseGame { reason } => GameAction::CloseGame { reason },
//     }
// }
//

// /// Sends a server message through the WebSocket
async fn send_server_message(
    ws_tx: &mut SplitSink<WebSocket, Message>,
    msg: GameUpdate,
) -> Result<(), Box<dyn std::error::Error>> {
    let text = serde_json::to_string(&msg)?;
    ws_tx.send(Message::Text(text.into())).await?;
    Ok(())
}

/// Handles cleanup when a connection is closed
async fn handle_disconnect(conn_state: Arc<RwLock<WSConnectionState>>, state: &AppState) {
    let state_read = conn_state.read().await;
    if let Some(player_id) = state_read.player_id {
        // Only remove the sender, preserving the lobby_id
        if let Some(conn) = state.connections.get(&player_id) {
            state.connections.insert(
                player_id,
                Connection {
                    lobby_id: conn.lobby_id,
                    tx: None,
                },
            );
        }
    }
}

async fn cleanup_lobbies(lobby_map: Arc<DashMap<Uuid, Arc<GameEngine>>>) {
    let mut tick = interval(Duration::from_secs(60));
    loop {
        tick.tick().await;
        for entry in lobby_map.iter() {
            let lobby = entry.value();
            if lobby.is_finished() {
                let lobby_id = *entry.key();
                info!("Removing finished lobby {}", lobby_id);
                lobby_map.remove(&lobby_id);
            }
        }
    }
}
