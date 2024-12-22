use crate::game::{
    ColorDef, ErrorCode, EventContext, GameAction, GameEvent, GameResponse, Recipients,
    ResponsePayload, Song,
};
use crate::game_manager::{GameLobby, GameManager};
use crate::messages::{convert_to_server_message, AdminAction, ClientMessage, ServerMessage};
use axum::{
    extract::ws::{Message, WebSocket},
    extract::State,
    extract::WebSocketUpgrade,
    response::IntoResponse,
    Json,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    time::Instant,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tracing::*;
use uuid::Uuid;

//--------------------------------------------------------------------------------
// Application State
//--------------------------------------------------------------------------------

#[derive(Clone)]
pub struct AppState {
    pub manager: Arc<Mutex<GameManager>>,
    pub connections: Arc<RwLock<HashMap<(Uuid, Uuid), UnboundedSender<ServerMessage>>>>,
    pub songs: Arc<Vec<Song>>,
    pub all_colors: Arc<Vec<ColorDef>>,
}

impl AppState {
    pub fn new(songs: Vec<Song>, all_colors: Vec<ColorDef>) -> Self {
        Self {
            manager: Arc::new(Mutex::new(GameManager::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            songs: Arc::new(songs),
            all_colors: Arc::new(all_colors),
        }
    }
}

//--------------------------------------------------------------------------------
// HTTP Handlers for Lobby Management
//--------------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateLobbyRequest {
    round_duration: Option<u64>,
}

#[derive(Debug, Serialize)]
struct CreateLobbyResponse {
    lobby_id: Uuid,
    join_code: String,
    admin_id: Uuid,
}

pub async fn create_lobby_handler(
    State(state): State<AppState>,
    Json(req): Json<CreateLobbyRequest>,
) -> impl IntoResponse {
    let round_duration = req.round_duration.unwrap_or(60);

    let mgr = state.manager.lock().unwrap();
    let (lobby_id, join_code, admin_id) = mgr.create_lobby(state.songs.to_vec(), state.all_colors.to_vec(), round_duration);

    Json(CreateLobbyResponse { lobby_id, join_code, admin_id })
}

//--------------------------------------------------------------------------------
// WebSocket Handler
//--------------------------------------------------------------------------------

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

// Then modify the WebSocket handler to take advantage of this
pub async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = unbounded_channel::<ServerMessage>();

    let mut lobby_ref: Option<Arc<GameLobby>> = None;
    let mut player_id: Uuid = Uuid::new_v4();
    info!("New WebSocket connection from player {}", player_id);

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(text) = serde_json::to_string(&msg) {
                if ws_tx.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
        }
    });

    while let Some(Ok(Message::Text(text))) = ws_rx.next().await {
        match serde_json::from_str::<ClientMessage>(&text) {
            Ok(client_msg) => {
                let now = Instant::now();

                // Handle join message specially to set up lobby_ref
                if let ClientMessage::JoinLobby {
                    join_code,
                    admin_id,
                    name,
                } = &client_msg
                {
                    if lobby_ref.is_none() {
                        let manager = state.manager.lock().unwrap();
                        let lobby_id = manager.get_lobby_id_from_join_code(&join_code).unwrap();
                        lobby_ref = manager.get_lobby(&lobby_id);
                        drop(manager);

                        // Set up player ID and add connection
                        if let Some(aid) = admin_id {
                            player_id = *aid;
                        }
                        if let Some(lobby) = &lobby_ref {
                            state
                                .connections
                                .write()
                                .unwrap()
                                .insert((lobby_id, player_id), tx.clone());
                        }
                    }
                }

                // Process messages using the cached lobby reference
                let responses = if let Some(ref lobby) = lobby_ref {
                    match client_msg {
                        ClientMessage::JoinLobby { name, .. } => {
                            let event = GameEvent {
                                context: EventContext {
                                    lobby_id: lobby.id(),
                                    sender_id: player_id,
                                    timestamp: now,
                                    is_admin: false,
                                },
                                action: GameAction::Join { name },
                            };
                            lobby.process_event(event)
                        }
                        ClientMessage::Leave { lobby_id } => {
                            let event = GameEvent {
                                context: EventContext {
                                    lobby_id,
                                    sender_id: player_id,
                                    timestamp: now,
                                    is_admin: false,
                                },
                                action: GameAction::Leave,
                            };
                            let responses = lobby.process_event(event);
                            state
                                .connections
                                .write()
                                .unwrap()
                                .remove(&(lobby_id, player_id));
                            responses
                        }
                        ClientMessage::Answer { lobby_id, color } => {
                            let event = GameEvent {
                                context: EventContext {
                                    lobby_id,
                                    sender_id: player_id,
                                    timestamp: now,
                                    is_admin: false,
                                },
                                action: GameAction::Answer { color },
                            };
                            lobby.process_event(event)
                        }
                        ClientMessage::AdminAction { lobby_id, action } => {
                            let game_action = match action {
                                AdminAction::StartGame => GameAction::StartGame,
                                AdminAction::StartRound { colors } => GameAction::StartRound {
                                    specified_colors: colors,
                                },
                                AdminAction::EndRound => GameAction::EndRound,
                                AdminAction::SkipSong => GameAction::SkipSong,
                                AdminAction::EndGame { reason } => GameAction::EndGame { reason },
                                AdminAction::CloseGame { reason } => {
                                    GameAction::CloseGame { reason }
                                }
                            };

                            let event = GameEvent {
                                context: EventContext {
                                    lobby_id,
                                    sender_id: player_id,
                                    timestamp: now,
                                    is_admin: true,
                                },
                                action: game_action.clone(),
                            };

                            let responses = lobby.process_event(event);

                            // Only lock manager briefly if we need to remove the lobby
                            if matches!(game_action, GameAction::CloseGame { .. }) {
                                state.manager.lock().unwrap().remove_lobby(&lobby_id);
                            }

                            responses
                        }
                    }
                } else {
                    vec![GameResponse {
                        recipients: Recipients::Single(player_id),
                        payload: ResponsePayload::Error {
                            code: ErrorCode::LobbyNotFound,
                            message: "Lobby not found".into(),
                        },
                    }]
                };

                broadcast_responses(responses, &state);
            }
            Err(e) => {
                warn!("Failed to parse client message: {}", e);
            }
        }
    }

    // Cleanup on disconnect
    if let Some(lobby) = lobby_ref {
        let lobby_id = lobby.id();
        state
            .connections
            .write()
            .unwrap()
            .remove(&(lobby_id, player_id));

        // Optional: Check if lobby is empty and remove it
        if lobby.is_empty() {
            state.manager.lock().unwrap().remove_lobby(&lobby_id);
        }
    }

    send_task.abort();
    info!("WebSocket closed");
}

fn broadcast_responses(responses: Vec<GameResponse>, state: &AppState) {
    let conns = state.connections.read().unwrap();

    for response in responses {
        let server_msg = convert_to_server_message(&response.payload);

        // Extract all recipients for this lobby from the connections map
        let lobby_connections: Vec<_> = conns
            .keys()
            .filter(|(lobby, _)| true) // We might want to filter by lobby here
            .collect();

        let recipients = match &response.recipients {
            Recipients::Single(id) => lobby_connections
                .iter()
                .filter(|(_, pid)| pid == id)
                .copied()
                .collect::<Vec<_>>(),
            Recipients::Multiple(ids) => lobby_connections
                .iter()
                .filter(|(_, pid)| ids.contains(pid))
                .copied()
                .collect(),
            Recipients::AllExcept(exclude_ids) => lobby_connections
                .iter()
                .filter(|(_, pid)| !exclude_ids.contains(pid))
                .copied()
                .collect(),
            Recipients::All => lobby_connections,
        };

        for &key in &recipients {
            if let Some(sender) = conns.get(key) {
                let _ = sender.send(server_msg.clone());
            }
        }
    }
}
