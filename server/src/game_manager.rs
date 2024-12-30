use crate::game::{GameEngine, GameEvent, GameResponse, Recipients};
use crate::messages::{convert_to_server_message, ServerMessage};
use crate::question::GameQuestion;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;
use tracing::*;
use uuid::Uuid;

/// Example error type for GameManager
#[derive(Error, Debug)]
pub enum GameManagerError {
    #[error("Failed to acquire lock: {0}")]
    LockError(String),
    #[error("Out of join codes.")]
    OutOfJoinCodes,
}

/// A single lobby instance that manages its own connection pool and game engine
pub struct GameLobby {
    id: Uuid,
    engine: Arc<RwLock<GameEngine>>,
    pub connections: Arc<RwLock<HashMap<Uuid, UnboundedSender<ServerMessage>>>>,
}

impl GameLobby {
    pub fn new(
        id: Uuid,
        admin_id: Uuid,
        questions: Vec<GameQuestion>,
        round_duration: u64,
    ) -> Self {
        Self {
            id,
            engine: Arc::new(RwLock::new(GameEngine::new(
                admin_id,
                questions,
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
    pub join_codes: Arc<RwLock<HashMap<String, Uuid>>>,
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            lobbies: Arc::new(RwLock::new(HashMap::new())),
            join_codes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn generate_join_code(&self, lobby_id: Uuid) -> Result<String, GameManagerError> {
        let mut join_codes = self
            .join_codes
            .write()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?;

        // First try 6-digit codes up to a limit.
        for _ in 0..10_000 {
            let code = format!("{:06}", fastrand::u32(0..1_000_000));
            if !join_codes.contains_key(&code) {
                join_codes.insert(code.clone(), lobby_id);
                return Ok(code);
            }
        }

        // If many collisions or lobbies, escalate to 7 digits.
        for _ in 0..1_000_000 {
            let code = format!("{:07}", fastrand::u32(0..10_000_00));
            if !join_codes.contains_key(&code) {
                join_codes.insert(code.clone(), lobby_id);
                return Ok(code);
            }
        }

        Err(GameManagerError::OutOfJoinCodes)
    }

    pub fn create_lobby(
        &self,
        questions: Vec<GameQuestion>,
        round_duration: u64,
    ) -> Result<(Uuid, String, Uuid), GameManagerError> {
        let lobby_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();

        let lobby = Arc::new(GameLobby::new(
            lobby_id,
            admin_id,
            questions,
            round_duration,
        ));

        let join_code = self.generate_join_code(lobby_id)?;
        self.lobbies.write().unwrap().insert(lobby_id, lobby);

        Ok((lobby_id, join_code, admin_id))
    }

    pub fn get_lobby(&self, id: &Uuid) -> Option<Arc<GameLobby>> {
        self.lobbies.read().unwrap().get(id).cloned()
    }

    pub fn get_lobby_id_from_join_code(&self, join_code: &str) -> Option<Uuid> {
        let bind = self.join_codes.read().unwrap();
        bind.get(join_code).copied()
    }

    pub fn remove_lobby(&self, id: &Uuid) {
        self.lobbies.write().unwrap().remove(id);
    }

    fn _list_lobbies(&self) -> Vec<Uuid> {
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
