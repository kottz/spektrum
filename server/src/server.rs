use crate::game::{
    ColorDef, ErrorCode, EventContext, GameAction, GameEvent, GamePhase, GameResponse, Recipients,
    ResponsePayload, Song,
};
use crate::game_manager::GameManager;
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
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tracing::*;
use uuid::Uuid;

//--------------------------------------------------------------------------------
// External Message Protocol (Client <-> Server)
//--------------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMsg {
    JoinLobby {
        lobby_id: Uuid,
        admin_id: Option<Uuid>,
        name: String,
    },
    Leave {
        lobby_id: Uuid,
    },
    Answer {
        lobby_id: Uuid,
        color: String,
    },
    AdminAction {
        lobby_id: Uuid,
        action: AdminAction,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum AdminAction {
    StartGame,
    StartRound { colors: Option<Vec<String>> },
    EndRound,
    EndGame { reason: String },
    CloseGame { reason: String },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    JoinedLobby {
        player_id: Uuid,
        name: String,
        round_duration: u64,
        players: Vec<(String, i32)>,
    },
    InitialPlayerList {
        players: Vec<(String, i32)>,
    },
    PlayerJoined {
        player_name: String,
        current_score: i32,
    },
    PlayerLeft {
        name: String,
    },
    PlayerAnswered {
        name: String,
        correct: bool,
        new_score: i32,
    },
    StateChanged {
        phase: String,
        colors: Vec<ColorDef>,
        scoreboard: Vec<(String, i32)>,
    },
    GameOver {
        scores: Vec<(String, i32)>,
        reason: String,
    },
    GameClosed {
        reason: String,
    },
    Error {
        message: String,
    },
}

//--------------------------------------------------------------------------------
// Application State
//--------------------------------------------------------------------------------

#[derive(Clone)]
pub struct AppState {
    pub manager: Arc<Mutex<GameManager>>,
    pub connections: Arc<Mutex<HashMap<(Uuid, Uuid), UnboundedSender<ServerMessage>>>>,
    pub songs: Arc<Vec<Song>>,
}

impl AppState {
    pub fn new(songs: Vec<Song>) -> Self {
        Self {
            manager: Arc::new(Mutex::new(GameManager::new())),
            connections: Arc::new(Mutex::new(HashMap::new())),
            songs: Arc::new(songs),
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
struct ListLobbiesResponse {
    lobbies: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
struct CreateLobbyResponse {
    lobby_id: Uuid,
    admin_id: Uuid,
}

pub async fn create_lobby_handler(
    State(state): State<AppState>,
    Json(req): Json<CreateLobbyRequest>,
) -> impl IntoResponse {
    let round_duration = req.round_duration.unwrap_or(60);

    let all_colors = vec![
        ColorDef {
            name: "Red".into(),
            rgb: "#FF0000".into(),
        },
        ColorDef {
            name: "Green".into(),
            rgb: "#00FF00".into(),
        },
        ColorDef {
            name: "Blue".into(),
            rgb: "#0000FF".into(),
        },
        ColorDef {
            name: "Yellow".into(),
            rgb: "#FFFF00".into(),
        },
        ColorDef {
            name: "Purple".into(),
            rgb: "#800080".into(),
        },
        ColorDef {
            name: "Gold".into(),
            rgb: "#FFD700".into(),
        },
        ColorDef {
            name: "Silver".into(),
            rgb: "#C0C0C0".into(),
        },
        ColorDef {
            name: "Pink".into(),
            rgb: "#FFC0CB".into(),
        },
        ColorDef {
            name: "Black".into(),
            rgb: "#000000".into(),
        },
        ColorDef {
            name: "White".into(),
            rgb: "#FFFFFF".into(),
        },
        ColorDef {
            name: "Brown".into(),
            rgb: "#3D251E".into(),
        },
        ColorDef {
            name: "Orange".into(),
            rgb: "#FFA500".into(),
        },
        ColorDef {
            name: "Gray".into(),
            rgb: "#808080".into(),
        },
    ];

    let mgr = state.manager.lock().unwrap();
    let (lobby_id, admin_id) = mgr.create_lobby(state.songs.to_vec(), all_colors, round_duration);

    Json(CreateLobbyResponse { lobby_id, admin_id })
}

pub async fn list_lobbies_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mgr = state.manager.lock().unwrap();
    let lobby_ids = mgr.list_lobbies();
    Json(ListLobbiesResponse { lobbies: lobby_ids })
}

//--------------------------------------------------------------------------------
// WebSocket Handler
//--------------------------------------------------------------------------------

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

pub async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = unbounded_channel::<ServerMessage>();

    let mut lobby_id: Option<Uuid> = None;
    let mut player_id: Uuid = Uuid::new_v4();
    info!("New WebSocket connection from player {}", player_id);

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            info!("Sending message: {:?}", msg);
            if let Ok(text) = serde_json::to_string(&msg) {
                if ws_tx.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
        }
    });

    while let Some(Ok(Message::Text(text))) = ws_rx.next().await {
        info!("Received message: {}", text);
        match serde_json::from_str::<ClientMsg>(&text) {
            Ok(client_msg) => {
                let now = Instant::now();
                let mgr = state.manager.lock().unwrap();

                let responses = match client_msg {
                    ClientMsg::JoinLobby {
                        lobby_id: lid,
                        admin_id,
                        name,
                    } => {
                        if let Some(aid) = admin_id {
                            player_id = aid;
                        }
                        {
                            let mut conns = state.connections.lock().unwrap();
                            conns.insert((lid, player_id), tx.clone());
                        }
                        lobby_id = Some(lid);

                        if let Some(lobby) = mgr.get_lobby(&lid) {
                            let event = GameEvent {
                                context: EventContext {
                                    lobby_id: lid,
                                    sender_id: player_id,
                                    timestamp: now,
                                    is_admin: admin_id.is_some(),
                                },
                                action: GameAction::Join { name },
                            };
                            lobby.process_event(event)
                        } else {
                            vec![GameResponse {
                                recipients: Recipients::Single(player_id),
                                payload: ResponsePayload::Error {
                                    code: ErrorCode::LobbyNotFound,
                                    message: "Lobby not found".into(),
                                },
                            }]
                        }
                    }
                    ClientMsg::Leave { lobby_id: lid } => {
                        if let Some(lobby) = mgr.get_lobby(&lid) {
                            let event = GameEvent {
                                context: EventContext {
                                    lobby_id: lid,
                                    sender_id: player_id,
                                    timestamp: now,
                                    is_admin: false,
                                },
                                action: GameAction::Leave,
                            };
                            let responses = lobby.process_event(event);
                            {
                                let mut conns = state.connections.lock().unwrap();
                                conns.remove(&(lid, player_id));
                            }
                            responses
                        } else {
                            vec![]
                        }
                    }
                    ClientMsg::Answer {
                        lobby_id: lid,
                        color,
                    } => {
                        if let Some(lobby) = mgr.get_lobby(&lid) {
                            let event = GameEvent {
                                context: EventContext {
                                    lobby_id: lid,
                                    sender_id: player_id,
                                    timestamp: now,
                                    is_admin: false,
                                },
                                action: GameAction::Answer { color },
                            };
                            lobby.process_event(event)
                        } else {
                            vec![]
                        }
                    }
                    ClientMsg::AdminAction {
                        lobby_id: lid,
                        action,
                    } => {
                        if let Some(lobby) = mgr.get_lobby(&lid) {
                            let game_action = match action {
                                AdminAction::StartGame => GameAction::StartGame,
                                AdminAction::StartRound { colors } => GameAction::StartRound {
                                    specified_colors: colors,
                                },
                                AdminAction::EndRound => GameAction::EndRound,
                                AdminAction::EndGame { reason } => GameAction::EndGame { reason },
                                AdminAction::CloseGame { reason } => {
                                    GameAction::CloseGame { reason }
                                }
                            };

                            let event = GameEvent {
                                context: EventContext {
                                    lobby_id: lid,
                                    sender_id: player_id,
                                    timestamp: now,
                                    is_admin: true,
                                },
                                action: game_action,
                            };
                            let responses = lobby.process_event(event);

                            // Check if we need to remove the lobby
                            if responses
                                .iter()
                                .any(|r| matches!(r.payload, ResponsePayload::GameClosed { .. }))
                            {
                                mgr.remove_lobby(&lid);
                            }
                            responses
                        } else {
                            vec![]
                        }
                    }
                };
                drop(mgr);

                broadcast_responses(responses, &state);
            }
            Err(e) => {
                warn!("Failed to parse client msg: {}", e);
            }
        }
    }

    // Client disconnected
    if let (Some(lid), pid) = (lobby_id, player_id) {
        let mut conns = state.connections.lock().unwrap();
        conns.remove(&(lid, pid));
    }

    send_task.abort();
    info!("WebSocket closed");
}

fn broadcast_responses(responses: Vec<GameResponse>, state: &AppState) {
    let conns = state.connections.lock().unwrap();

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
            if let Some(sender) = conns.get(&key) {
                let _ = sender.send(server_msg.clone());
            }
        }
    }
}

pub fn convert_to_server_message(payload: &ResponsePayload) -> ServerMessage {
    match payload {
        ResponsePayload::Joined {
            player_id,
            name,
            round_duration,
            current_players,
        } => ServerMessage::JoinedLobby {
            player_id: *player_id,
            name: name.clone(),
            round_duration: *round_duration,
            players: current_players.clone(),
        },
        ResponsePayload::PlayerLeft { name } => ServerMessage::PlayerLeft { name: name.clone() },
        ResponsePayload::PlayerAnswered {
            name,
            correct,
            new_score,
        } => ServerMessage::PlayerAnswered {
            name: name.clone(),
            correct: *correct,
            new_score: *new_score,
        },
        ResponsePayload::StateChanged {
            phase,
            colors,
            scoreboard,
        } => {
            let phase_str = match phase {
                GamePhase::Lobby => "lobby",
                GamePhase::Score => "score",
                GamePhase::Question => "question",
                GamePhase::GameOver => "gameover",
            };
            ServerMessage::StateChanged {
                phase: phase_str.to_string(),
                colors: colors.clone(),
                scoreboard: scoreboard.clone(),
            }
        }
        ResponsePayload::GameOver {
            final_scores,
            reason,
        } => ServerMessage::GameOver {
            scores: final_scores.clone(),
            reason: reason.clone(),
        },
        ResponsePayload::GameClosed { reason } => ServerMessage::GameClosed {
            reason: reason.clone(),
        },
        ResponsePayload::Error { code, message } => ServerMessage::Error {
            message: format!("{:?}: {}", code, message),
        },
    }
}
