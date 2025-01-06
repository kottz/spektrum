use crate::game::{GameEngine, GameEvent, GameResponse, ResponsePayload, Recipients};
use crate::messages::ServerMessage;
use crate::question::GameQuestion;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
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

    #[error("Player not found")]
    PlayerNotFound,

    #[error("Lobby not found")]
    LobbyNotFound,

    #[error("Cannot reconnect: session expired")]
    SessionExpired,
}

pub type GameResult<T> = Result<T, GameManagerError>;

#[derive(Debug)]
pub struct PlayerConnection {
    pub sender: Option<UnboundedSender<ServerMessage>>,
    pub name: String,
    pub last_seen: Instant,
    pub disconnected_at: Option<Instant>,
}

impl PlayerConnection {
    pub fn new(sender: UnboundedSender<ServerMessage>, name: String) -> Self {
        Self {
            sender: Some(sender),
            name,
            last_seen: Instant::now(),
            disconnected_at: None,
        }
    }

    pub fn mark_disconnected(&mut self) {
        self.sender = None;
        self.disconnected_at = Some(Instant::now());
    }

    pub fn update_connection(&mut self, sender: UnboundedSender<ServerMessage>) {
        self.sender = Some(sender);
        self.last_seen = Instant::now();
        self.disconnected_at = None;
    }

    pub fn is_disconnected(&self) -> bool {
        self.sender.is_none()
    }

    pub fn can_reconnect(&self) -> bool {
        if let Some(disconnected_at) = self.disconnected_at {
            disconnected_at.elapsed() < Duration::from_secs(600)
        } else {
            true
        }
    }
}

/// A single lobby instance that manages its own connection pool and game engine
pub struct GameLobby {
    id: Uuid,
    engine: Arc<RwLock<GameEngine>>,
    connections: Arc<RwLock<HashMap<Uuid, PlayerConnection>>>,
    created_at: Instant,
    last_activity: Arc<RwLock<Instant>>,
}

impl GameLobby {
    pub fn new(
        id: Uuid,
        admin_id: Uuid,
        questions: Vec<GameQuestion>,
        round_duration: u64,
    ) -> Self {
        let now = Instant::now();
        Self {
            id,
            engine: Arc::new(RwLock::new(GameEngine::new(
                admin_id,
                questions,
                round_duration,
            ))),
            connections: Arc::new(RwLock::new(HashMap::new())),
            created_at: now,
            last_activity: Arc::new(RwLock::new(now)),
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
        let mut connections = self
            .connections
            .write()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?;

        if let Some(conn) = connections.get_mut(&player_id) {
            if conn.can_reconnect() {
                conn.update_connection(sender);
                return Ok(());
            }
        }

        connections.insert(player_id, PlayerConnection::new(sender, String::new()));
        self.update_last_activity()?;
        Ok(())
    }

    pub fn mark_player_disconnected(&self, player_id: &Uuid) -> GameResult<()> {
        let mut connections = self
            .connections
            .write()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?;

        if let Some(conn) = connections.get_mut(player_id) {
            conn.mark_disconnected();
            self.update_last_activity()?;
        }
        Ok(())
    }

    pub fn reconnect_player(
        &self,
        player_id: Uuid,
        sender: UnboundedSender<ServerMessage>,
    ) -> GameResult<()> {
        let mut connections = self
            .connections
            .write()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?;

        if let Some(conn) = connections.get_mut(&player_id) {
            if conn.can_reconnect() {
                conn.update_connection(sender);
                self.update_last_activity()?;
                return Ok(());
            }
            return Err(GameManagerError::SessionExpired);
        }
        Err(GameManagerError::PlayerNotFound)
    }

    pub fn process_event(&self, event: GameEvent) -> GameResult<Vec<GameResponse>> {
        self.update_last_activity()?;
        let res = self
            .engine
            .write()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?
            .process_event(event)
            .into_iter()
            .collect::<Vec<_>>();
        Ok(res)
    }

    pub fn update_last_activity(&self) -> GameResult<()> {
        let mut last_activity = self
            .last_activity
            .write()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?;
        *last_activity = Instant::now();
        Ok(())
    }

    pub fn is_inactive(&self) -> GameResult<bool> {
        let last_activity = self
            .last_activity
            .read()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?;

        // Consider inactive if no activity for 1 hour
        Ok(last_activity.elapsed() > Duration::from_secs(3600))
    }

    pub fn is_empty_for_cleanup(&self) -> GameResult<bool> {
        let connections = self
            .connections
            .read()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?;

        let all_long_disconnected = connections.values().all(|conn| {
            if let Some(disconnected_at) = conn.disconnected_at {
                disconnected_at.elapsed() > Duration::from_secs(600)
            } else {
                false
            }
        });

        Ok(all_long_disconnected && self.is_inactive()?)
    }

    pub fn is_admin_connected(&self) -> GameResult<bool> {
        let connections = self
            .connections
            .read()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?;

        Ok(connections.values().any(|conn| conn.name == "admin"))
    }

    pub fn is_empty(&self) -> GameResult<bool> {
        let connections = self
            .connections
            .read()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?;

        Ok(connections.is_empty())
    }

    pub fn get_active_connections(
        &self,
    ) -> GameResult<Vec<(Uuid, UnboundedSender<ServerMessage>)>> {
        let connections = self
            .connections
            .read()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?;

        Ok(connections
            .iter()
            .filter_map(|(id, conn)| conn.sender.clone().map(|sender| (*id, sender)))
            .collect())
    }

    pub fn get_game_state(&self) -> GameResult<GameResponse> {
        let engine = self
            .engine
            .read()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?;

        Ok(GameResponse {
            recipients: Recipients::All, // Caller can override this
            payload: ResponsePayload::StateChanged {
                phase: engine.get_phase(),
                question_type: "".to_string(), // Only needed during question phase
                alternatives: Vec::new(),      // Only needed during question phase
                scoreboard: engine.get_scoreboard(),
            },
        })
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

        // First try 6-digit codes
        for _ in 0..10_000 {
            let code = format!("{:06}", fastrand::u32(0..1_000_000));
            if !join_codes.contains_key(&code) {
                join_codes.insert(code.clone(), lobby_id);
                return Ok(code);
            }
        }

        // If many collisions, escalate to 7 digits
        for _ in 0..1_000_000 {
            let code = format!("{:07}", fastrand::u32(0..10_000_000));
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

        let mut join_codes = self
            .join_codes
            .write()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?;

        if lobbies.remove(id).is_some() {
            join_codes.retain(|_, &mut lobby_id| lobby_id != *id);
        }

        Ok(())
    }

    pub fn cleanup_inactive_lobbies(&self) -> GameResult<()> {
        let lobbies = self
            .lobbies
            .read()
            .map_err(|e| GameManagerError::LockError(e.to_string()))?;

        let mut to_remove = Vec::new();

        for (&id, lobby) in lobbies.iter() {
            if let Ok(true) = lobby.is_empty_for_cleanup() {
                to_remove.push(id);
            }
        }

        drop(lobbies); // Release the read lock

        for id in to_remove {
            if let Err(e) = self.remove_lobby(&id) {
                error!("Failed to remove inactive lobby {}: {}", id, e);
            } else {
                info!("Cleaned up inactive lobby {}", id);
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
