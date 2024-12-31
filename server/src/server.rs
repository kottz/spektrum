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
        Self {
            manager: Arc::new(Mutex::new(GameManager::new())),
            questions: Arc::new(questions),
        }
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

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = unbounded_channel::<ServerMessage>();

    let mut lobby_ref: Option<Arc<GameLobby>> = None;
    let mut player_id: Uuid = Uuid::new_v4();
    info!("New WebSocket connection from player {}", player_id);

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(text) = serde_json::to_string(&msg) {
                if let Err(e) = ws_tx.send(Message::Text(text)).await {
                    error!("Failed to send WebSocket message: {}", e);
                    break;
                }
            }
        }
    });

    while let Some(Ok(Message::Text(text))) = ws_rx.next().await {
        match serde_json::from_str::<ClientMessage>(&text) {
            Ok(client_msg) => {
                let now = Instant::now();

                match &client_msg {
                    ClientMessage::JoinLobby {
                        join_code,
                        admin_id,
                        ..
                    } if lobby_ref.is_none() => {
                        if let Ok(manager) = state.manager.lock() {
                            if let Ok(Some(lobby_id)) =
                                manager.get_lobby_id_from_join_code(join_code)
                            {
                                if let Ok(Some(lobby)) = manager.get_lobby(&lobby_id) {
                                    if let Some(aid) = admin_id {
                                        player_id = *aid;
                                    }
                                    if let Err(e) = lobby.add_connection(player_id, tx.clone()) {
                                        error!("Failed to add connection: {}", e);
                                        continue;
                                    }
                                    lobby_ref = Some(lobby);
                                }
                            }
                        }
                    }
                    _ => {}
                }

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
                            let event = GameEvent {
                                context: EventContext {
                                    lobby_id,
                                    sender_id: player_id,
                                    timestamp: now,
                                },
                                action: GameAction::Leave,
                            };
                            if let Err(e) = lobby.remove_connection(&player_id) {
                                error!("Failed to remove connection: {}", e);
                            }
                            event
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

                    if let Ok(responses) = lobby.process_event(event) {
                        for response in responses {
                            let server_msg = convert_to_server_message(&response.payload);
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

        if let Ok(true) = lobby.is_empty() {
            if let Ok(manager) = state.manager.lock() {
                if let Err(e) = manager.remove_lobby(&lobby.id()) {
                    error!("Failed to remove empty lobby: {}", e);
                }
            }
        }
    }

    send_task.abort();
    info!("WebSocket closed for player {}", player_id);
}
