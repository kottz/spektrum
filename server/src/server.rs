use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::warn;
use uuid::Uuid;

use crate::game::{GameError, GameEvent, GameHandle, GameState};
use crate::game_manager::GameLobbyManager;
use crate::models::{
    ClientMessage, ColorResult, GameStateMsg, LeaderboardEntry, LobbyCreateRequest, LobbyInfo,
    PlayerAnsweredMsg, ServerMessage, Song, UpdateAnswerCount,
};
use crate::spotify::SpotifyController;

impl IntoResponse for GameError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            GameError::LobbyNotFound(_) => StatusCode::NOT_FOUND,
            GameError::PlayerNotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, self.to_string()).into_response()
    }
}

#[derive(Debug)]
pub struct AppError(pub GameError);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.0 {
            GameError::LobbyNotFound(_) => StatusCode::NOT_FOUND,
            GameError::PlayerNotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(json!({
            "error": self.0.to_string()
        }));

        (status, body).into_response()
    }
}

impl From<GameError> for AppError {
    fn from(err: GameError) -> Self {
        AppError(err)
    }
}

#[derive(Clone)]
pub struct ServerState {
    pub game_manager: Arc<GameLobbyManager>,
    pub spotify: Option<Arc<Mutex<SpotifyController>>>,
    pub songs: Arc<Vec<Song>>,
}

#[axum::debug_handler]
pub async fn create_lobby_handler(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<LobbyCreateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let (handle, events) = state
        .game_manager
        .create_lobby(req.name, (*state.songs).clone())
        .await?;

    let spotify = state.spotify.clone();
    let handle_clone = handle.clone();

    tokio::spawn(async move {
        handle_lobby_events(handle_clone, events, spotify).await;
    });

    let snapshot = handle.get_state().await?;
    Ok(Json(LobbyInfo {
        id: snapshot.id,
        name: snapshot.name,
        player_count: snapshot.players.len(),
    }))
}

pub async fn list_lobbies_handler(State(state): State<Arc<ServerState>>) -> impl IntoResponse {
    let lobbies = state.game_manager.get_all_lobbies().await;
    Json(lobbies)
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ServerState>>,
    Query(params): Query<HashMap<String, String>>,
) -> axum::response::Response {
    let lobby_id = match params.get("lobby").and_then(|id| Uuid::parse_str(id).ok()) {
        Some(id) => id,
        None => return (axum::http::StatusCode::BAD_REQUEST, "Invalid lobby ID").into_response(),
    };

    match state.game_manager.get_lobby(lobby_id).await {
        Ok(handle) => ws.on_upgrade(move |socket| handle_socket(socket, handle)),
        Err(_) => (axum::http::StatusCode::NOT_FOUND, "Lobby not found").into_response(),
    }
}

async fn handle_socket(socket: WebSocket, handle: GameHandle) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let mut player_name: Option<String> = None;

    // Forward messages from game to websocket
    let forward_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(Message::Text(text))) = ws_rx.next().await {
        if let Ok(msg) = serde_json::from_str::<ClientMessage>(&text) {
            match msg {
                ClientMessage::Join { name } => {
                    match handle.add_player(name.clone(), tx.clone()).await {
                        Ok(()) => {
                            player_name = Some(name);
                            send_initial_state(&handle, &tx).await;
                        }
                        Err(e) => {
                            send_error(&tx, &format!("Failed to join: {}", e));
                            break;
                        }
                    }
                }
                ClientMessage::SelectColor { color } => {
                    if let Some(name) = &player_name {
                        if let Ok((correct, score)) = handle.answer_color(name.clone(), color).await
                        {
                            let response = ServerMessage::ColorResult(ColorResult {
                                correct,
                                score,
                                total_score: score,
                            });
                            send_message(&tx, &response);
                        }
                    }
                }
                ClientMessage::CreateLobby { name } => {
                    send_error(&tx, "Cannot create lobby through WebSocket connection");
                }
                ClientMessage::UpdateLobbyName { name } => {
                    if let Err(e) = handle.update_name(name).await {
                        send_error(&tx, &format!("Failed to update lobby name: {}", e));
                    }
                }
            }
        }
    }

    if let Some(name) = player_name {
        handle.remove_player(name).await.ok();
    }
    forward_task.abort();
}

pub async fn handle_lobby_events(
    handle: GameHandle,
    mut events: mpsc::UnboundedReceiver<GameEvent>,
    spotify: Option<Arc<Mutex<SpotifyController>>>,
) {
    while let Some(event) = events.recv().await {
        match event {
            GameEvent::StateChanged {
                state,
                colors: _,
                current_song,
            } => {
                handle_spotify_state(&spotify, &state, current_song).await;
                broadcast_game_state(&handle).await;
            }
            GameEvent::PlayerJoined { .. } | GameEvent::PlayerLeft { .. } => {
                broadcast_game_state(&handle).await;
                broadcast_answer_count(&handle).await;
            }
            GameEvent::ColorSelected {
                player,
                correct,
                score,
            } => {
                broadcast_player_answered(&handle, &player, correct).await;
                broadcast_answer_count(&handle).await;
            }
            GameEvent::NameUpdated { name } => {
                broadcast_game_state(&handle).await;
            }
            _ => {}
        }
    }
}

async fn handle_spotify_state(
    spotify: &Option<Arc<Mutex<SpotifyController>>>,
    state: &GameState,
    current_song: Option<Song>,
) {
    if let Some(spotify) = spotify {
        let mut ctrl = spotify.lock().await.clone();
        match state {
            GameState::Question => {
                if let Some(song) = current_song {
                    if let Err(e) = ctrl.play_track(&song.uri).await {
                        warn!("Could not start playback: {:?}", e);
                    }
                }
            }
            GameState::Score => {
                if let Err(e) = ctrl.pause().await {
                    warn!("Could not pause playback: {:?}", e);
                }
            }
        }
    }
}

async fn send_initial_state(handle: &GameHandle, tx: &mpsc::UnboundedSender<String>) {
    if let Ok(snapshot) = handle.get_state().await {
        let msg = ServerMessage::GameState(GameStateMsg {
            state: game_state_to_string(&snapshot.state),
            score: 0,
            colors: if snapshot.state == GameState::Question {
                Some(snapshot.round_colors)
            } else {
                None
            },
            leaderboard: Some(
                snapshot
                    .players
                    .values()
                    .map(|p| LeaderboardEntry {
                        name: p.name.clone(),
                        score: p.score,
                    })
                    .collect(),
            ),
            round_time_left: snapshot.round_time_left,
            has_answered: false,
            answer: None,
            answered_count: snapshot.players.values().filter(|p| p.has_answered).count(),
            total_players: snapshot.players.len(),
            lobby_id: snapshot.id,
            lobby_name: snapshot.name,
        });
        send_message(tx, &msg);
    }
}

async fn broadcast_game_state(handle: &GameHandle) {
    if let Ok(snapshot) = handle.get_state().await {
        let name = snapshot.name.clone(); // Clone outside the loop
        for player in snapshot.players.values() {
            let msg = ServerMessage::GameState(GameStateMsg {
                state: game_state_to_string(&snapshot.state),
                score: player.score,
                colors: if snapshot.state == GameState::Question {
                    Some(snapshot.round_colors.clone())
                } else {
                    None
                },
                leaderboard: Some(
                    snapshot
                        .players
                        .values()
                        .map(|p| LeaderboardEntry {
                            name: p.name.clone(),
                            score: p.score,
                        })
                        .collect(),
                ),
                round_time_left: snapshot.round_time_left,
                has_answered: player.has_answered,
                answer: player.answer.clone(),
                answered_count: snapshot.players.values().filter(|p| p.has_answered).count(),
                total_players: snapshot.players.len(),
                lobby_id: snapshot.id,
                lobby_name: name.clone(), // Use the cloned name
            });
            send_message(&player.tx, &msg);
        }
    }
}

async fn broadcast_answer_count(handle: &GameHandle) {
    if let Ok(snapshot) = handle.get_state().await {
        let msg = ServerMessage::UpdateAnswerCount(UpdateAnswerCount {
            answered_count: snapshot.players.values().filter(|p| p.has_answered).count(),
            total_players: snapshot.players.len(),
        });

        for player in snapshot.players.values() {
            send_message(&player.tx, &msg);
        }
    }
}

async fn broadcast_player_answered(handle: &GameHandle, player_name: &str, is_correct: bool) {
    if let Ok(snapshot) = handle.get_state().await {
        let msg = ServerMessage::PlayerAnswered(PlayerAnsweredMsg {
            player_name: player_name.to_string(),
            correct: is_correct,
        });

        for player in snapshot.players.values() {
            send_message(&player.tx, &msg);
        }
    }
}

fn send_message(tx: &mpsc::UnboundedSender<String>, msg: &ServerMessage) {
    if let Ok(json) = serde_json::to_string(msg) {
        tx.send(json).ok();
    }
}

fn send_error(tx: &mpsc::UnboundedSender<String>, error: &str) {
    let msg = ServerMessage::Error {
        message: error.to_string(),
    };
    send_message(tx, &msg);
}

fn game_state_to_string(state: &GameState) -> String {
    match state {
        GameState::Score => "score".to_string(),
        GameState::Question => "question".to_string(),
    }
}
