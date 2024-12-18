use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

use crate::game::{ColorDef, GameEngine, InputEvent, OutputEvent, OutputEventData, Song};

/// A single "manager" event that either creates a lobby or forwards an `InputEvent`
/// to a specific lobby.
pub enum ManagerEvent {
    /// Create a new lobby with optional parameters like custom songs/colors.
    CreateLobby {
        songs: Vec<Song>,
        colors: Vec<ColorDef>,
        round_duration: u64,
    },
    /// An event directed at an existing lobby.
    LobbyEvent { lobby_id: Uuid, event: InputEvent },
}

/// We include the original `OutputEvent` plus the `lobby_id` for easy routing
/// back to the correct set of end users.
pub struct ManagerOutput {
    pub lobby_id: Uuid,
    pub event: OutputEvent,
}

/// The GameManager holds many active lobbies in a HashMap.
///
/// Each entry is: `lobby_id -> GameEngine`.
///
/// You might store this in an `Arc<Mutex<GameManager>>` if you want concurrency.
pub struct GameManager {
    pub lobbies: HashMap<Uuid, GameEngine>,
}

impl GameManager {
    /// Creates an empty manager.
    pub fn new() -> Self {
        Self {
            lobbies: HashMap::new(),
        }
    }

    /// Handle a `ManagerEvent`. If it's `CreateLobby`, this will create a new `GameEngine`
    /// with a fresh `lobby_id`. If it's a `LobbyEvent`, it finds the correct engine and
    /// updates it. Returns a list of `ManagerOutput` for the caller to broadcast.
    ///
    /// `now` is typically `Instant::now()` passed in from external code,
    /// so the game logic can handle timing.
    pub fn update(&mut self, manager_event: ManagerEvent, now: Instant) -> Vec<ManagerOutput> {
        match manager_event {
            ManagerEvent::CreateLobby {
                songs,
                colors,
                round_duration,
            } => {
                let new_lobby_id = Uuid::new_v4();
                let engine = GameEngine::new(new_lobby_id, songs, colors, round_duration);
                self.lobbies.insert(new_lobby_id, engine);

                // You might want to return an acknowledgement event or just an empty vec.
                // For example, an external code can broadcast "LobbyCreated" to the user.
                vec![]
            }

            ManagerEvent::LobbyEvent { lobby_id, event } => {
                let mut outputs = Vec::new();

                // Find the engine
                if let Some(engine) = self.lobbies.get_mut(&lobby_id) {
                    // Run the event through the engine
                    let engine_outputs = engine.update(event, now);

                    // Convert them to ManagerOutput, tagging the lobby_id
                    for evt in engine_outputs {
                        // If the engine told us the game is closed, remove the lobby
                        match &evt.data {
                            OutputEventData::GameClosed { reason: _ } => {
                                // We'll remove the lobby AFTER collecting the outputs so that
                                // the outside code can still send the "GameClosed" notification
                                // to all players.
                                // The removal is done below.
                            }
                            _ => {}
                        }

                        outputs.push(ManagerOutput {
                            lobby_id,
                            event: evt,
                        });
                    }

                    // If any of the engine outputs was a GameClosed event, remove the lobby
                    let remove_lobby = outputs
                        .iter()
                        .any(|o| matches!(o.event.data, OutputEventData::GameClosed { .. }));

                    if remove_lobby {
                        self.lobbies.remove(&lobby_id);
                    }

                    outputs
                } else {
                    // If there's no lobby matching the given ID, we can return an error event.
                    // Possibly return one output event that signals "Lobby not found".
                    vec![ManagerOutput {
                        lobby_id,
                        event: crate::game::OutputEvent {
                            recipient: Uuid::nil(), // or use the same sender_id that sent the event
                            data: crate::game::OutputEventData::Error {
                                message: "Lobby not found".to_string(),
                            },
                        },
                    }]
                }
            }
        }
    }
}
