use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::game::GameError;
use crate::game::{GameEvent, GameHandle};
use crate::models::{Song, LobbyInfo};

pub struct GameLobbyManager {
    lobbies: Arc<Mutex<HashMap<Uuid, GameHandle>>>,
}

impl GameLobbyManager {
    pub fn new() -> Self {
        Self {
            lobbies: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn create_lobby(
        &self,
        name: Option<String>,
        songs: Vec<Song>,
    ) -> Result<(GameHandle, mpsc::UnboundedReceiver<GameEvent>), GameError> {
        let (handle, events) = crate::game::GameCore::spawn(name, songs);
        let id = handle.id();
        self.lobbies.lock().await.insert(id, handle.clone());
        Ok((handle, events))
    }

    pub async fn get_lobby(&self, id: Uuid) -> Result<GameHandle, GameError> {
        self.lobbies
            .lock()
            .await
            .get(&id)
            .cloned()
            .ok_or_else(|| GameError::LobbyNotFound(id.to_string()))
    }

    pub async fn get_all_lobbies(&self) -> Vec<LobbyInfo> {
        let lobbies = self.lobbies.lock().await;
        let mut lobby_list = Vec::new();

        for (id, handle) in lobbies.iter() {
            if let Ok(snapshot) = handle.get_state().await {
                lobby_list.push(LobbyInfo {
                    id: *id,
                    name: snapshot.name,
                    player_count: snapshot.players.len(),
                });
            }
        }

        lobby_list
    }
    pub async fn remove_lobby(&self, id: Uuid) -> Result<(), GameError> {
        self.lobbies
            .lock()
            .await
            .remove(&id)
            .map(|_| ())
            .ok_or_else(|| GameError::LobbyNotFound(id.to_string()))
    }
}
