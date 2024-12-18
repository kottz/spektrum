use crate::game::{
    ColorDef, GamePhase, InputEvent, OutputEvent, OutputEventData, Song, StateOperation,
};
use crate::game_manager::{GameManager, ManagerEvent, ManagerOutput};
use axum::{
    extract::ws::{Message, WebSocket},
    extract::State,
    extract::WebSocketUpgrade,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tower_http::services::ServeDir;
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
        player_id: Uuid,
        name: String,
        is_admin: bool,
    },
    Leave {
        lobby_id: Uuid,
        player_id: Uuid,
    },
    Answer {
        lobby_id: Uuid,
        player_id: Uuid,
        color: String,
    },
    ToggleState {
        lobby_id: Uuid,
        player_id: Uuid,
        specified_colors: Option<Vec<String>>,
        operation: StateOperation,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ServerMsg {
    InitialPlayerList {
        players: Vec<(String, i32)>,
    },
    PlayerJoined {
        player_name: String,
        current_score: i32,
    },
    PlayerLeft {
        player_name: String,
    },
    PlayerAnswered {
        player_name: String,
        correct: bool,
        new_score: i32,
    },
    StateChanged {
        new_phase: String,
        colors: Vec<ColorDef>,
    },
    GameOver {
        final_scores: Vec<(String, i32)>,
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
    pub connections: Arc<Mutex<HashMap<(Uuid, Uuid), UnboundedSender<ServerMsg>>>>,
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

    let mut mgr = state.manager.lock().unwrap();
    mgr.update(
        ManagerEvent::CreateLobby {
            songs: state.songs.to_vec(),
            colors: all_colors,
            round_duration,
        },
        Instant::now(),
    );
    let lobby_id = mgr.lobbies.keys().last().cloned().unwrap();

    Json(CreateLobbyResponse { lobby_id })
}

pub async fn list_lobbies_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mgr = state.manager.lock().unwrap();
    let lobby_ids: Vec<Uuid> = mgr.lobbies.keys().cloned().collect();
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
    let (tx, mut rx) = unbounded_channel::<ServerMsg>();

    let mut lobby_id: Option<Uuid> = None;
    let mut player_id: Option<Uuid> = None;

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
                let mut mgr = state.manager.lock().unwrap();
                let events = match client_msg {
                    ClientMsg::JoinLobby {
                        lobby_id: lid,
                        player_id: pid,
                        name,
                        is_admin,
                    } => {
                        {
                            let mut conns = state.connections.lock().unwrap();
                            conns.insert((lid, pid), tx.clone());
                        }
                        lobby_id = Some(lid);
                        player_id = Some(pid);
                        mgr.update(
                            ManagerEvent::LobbyEvent {
                                lobby_id: lid,
                                event: InputEvent::Join {
                                    sender_id: pid,
                                    name,
                                    is_admin,
                                },
                            },
                            now,
                        )
                    }
                    ClientMsg::Leave {
                        lobby_id: lid,
                        player_id: pid,
                    } => {
                        let ev = mgr.update(
                            ManagerEvent::LobbyEvent {
                                lobby_id: lid,
                                event: InputEvent::Leave { sender_id: pid },
                            },
                            now,
                        );
                        {
                            let mut conns = state.connections.lock().unwrap();
                            conns.remove(&(lid, pid));
                        }
                        ev
                    }
                    ClientMsg::Answer {
                        lobby_id: lid,
                        player_id: pid,
                        color,
                    } => mgr.update(
                        ManagerEvent::LobbyEvent {
                            lobby_id: lid,
                            event: InputEvent::Answer {
                                sender_id: pid,
                                color,
                            },
                        },
                        now,
                    ),
                    ClientMsg::ToggleState {
                        lobby_id: lid,
                        player_id: pid,
                        specified_colors,
                        operation,
                    } => mgr.update(
                        ManagerEvent::LobbyEvent {
                            lobby_id: lid,
                            event: InputEvent::ToggleState {
                                sender_id: pid,
                                specified_colors,
                                operation,
                            },
                        },
                        now,
                    ),
                };
                drop(mgr);

                broadcast_manager_outputs(events, &state);
            }
            Err(e) => {
                warn!("Failed to parse client msg: {}", e);
            }
        }
    }

    // Client disconnected
    if let (Some(lid), Some(pid)) = (lobby_id, player_id) {
        let mut conns = state.connections.lock().unwrap();
        conns.remove(&(lid, pid));
    }

    send_task.abort();
    info!("WebSocket closed");
}

fn broadcast_manager_outputs(outputs: Vec<ManagerOutput>, state: &AppState) {
    let conns = state.connections.lock().unwrap();

    for mo in outputs {
        let lobby_id = mo.lobby_id;
        let recipient = mo.event.recipient;
        let data = &mo.event.data;

        let server_msg = match data {
            OutputEventData::InitialPlayerList { players } => ServerMsg::InitialPlayerList {
                players: players.clone(),
            },
            OutputEventData::PlayerJoined {
                player_name,
                current_score,
            } => ServerMsg::PlayerJoined {
                player_name: player_name.clone(),
                current_score: *current_score,
            },
            OutputEventData::PlayerLeft { player_name } => ServerMsg::PlayerLeft {
                player_name: player_name.clone(),
            },
            OutputEventData::PlayerAnswered {
                player_name,
                correct,
                new_score,
            } => ServerMsg::PlayerAnswered {
                player_name: player_name.clone(),
                correct: *correct,
                new_score: *new_score,
            },
            OutputEventData::StateChanged { new_phase, colors } => {
                let pstr = match new_phase {
                    GamePhase::Lobby => "lobby",
                    GamePhase::Score => "score",
                    GamePhase::Question => "question",
                    GamePhase::GameOver => "gameover",
                }
                .to_string();
                ServerMsg::StateChanged {
                    new_phase: pstr,
                    colors: colors.clone(),
                }
            }
            OutputEventData::GameOver {
                final_scores,
                reason,
            } => ServerMsg::GameOver {
                final_scores: final_scores.clone(),
                reason: reason.clone(),
            },
            OutputEventData::GameClosed { reason } => ServerMsg::GameClosed {
                reason: reason.clone(),
            },
            OutputEventData::Error { message } => ServerMsg::Error {
                message: message.clone(),
            },
        };

        if let Some(client_sender) = conns.get(&(lobby_id, recipient)) {
            let _ = client_sender.send(server_msg);
        }
    }
}
