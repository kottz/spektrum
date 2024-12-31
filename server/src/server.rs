use crate::game::{EventContext, GameAction, GameEvent, Recipients};
use crate::game_manager::{GameLobby, GameManager};
use crate::messages::{convert_to_server_message, AdminAction, ClientMessage, ServerMessage};
use crate::question::GameQuestion;
use axum::{
    extract::ws::{Message, WebSocket},
    extract::State,
    extract::WebSocketUpgrade,
    response::IntoResponse,
    Json,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use thiserror::Error;
use tokio::sync::mpsc::unbounded_channel;
use tokio::time::Duration;
use tracing::*;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Lobby error: {0}")]
    Lobby(String),

    #[error("Lock acquisition failed: {0}")]
    Lock(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match &self {
            ApiError::Validation(_) => (axum::http::StatusCode::BAD_REQUEST, self.to_string()),
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
    pub questions: Arc<Vec<GameQuestion>>,
}

impl AppState {
    pub fn new(questions: Vec<GameQuestion>) -> Self {
        let state = Self {
            manager: Arc::new(Mutex::new(GameManager::new())),
            questions: Arc::new(questions),
        };

        let manager = state.manager.clone();

        // Spawn cleanup task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Run every hour
            loop {
                interval.tick().await;
                if let Ok(mgr) = manager.lock() {
                    if let Err(e) = mgr.cleanup_empty_lobbies() {
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
) -> Result<impl IntoResponse, ApiError> {
    let round_duration = req.round_duration.unwrap_or(60);
    if round_duration < 10 {
        return Err(ApiError::Validation(
            "Round duration must be at least 10 seconds".into(),
        ));
    }

    let mgr = state
        .manager
        .lock()
        .map_err(|e| ApiError::Lock(e.to_string()))?;

    let (lobby_id, join_code, admin_id) = mgr
        .create_lobby(state.questions.to_vec(), round_duration)
        .map_err(|e| ApiError::Lobby(e.to_string()))?;

    Ok(Json(CreateLobbyResponse {
        lobby_id,
        join_code,
        admin_id,
    }))
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

pub async fn handle_socket(socket: WebSocket, state: AppState) {
    // Split the WebSocket into sender and receiver streams
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = unbounded_channel::<ServerMessage>();

    // Track the current lobby reference, if any, and the player's unique ID
    let mut lobby_ref: Option<Arc<GameLobby>> = None;
    let mut player_id: Uuid = Uuid::new_v4();

    info!("New WebSocket connection from player {}", player_id);

    // Task to handle sending messages to the client
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            // Convert the ServerMessage to JSON and send it over the WebSocket
            match serde_json::to_string(&msg) {
                Ok(text) => {
                    if let Err(e) = ws_tx.send(Message::Text(text)).await {
                        error!("Failed to send WebSocket message: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize server message: {}", e);
                }
            }
        }
    });

    // Process incoming messages from the client
    while let Some(Ok(Message::Text(text))) = ws_rx.next().await {
        match serde_json::from_str::<ClientMessage>(&text) {
            Ok(client_msg) => {
                let now = Instant::now();

                // Handle the JoinLobby logic for every JoinLobby message,
                // rather than only on the first one.
                if let ClientMessage::JoinLobby {
                    join_code,
                    admin_id,
                    name: _,
                } = &client_msg
                {
                    if let Ok(manager) = state.manager.lock() {
                        info!(
                            "number of lobbies at open: {}",
                            manager.list_lobbies().unwrap().len()
                        );
                        if let Ok(Some(lobby_id)) = manager.get_lobby_id_from_join_code(join_code) {
                            if let Ok(Some(lobby)) = manager.get_lobby(&lobby_id) {
                                // If there's an admin_id, use that as the player ID
                                if let Some(aid) = admin_id {
                                    player_id = *aid;
                                }

                                // Add the connection to the lobby
                                if let Err(e) = lobby.add_connection(player_id, tx.clone()) {
                                    error!("Failed to add connection: {}", e);
                                    // Skip further processing for this message
                                    continue;
                                }

                                // Update the tracked lobby reference
                                lobby_ref = Some(lobby);
                            }
                        }
                    }
                }

                // Process other client events if we have a lobby
                if let Some(ref lobby) = lobby_ref {
                    let event = match client_msg {
                        ClientMessage::JoinLobby { name, .. } => GameEvent {
                            context: EventContext {
                                lobby_id: lobby.id(),
                                sender_id: player_id,
                                timestamp: now,
                            },
                            action: GameAction::Join { name },
                        },
                        ClientMessage::Leave { lobby_id } => {
                            // Attempt to remove the player from the connections
                            if let Err(e) = lobby.remove_connection(&player_id) {
                                error!("Failed to remove connection: {}", e);
                            }
                            GameEvent {
                                context: EventContext {
                                    lobby_id,
                                    sender_id: player_id,
                                    timestamp: now,
                                },
                                action: GameAction::Leave,
                            }
                        }
                        ClientMessage::Answer { lobby_id, answer } => GameEvent {
                            context: EventContext {
                                lobby_id,
                                sender_id: player_id,
                                timestamp: now,
                            },
                            action: GameAction::Answer { answer },
                        },
                        ClientMessage::AdminAction { lobby_id, action } => {
                            let game_action = match &action {
                                AdminAction::StartGame => GameAction::StartGame,
                                AdminAction::StartRound {
                                    specified_alternatives,
                                } => GameAction::StartRound {
                                    specified_alternatives: specified_alternatives.clone(),
                                },
                                AdminAction::EndRound => GameAction::EndRound,
                                AdminAction::SkipQuestion => GameAction::SkipQuestion,
                                AdminAction::EndGame { reason } => GameAction::EndGame {
                                    reason: reason.clone(),
                                },
                                AdminAction::CloseGame { reason } => {
                                    // Remove the entire lobby from the manager
                                    if let Ok(manager) = state.manager.lock() {
                                        if let Err(e) = manager.remove_lobby(&lobby_id) {
                                            error!("Failed to remove lobby: {}", e);
                                        }
                                    }
                                    GameAction::CloseGame {
                                        reason: reason.clone(),
                                    }
                                }
                            };
                            GameEvent {
                                context: EventContext {
                                    lobby_id,
                                    sender_id: player_id,
                                    timestamp: now,
                                },
                                action: game_action,
                            }
                        }
                    };

                    // Process the event in the game logic
                    if let Ok(responses) = lobby.process_event(event) {
                        for response in responses {
                            let server_msg = convert_to_server_message(&response.payload);

                            // Determine who receives the message
                            if let Ok(connections) = lobby.connections.read() {
                                let recipient_ids = match response.recipients {
                                    Recipients::Single(id) => vec![id],
                                    Recipients::Multiple(ids) => ids,
                                    Recipients::AllExcept(exclude_ids) => connections
                                        .keys()
                                        .filter(|&&id| !exclude_ids.contains(&id))
                                        .copied()
                                        .collect(),
                                    Recipients::All => connections.keys().copied().collect(),
                                };

                                // Send the message to the chosen recipients
                                for id in recipient_ids {
                                    if let Some(sender) = connections.get(&id) {
                                        let _ = sender.send(server_msg.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to parse client message: {}", e);
                let error_msg = ServerMessage::Error {
                    message: format!("Invalid message format: {}", e),
                };
                let _ = tx.send(error_msg);
            }
        }
    }

    // Cleanup on disconnect
    if let Some(lobby) = lobby_ref {
        if let Err(e) = lobby.remove_connection(&player_id) {
            error!("Failed to remove connection during cleanup: {}", e);
        }

        // Acquire a write lock on the manager
        if let Ok(manager) = state.manager.lock() {
            // Check if the lobby is now empty
            if let Ok(is_empty) = lobby.is_empty() {
                if is_empty {
                    // Remove the lobby
                    if let Err(e) = manager.remove_lobby(&lobby.id()) {
                        error!("Failed to remove empty lobby: {}", e);
                    } else {
                        info!(
                            "number of lobbies at close: {}",
                            manager.list_lobbies().unwrap().len()
                        );
                    }
                }
            }
        }
    }

    // Stop the sending task
    send_task.abort();
    info!("WebSocket closed for player {}", player_id);
}
