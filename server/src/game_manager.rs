use crate::game::{GameEngine, GameEvent, GameResponse};
use crate::messages::ServerMessage;
use crate::question::GameQuestion;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;
use tracing::*;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum GameManagerError {
    #[error("Failed to acquire lock: {0}")]
    LockError(String),

    #[error("Out of join codes")]
    OutOfJoinCodes,
}

pub type GameResult<T> = Result<T, GameManagerError>;

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

    pub fn add_connection(
        &self,
        player_id: Uuid,
        sender: UnboundedSender<ServerMessage>,
    ) -> GameResult<()> {
        self.connections
            .write()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?
            .insert(player_id, sender);
        Ok(())
    }

    pub fn remove_connection(&self, player_id: &Uuid) -> GameResult<()> {
        self.connections
            .write()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?
            .remove(player_id);
        Ok(())
    }

    pub fn process_event(&self, event: GameEvent) -> GameResult<Vec<GameResponse>> {
        let res = self
            .engine
            .write()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?
            .process_event(event)
            .into_iter()
            .collect::<Vec<_>>();
        Ok(res)
    }

    pub fn is_empty(&self) -> GameResult<bool> {
        Ok(self
            .connections
            .read()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?
            .is_empty())
    }
}

/// The GameManager focuses purely on lobby lifecycle management
pub struct GameManager {
    lobbies: Arc<RwLock<HashMap<Uuid, Arc<GameLobby>>>>,
    join_codes: Arc<RwLock<HashMap<String, Uuid>>>,
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            lobbies: Arc::new(RwLock::new(HashMap::new())),
            join_codes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn generate_join_code(&self, lobby_id: Uuid) -> GameResult<String> {
        let mut join_codes = self
            .join_codes
            .write()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?;

        // First try 6-digit codes up to a limit
        for _ in 0..10_000 {
            let code = format!("{:06}", fastrand::u32(0..1_000_000));
            if !join_codes.contains_key(&code) {
                join_codes.insert(code.clone(), lobby_id);
                return Ok(code);
            }
        }

        // If many collisions or lobbies, escalate to 7 digits
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
    ) -> GameResult<(Uuid, String, Uuid)> {
        let lobby_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();

        let lobby = Arc::new(GameLobby::new(
            lobby_id,
            admin_id,
            questions,
            round_duration,
        ));

        let join_code = self.generate_join_code(lobby_id)?;

        self.lobbies
            .write()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?
            .insert(lobby_id, lobby);

        Ok((lobby_id, join_code, admin_id))
    }

    pub fn get_lobby(&self, id: &Uuid) -> GameResult<Option<Arc<GameLobby>>> {
        Ok(self
            .lobbies
            .read()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?
            .get(id)
            .cloned())
    }

    pub fn get_lobby_id_from_join_code(&self, join_code: &str) -> GameResult<Option<Uuid>> {
        Ok(self
            .join_codes
            .read()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?
            .get(join_code)
            .copied())
    }

    pub fn remove_lobby(&self, id: &Uuid) -> GameResult<()> {
        let mut lobbies = self
            .lobbies
            .write()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?;

        // Also remove from join codes
        if lobbies.remove(id).is_some() {
            let mut join_codes = self
                .join_codes
                .write()
                .map_err(|e| GameManagerError::LockError(e.to_string()))?;

            // Find and remove any join codes pointing to this lobby
            let codes_to_remove: Vec<String> = join_codes
                .iter()
                .filter(|(_, &lobby_id)| lobby_id == *id)
                .map(|(code, _)| code.clone())
                .collect();

            for code in codes_to_remove {
                join_codes.remove(&code);
            }
        }

        Ok(())
    }

    fn _list_lobbies(&self) -> GameResult<Vec<Uuid>> {
        Ok(self
            .lobbies
            .read()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?
            .keys()
            .copied()
            .collect())
    }

    fn _cleanup_empty_lobbies(&self) -> GameResult<()> {
        let to_remove: Vec<Uuid> = {
            let lobbies = self
                .lobbies
                .read()
                .map_err(|e| GameManagerError::LockError(e.to_string()))?;

            let mut empty_lobbies = Vec::new();
            for (&id, lobby) in lobbies.iter() {
                if lobby.is_empty()? {
                    empty_lobbies.push(id);
                }
            }
            empty_lobbies
        };

        if !to_remove.is_empty() {
            for id in to_remove {
                self.remove_lobby(&id)?;
                info!("Cleaned up empty lobby {}", id);
            }
        }

        Ok(())
    }
}

impl Default for GameManager {
    fn default() -> Self {
        Self::new()
    }
}
