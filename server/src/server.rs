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
use tracing::{info, warn};
use uuid::Uuid;

use crate::game::{GameError, GameEvent, GameHandle, GameState};
use crate::game_manager::GameLobbyManager;
use crate::models::{
    ClientMessage, ColorResult, GameStateMsg, LeaderboardEntry, LobbyCreateRequest, LobbyInfo,
    PlayerAnsweredMsg, ServerMessage, Song, UpdateAnswerCount,
};
use crate::spotify::SpotifyController;

#[derive(Debug)]
enum ClientErrorMessage {
    InvalidAction,
    JoinFailed,
    AnswerRejected,
    GameStateFailed,
    NotAuthorized,
    InvalidInput,
    ServerError,
}

impl ClientErrorMessage {
    fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidAction => "This action is not allowed",
            Self::JoinFailed => "Unable to join the game",
            Self::AnswerRejected => "Your answer could not be processed",
            Self::GameStateFailed => "Failed to update game state",
            Self::NotAuthorized => "You are not authorized to perform this action",
            Self::InvalidInput => "Invalid input provided",
            Self::ServerError => "An unexpected error occurred",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WebSocketError {
    #[error("Failed to parse message: {0}")]
    MessageParse(#[from] serde_json::Error),
    #[error("Game error: {0}")]
    Game(#[from] GameError),
}

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
) -> Result<impl IntoResponse, AppError> {
    let lobby_id = params
        .get("lobby")
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(|| GameError::LobbyNotFound("Invalid lobby ID".to_string()))?;

    let is_admin = params.get("role").map_or(false, |role| role == "admin");
    let handle = state.game_manager.get_lobby(lobby_id).await?;

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, handle, is_admin)))
}

async fn handle_socket(socket: WebSocket, handle: GameHandle, is_admin: bool) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let mut player_name: Option<String> = None;

    // Forward messages from game to websocket
    let forward_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_tx.send(Message::Text(msg)).await {
                warn!("WebSocket send error: {:?}", e);
                break;
            }
        }
    });

    // Handle incoming WebSocket messages
    while let Some(msg_result) = ws_rx.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                if let Err(e) =
                    handle_client_message(&text, &handle, &tx, &mut player_name, is_admin).await
                {
                    warn!("Error handling client message: {:?}", e);
                    send_error(&tx, ClientErrorMessage::ServerError.as_str());
                }
            }
            Ok(Message::Close(_)) => {
                info!("Client initiated close connection");
                break;
            }
            Ok(Message::Ping(_)) => continue,
            Ok(Message::Pong(_)) => continue,
            Ok(Message::Binary(_)) => {
                warn!("Received unexpected binary message");
                send_error(&tx, ClientErrorMessage::InvalidInput.as_str());
            }
            Err(e) => {
                warn!("WebSocket receive error: {:?}", e);
                break;
            }
        }
    }

    if let Some(name) = player_name {
        if !is_admin {
            if let Err(e) = handle.remove_player(name).await {
                warn!("Error removing player on disconnect: {:?}", e);
            }
        }
    }
    forward_task.abort();
}

async fn handle_client_message(
    text: &str,
    handle: &GameHandle,
    tx: &mpsc::UnboundedSender<String>,
    player_name: &mut Option<String>,
    is_admin: bool,
) -> Result<(), WebSocketError> {
    let msg: ClientMessage = serde_json::from_str(text).map_err(|e| {
        warn!("Failed to parse client message: {:?}", e);
        WebSocketError::MessageParse(e)
    })?;

    match msg {
        ClientMessage::ToggleState { specified_colors } if is_admin => {
            match handle.toggle_state(specified_colors).await {
                Ok(new_state) => {
                    let response = ServerMessage::StateUpdated { state: new_state };
                    send_message_safe(tx, &response);
                }
                Err(e) => {
                    warn!("Error toggling state: {:?}", e);
                    send_error(tx, ClientErrorMessage::GameStateFailed.as_str());
                }
            }
        }

        ClientMessage::Join { name } if !is_admin => {
            match handle.add_player(name.clone(), tx.clone()).await {
                Ok(()) => {
                    *player_name = Some(name);
                    if let Err(e) = send_initial_state(handle, tx).await {
                        warn!("Error sending initial state: {:?}", e);
                        send_error(tx, ClientErrorMessage::GameStateFailed.as_str());
                    }
                }
                Err(e) => {
                    warn!("Error adding player: {:?}", e);
                    send_error(tx, ClientErrorMessage::JoinFailed.as_str());
                    return Err(WebSocketError::Game(e));
                }
            }
        }

        ClientMessage::SelectColor { color } if !is_admin => {
            if let Some(name) = player_name {
                match handle.answer_color(name.clone(), color).await {
                    Ok((correct, score)) => {
                        let response = ServerMessage::ColorResult(ColorResult {
                            correct,
                            score,
                            total_score: score,
                        });
                        send_message_safe(tx, &response);
                    }
                    Err(e) => {
                        warn!("Error processing color selection: {:?}", e);
                        send_error(tx, ClientErrorMessage::AnswerRejected.as_str());
                    }
                }
            } else {
                send_error(tx, ClientErrorMessage::NotAuthorized.as_str());
            }
        }

        ClientMessage::Join { name: _ } if is_admin => {
            *player_name = Some("Admin".to_string());
            if let Err(e) = send_initial_state(handle, tx).await {
                warn!("Error sending initial state to admin: {:?}", e);
                send_error(tx, ClientErrorMessage::GameStateFailed.as_str());
            }
        }

        _ => {
            warn!("Invalid action received: {:?}", msg);
            send_error(tx, ClientErrorMessage::InvalidAction.as_str());
        }
    }

    Ok(())
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
                if let Err(e) = broadcast_game_state(&handle).await {
                    warn!("Failed to broadcast game state: {:?}", e);
                }
            }
            GameEvent::PlayerJoined { .. } | GameEvent::PlayerLeft { .. } => {
                if let Err(e) = broadcast_game_state(&handle).await {
                    warn!("Failed to broadcast game state: {:?}", e);
                }
                if let Err(e) = broadcast_answer_count(&handle).await {
                    warn!("Failed to broadcast answer count: {:?}", e);
                }
            }
            GameEvent::ColorSelected {
                player,
                correct,
                score: _,
            } => {
                if let Err(e) = broadcast_player_answered(&handle, &player, correct).await {
                    warn!("Failed to broadcast player answered: {:?}", e);
                }
                if let Err(e) = broadcast_answer_count(&handle).await {
                    warn!("Failed to broadcast answer count: {:?}", e);
                }
            }
            GameEvent::NameUpdated { name: _ } => {
                if let Err(e) = broadcast_game_state(&handle).await {
                    warn!("Failed to broadcast game state: {:?}", e);
                }
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
    if let Some(spotify) = spotify.clone() {
        match state {
            GameState::Question => {
                if let Some(song) = current_song {
                    let song_uri = song.uri.clone();
                    tokio::spawn(async move {
                        let mut ctrl = spotify.lock().await.clone();
                        if let Err(e) = ctrl.play_track(&song_uri).await {
                            warn!("Could not start playback: {:?}", e);
                        }
                    });
                }
            }
            GameState::Score => {
                tokio::spawn(async move {
                    let mut ctrl = spotify.lock().await.clone();
                    if let Err(e) = ctrl.pause().await {
                        warn!("Could not pause playback: {:?}", e);
                    }
                });
            }
        }
    }
}

async fn send_initial_state(
    handle: &GameHandle,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<(), WebSocketError> {
    let snapshot = handle.get_state().await.map_err(WebSocketError::Game)?;
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
    send_message_safe(tx, &msg);
    Ok(())
}

async fn broadcast_game_state(handle: &GameHandle) -> Result<(), WebSocketError> {
    let snapshot = handle.get_state().await.map_err(WebSocketError::Game)?;
    let name = snapshot.name.clone();

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
            lobby_name: name.clone(),
        });
        send_message_safe(&player.tx, &msg);
    }
    Ok(())
}

async fn broadcast_answer_count(handle: &GameHandle) -> Result<(), WebSocketError> {
    let snapshot = handle.get_state().await.map_err(WebSocketError::Game)?;
    let msg = ServerMessage::UpdateAnswerCount(UpdateAnswerCount {
        answered_count: snapshot.players.values().filter(|p| p.has_answered).count(),
        total_players: snapshot.players.len(),
    });

    for player in snapshot.players.values() {
        send_message_safe(&player.tx, &msg);
    }
    Ok(())
}

async fn broadcast_player_answered(
    handle: &GameHandle,
    player_name: &str,
    is_correct: bool,
) -> Result<(), WebSocketError> {
    let snapshot = handle.get_state().await.map_err(WebSocketError::Game)?;
    let msg = ServerMessage::PlayerAnswered(PlayerAnsweredMsg {
        player_name: player_name.to_string(),
        correct: is_correct,
    });

    for player in snapshot.players.values() {
        send_message_safe(&player.tx, &msg);
    }
    Ok(())
}

fn send_message_safe(tx: &mpsc::UnboundedSender<String>, msg: &ServerMessage) {
    match serde_json::to_string(msg) {
        Ok(json) => {
            if let Err(e) = tx.send(json) {
                warn!("Failed to send message through channel: {:?}", e);
            }
        }
        Err(e) => {
            warn!("Failed to serialize message: {:?}", e);
        }
    }
}

fn send_error(tx: &mpsc::UnboundedSender<String>, error: &str) {
    let msg = ServerMessage::Error {
        message: error.to_string(),
    };
    send_message_safe(tx, &msg);
}

fn game_state_to_string(state: &GameState) -> String {
    match state {
        GameState::Score => "score".to_string(),
        GameState::Question => "question".to_string(),
    }
}
