use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Instant,
};
use tokio::sync::mpsc::UnboundedSender;
use tracing::*;
use uuid::Uuid;

use crate::game::{
    ColorDef, EventContext, GameAction, GameEngine, GameEvent, GamePhase, GameResponse, Recipients, ResponsePayload, Song
};

/// A single lobby instance that manages its own connection pool and game engine
pub struct GameLobby {
    id: Uuid,
    engine: Arc<RwLock<GameEngine>>,
    connections: Arc<RwLock<HashMap<Uuid, UnboundedSender<ServerMessage>>>>,
}

impl GameLobby {
    pub fn new(
        id: Uuid,
        admin_id: Uuid,
        songs: Vec<Song>,
        colors: Vec<ColorDef>,
        round_duration: u64,
    ) -> Self {
        Self {
            id,
            engine: Arc::new(RwLock::new(GameEngine::new(
                admin_id,
                songs,
                colors,
                round_duration,
            ))),
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn add_connection(&self, player_id: Uuid, sender: UnboundedSender<ServerMessage>) {
        self.connections.write().unwrap().insert(player_id, sender);
    }

    pub fn remove_connection(&self, player_id: &Uuid) {
        self.connections.write().unwrap().remove(player_id);
    }

    pub fn process_event(&self, event: GameEvent) -> Vec<GameResponse> {
        let mut engine = self.engine.write().unwrap();
        engine.process_event(event)
    }

    pub fn broadcast_responses(&self, responses: Vec<GameResponse>) {
        let connections = self.connections.read().unwrap();

        for response in responses {
            let server_msg = convert_to_server_message(&response.payload);

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

            for &id in &recipient_ids {
                if let Some(sender) = connections.get(&id) {
                    let _ = sender.send(server_msg.clone());
                }
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.connections.read().unwrap().is_empty()
    }
}

/// The GameManager now focuses purely on lobby lifecycle management
pub struct GameManager {
    pub lobbies: Arc<RwLock<HashMap<Uuid, Arc<GameLobby>>>>,
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            lobbies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_lobby(
        &self,
        songs: Vec<Song>,
        colors: Vec<ColorDef>,
        round_duration: u64,
    ) -> (Uuid, Uuid) {
        let lobby_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();

        let lobby = Arc::new(GameLobby::new(
            lobby_id,
            admin_id,
            songs,
            colors,
            round_duration,
        ));

        self.lobbies.write().unwrap().insert(lobby_id, lobby);
        (lobby_id, admin_id)
    }

    pub fn get_lobby(&self, id: &Uuid) -> Option<Arc<GameLobby>> {
        self.lobbies.read().unwrap().get(id).cloned()
    }

    pub fn remove_lobby(&self, id: &Uuid) {
        self.lobbies.write().unwrap().remove(id);
        info!("Removed lobby {}", id);
    }

    pub fn list_lobbies(&self) -> Vec<Uuid> {
        self.lobbies.read().unwrap().keys().cloned().collect()
    }

    pub fn cleanup_empty_lobbies(&self) {
        let to_remove: Vec<Uuid> = {
            let lobbies = self.lobbies.read().unwrap();
            lobbies
                .iter()
                .filter(|(_, lobby)| lobby.is_empty())
                .map(|(&id, _)| id)
                .collect()
        };

        if !to_remove.is_empty() {
            let mut lobbies = self.lobbies.write().unwrap();
            for id in to_remove {
                lobbies.remove(&id);
                info!("Cleaned up empty lobby {}", id);
            }
        }
    }
}

/// Helper function to convert game responses to server messages
fn convert_to_server_message(payload: &ResponsePayload) -> ServerMessage {
    match payload {
        ResponsePayload::Joined {
            player_id,
            name,
            current_players,
        } => ServerMessage::JoinedLobby {
            player_id: *player_id,
            name: name.clone(),
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
        } => ServerMessage::PhaseChanged {
            phase: *phase,
            colors: colors.clone(),
            scoreboard: scoreboard.clone(),
        },
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

// External message protocol (Client <-> Server messages)
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
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
        players: Vec<(String, i32)>,
    },
    PlayerLeft {
        name: String,
    },
    PlayerAnswered {
        name: String,
        correct: bool,
        new_score: i32,
    },
    PhaseChanged {
        phase: GamePhase,
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

// Convert client messages to game events
impl ClientMessage {
    pub fn into_game_event(self, sender_id: Uuid, is_admin: bool) -> GameEvent {
        let context = EventContext {
            lobby_id: match &self {
                ClientMessage::JoinLobby { lobby_id, .. } => *lobby_id,
                ClientMessage::Leave { lobby_id } => *lobby_id,
                ClientMessage::Answer { lobby_id, .. } => *lobby_id,
                ClientMessage::AdminAction { lobby_id, .. } => *lobby_id,
            },
            sender_id,
            timestamp: Instant::now(),
            is_admin,
        };

        let action = match self {
            ClientMessage::JoinLobby { name, .. } => GameAction::Join { name },
            ClientMessage::Leave { .. } => GameAction::Leave,
            ClientMessage::Answer { color, .. } => GameAction::Answer { color },
            ClientMessage::AdminAction { action, .. } => match action {
                AdminAction::StartGame => GameAction::StartGame,
                AdminAction::StartRound { colors } => GameAction::StartRound {
                    specified_colors: colors,
                },
                AdminAction::EndRound => GameAction::EndRound,
                AdminAction::EndGame { reason } => GameAction::EndGame { reason },
                AdminAction::CloseGame { reason } => GameAction::CloseGame { reason },
            },
        };

        GameEvent { context, action }
    }
}
