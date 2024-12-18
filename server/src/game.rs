use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};
use uuid::Uuid;

/// Represents the different phases or states the game might be in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamePhase {
    Lobby,
    Score,
    Question,
    GameOver,
}

/// Holds information about each color option in the game.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ColorDef {
    pub name: String,
    pub rgb: String,
}

/// Represents a single song.
#[derive(Clone, Debug)]
pub struct Song {
    pub id: u32,
    pub song_name: String,
    pub artist: String,
    pub uri: String,
    pub colors: Vec<String>,
}

/// Tracks an individual player's state.
#[derive(Clone, Debug)]
pub struct PlayerState {
    pub player_id: Uuid,
    pub name: String,
    pub score: i32,
    pub has_answered: bool,
    pub answer: Option<String>,
    pub is_admin: bool,
}

/// Special operations that can be performed during state transitions
#[derive(Clone, Debug, Deserialize)]
pub enum StateOperation {
    StartGame,         // Start the game from lobby
    ToggleQuestion,    // Toggle between Score and Question phases
    EndGame(String),   // End the game with reason
    CloseGame(String), // Close the game instance with reason
}

/// Enumerates all possible incoming events for the game engine.
#[derive(Clone, Debug)]
pub enum InputEvent {
    Join {
        sender_id: Uuid,
        name: String,
        is_admin: bool,
    },
    Leave {
        sender_id: Uuid,
    },
    Answer {
        sender_id: Uuid,
        color: String,
    },
    ToggleState {
        sender_id: Uuid,
        specified_colors: Option<Vec<String>>,
        operation: StateOperation,
    },
}

/// Enumerates all possible outgoing events after processing an `InputEvent`.
#[derive(Clone, Debug)]
pub struct OutputEvent {
    pub recipient: Uuid,
    pub data: OutputEventData,
}

/// Actual content of the outgoing event.
#[derive(Clone, Debug)]
pub enum OutputEventData {
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
        new_phase: GamePhase,
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

/// Main game state structure.
#[derive(Debug)]
pub struct GameState {
    pub lobby_id: Uuid,
    pub players: HashMap<Uuid, PlayerState>,
    pub phase: GamePhase,
    pub round_start_time: Option<Instant>,
    pub round_duration: u64,

    /// Current set of active colors in a question round.
    pub colors: Vec<ColorDef>,
    /// Subset of colors that are correct answers this round.
    pub correct_colors: Vec<String>,

    /// Optional reference to the current song.
    pub current_song: Option<Song>,
    /// Track used songs to avoid repetition.
    pub used_songs: HashSet<String>,

    /// All possible songs loaded for this game instance.
    pub all_songs: Vec<Song>,
    /// Complete set of color definitions.
    pub all_colors: Vec<ColorDef>,
}

/// The game engine wrapper that processes events against the `GameState`.
pub struct GameEngine {
    state: GameState,
}

impl GameEngine {
    /// Creates a new GameEngine with a fresh `GameState`.
    pub fn new(
        lobby_id: Uuid,
        all_songs: Vec<Song>,
        all_colors: Vec<ColorDef>,
        round_duration: u64,
    ) -> Self {
        Self {
            state: GameState {
                lobby_id,
                players: HashMap::new(),
                phase: GamePhase::Lobby,
                round_start_time: None,
                round_duration,

                colors: Vec::new(),
                correct_colors: Vec::new(),

                current_song: None,
                used_songs: HashSet::new(),

                all_songs,
                all_colors,
            },
        }
    }

    /// Processes an input event and returns a list of output events.
    pub fn update(&mut self, event: InputEvent, now: Instant) -> Vec<OutputEvent> {
        let mut outputs = Vec::new();

        match event {
            InputEvent::Join {
                sender_id,
                name,
                is_admin,
            } => {
                if self.state.players.contains_key(&sender_id) {
                    outputs.push(OutputEvent {
                        recipient: sender_id,
                        data: OutputEventData::Error {
                            message: "Player ID already exists.".to_string(),
                        },
                    });
                    return outputs;
                }

                let player = PlayerState {
                    player_id: sender_id,
                    name: name.clone(),
                    score: 0,
                    has_answered: false,
                    answer: None,
                    is_admin,
                };
                self.state.players.insert(sender_id, player);

                // Send initial player list to the new player
                let player_list: Vec<(String, i32)> = self
                    .state
                    .players
                    .values()
                    .map(|p| (p.name.clone(), p.score))
                    .collect();

                outputs.push(OutputEvent {
                    recipient: sender_id,
                    data: OutputEventData::InitialPlayerList {
                        players: player_list,
                    },
                });

                // Notify others about the new player
                for (&pid, _) in &self.state.players {
                    if pid != sender_id {
                        outputs.push(OutputEvent {
                            recipient: pid,
                            data: OutputEventData::PlayerJoined {
                                player_name: name.clone(),
                                current_score: 0,
                            },
                        });
                    }
                }
            }

            InputEvent::Leave { sender_id } => {
                if let Some(removed) = self.state.players.remove(&sender_id) {
                    for &pid in &self.state.players.keys().cloned().collect::<Vec<_>>() {
                        outputs.push(OutputEvent {
                            recipient: pid,
                            data: OutputEventData::PlayerLeft {
                                player_name: removed.name.clone(),
                            },
                        });
                    }
                } else {
                    outputs.push(OutputEvent {
                        recipient: sender_id,
                        data: OutputEventData::Error {
                            message: "Player not found".to_string(),
                        },
                    });
                }
            }

            InputEvent::Answer { sender_id, color } => {
                if self.state.phase != GamePhase::Question {
                    outputs.push(OutputEvent {
                        recipient: sender_id,
                        data: OutputEventData::Error {
                            message: "Not in Question phase".to_string(),
                        },
                    });
                    return outputs;
                }

                let (player_name, correct, new_score) = {
                    let player = match self.state.players.get_mut(&sender_id) {
                        Some(p) => p,
                        None => {
                            outputs.push(OutputEvent {
                                recipient: sender_id,
                                data: OutputEventData::Error {
                                    message: "Player not found".to_string(),
                                },
                            });
                            return outputs;
                        }
                    };

                    if player.has_answered {
                        outputs.push(OutputEvent {
                            recipient: sender_id,
                            data: OutputEventData::Error {
                                message: "Player has already answered".to_string(),
                            },
                        });
                        return outputs;
                    }

                    let time_is_up = if let Some(start) = self.state.round_start_time {
                        now.duration_since(start).as_secs() > self.state.round_duration
                    } else {
                        false
                    };

                    let mut correct = false;
                    let mut new_score = player.score;

                    if time_is_up {
                        correct = false;
                    } else {
                        correct = self.state.correct_colors.contains(&color);
                        if correct {
                            let elapsed = now
                                .duration_since(self.state.round_start_time.unwrap())
                                .as_secs_f64();
                            let calc_score = (5000.0 - (elapsed * 100.0)).max(0.0) as i32;
                            new_score += calc_score;
                            player.score = new_score;
                        }
                    }

                    player.has_answered = true;
                    player.answer = Some(color);

                    (player.name.clone(), correct, new_score)
                };

                for &pid in self.state.players.keys() {
                    outputs.push(OutputEvent {
                        recipient: pid,
                        data: OutputEventData::PlayerAnswered {
                            player_name: player_name.clone(),
                            correct,
                            new_score: if pid == sender_id { new_score } else { 0 },
                        },
                    });
                }
            }

            InputEvent::ToggleState {
                sender_id,
                specified_colors,
                operation,
            } => {
                if !self
                    .state
                    .players
                    .get(&sender_id)
                    .map_or(false, |p| p.is_admin)
                {
                    outputs.push(OutputEvent {
                        recipient: sender_id,
                        data: OutputEventData::Error {
                            message: "Not authorized (admin only)".to_string(),
                        },
                    });
                    return outputs;
                }

                match operation {
                    StateOperation::StartGame => {
                        if self.state.phase != GamePhase::Lobby {
                            outputs.push(OutputEvent {
                                recipient: sender_id,
                                data: OutputEventData::Error {
                                    message: "Can only start game from lobby".to_string(),
                                },
                            });
                            return outputs;
                        }
                        self.state.phase = GamePhase::Score;
                        for &pid in self.state.players.keys() {
                            outputs.push(OutputEvent {
                                recipient: pid,
                                data: OutputEventData::StateChanged {
                                    new_phase: GamePhase::Score,
                                    colors: Vec::new(),
                                },
                            });
                        }
                        return outputs;
                    }
                    StateOperation::EndGame(reason) => {
                        if self.state.phase == GamePhase::GameOver {
                            outputs.push(OutputEvent {
                                recipient: sender_id,
                                data: OutputEventData::Error {
                                    message: "Game is already over".to_string(),
                                },
                            });
                            return outputs;
                        }
                        self.transition_to_game_over(reason, &mut outputs);
                        return outputs;
                    }
                    StateOperation::CloseGame(reason) => {
                        for &pid in self.state.players.keys() {
                            outputs.push(OutputEvent {
                                recipient: pid,
                                data: OutputEventData::GameClosed {
                                    reason: reason.clone(),
                                },
                            });
                        }
                        return outputs;
                    }
                    StateOperation::ToggleQuestion => match self.state.phase {
                        GamePhase::Lobby => {
                            outputs.push(OutputEvent {
                                recipient: sender_id,
                                data: OutputEventData::Error {
                                    message: "Must use StartGame operation to leave lobby"
                                        .to_string(),
                                },
                            });
                        }
                        GamePhase::Score => {
                            if self.state.used_songs.len() >= self.state.all_songs.len() {
                                self.transition_to_game_over(
                                    "All songs have been used".to_string(),
                                    &mut outputs,
                                );
                                return outputs;
                            }

                            let result = self.start_new_round(now, specified_colors);
                            match result {
                                Ok(()) => {
                                    for &pid in self.state.players.keys() {
                                        outputs.push(OutputEvent {
                                            recipient: pid,
                                            data: OutputEventData::StateChanged {
                                                new_phase: GamePhase::Question,
                                                colors: self.state.colors.clone(),
                                            },
                                        });
                                    }
                                }
                                Err(err) => {
                                    outputs.push(OutputEvent {
                                        recipient: sender_id,
                                        data: OutputEventData::Error { message: err },
                                    });
                                }
                            }
                        }
                        GamePhase::Question => {
                            self.end_round();
                            for &pid in self.state.players.keys() {
                                outputs.push(OutputEvent {
                                    recipient: pid,
                                    data: OutputEventData::StateChanged {
                                        new_phase: GamePhase::Score,
                                        colors: Vec::new(),
                                    },
                                });
                            }
                        }
                        GamePhase::GameOver => {
                            outputs.push(OutputEvent {
                                recipient: sender_id,
                                data: OutputEventData::Error {
                                    message: "Game is already over".to_string(),
                                },
                            });
                        }
                    },
                }
            }
        }

        outputs
    }

    /// Move from Score -> Question by setting up new round data.
    fn start_new_round(
        &mut self,
        now: Instant,
        specified_colors: Option<Vec<String>>,
    ) -> Result<(), String> {
        self.state.colors.clear();
        self.state.correct_colors.clear();

        // Reset players for a new round
        for player in self.state.players.values_mut() {
            player.has_answered = false;
            player.answer = None;
        }

        // If admin specified custom colors, use them
        if let Some(specs) = specified_colors {
            let mut chosen: Vec<ColorDef> = Vec::new();
            for cname in &specs {
                if let Some(col) = self
                    .state
                    .all_colors
                    .iter()
                    .find(|c| c.name.eq_ignore_ascii_case(cname))
                {
                    chosen.push(col.clone());
                }
            }
            if chosen.is_empty() {
                return Err("No valid specified colors".to_string());
            }
            self.state.colors = chosen.clone();
            self.state.correct_colors = chosen.iter().map(|c| c.name.clone()).collect();
            self.state.current_song = None;
        } else {
            // Pick a random unused song
            let available_songs: Vec<_> = self
                .state
                .all_songs
                .iter()
                .filter(|s| !self.state.used_songs.contains(&s.uri))
                .cloned()
                .collect();

            if available_songs.is_empty() {
                return Err("No available songs".to_string());
            }
            let chosen_song = match available_songs.choose(&mut rand::thread_rng()) {
                Some(s) => s.clone(),
                None => return Err("No available songs".to_string()),
            };
            self.state.current_song = Some(chosen_song.clone());
            self.state.used_songs.insert(chosen_song.uri.clone());
            println!("Chose song: {}", chosen_song.song_name);

            // Gather correct colors from the chosen song
            let mut chosen_correct_colors = Vec::new();
            for cname in &chosen_song.colors {
                if let Some(cdef) = self
                    .state
                    .all_colors
                    .iter()
                    .find(|c| c.name.eq_ignore_ascii_case(cname))
                {
                    chosen_correct_colors.push(cdef.clone());
                }
            }
            if chosen_correct_colors.is_empty() {
                return Err("Song has no valid colors".to_string());
            }
            self.state.correct_colors = chosen_correct_colors
                .iter()
                .map(|c| c.name.clone())
                .collect();
            self.state.colors = self.setup_round_colors(chosen_correct_colors);
        }

        self.state.phase = GamePhase::Question;
        self.state.round_start_time = Some(now);
        Ok(())
    }

    fn setup_round_colors(&self, chosen_correct_colors: Vec<ColorDef>) -> Vec<ColorDef> {
        let mut round_colors = Vec::new();
        // Add all correct colors first
        round_colors.extend(chosen_correct_colors.clone());

        // Create excluded set based on correct colors
        let mut excluded = HashSet::new();
        if chosen_correct_colors
            .iter()
            .any(|cc| ["Yellow", "Gold", "Orange"].contains(&cc.name.as_str()))
        {
            excluded.extend(["Yellow", "Gold", "Orange"]);
        }
        if chosen_correct_colors
            .iter()
            .any(|cc| ["Silver", "Gray"].contains(&cc.name.as_str()))
        {
            excluded.extend(["Silver", "Gray"]);
        }

        // Get available colors (not in correct colors, not excluded)
        let mut available: Vec<_> = self
            .state
            .all_colors
            .iter()
            .filter(|col| !chosen_correct_colors.contains(col))
            .filter(|col| !excluded.contains(col.name.as_str()))
            .cloned()
            .collect();

        // Add random colors until we reach 6 or run out of available colors
        while round_colors.len() < 6 && !available.is_empty() {
            let idx = rand::random::<usize>() % available.len();
            let chosen_color = available.remove(idx);
            round_colors.push(chosen_color.clone());

            // Update excluded colors based on what we just added
            if ["Yellow", "Gold", "Orange"].contains(&chosen_color.name.as_str()) {
                available.retain(|c| !["Yellow", "Gold", "Orange"].contains(&c.name.as_str()));
            } else if ["Silver", "Gray"].contains(&chosen_color.name.as_str()) {
                available.retain(|c| !["Silver", "Gray"].contains(&c.name.as_str()));
            }
        }

        // Shuffle all colors before returning
        round_colors.shuffle(&mut rand::thread_rng());
        round_colors
    }

    /// Ends a question round and transitions back to Score phase.
    fn end_round(&mut self) {
        if let Some(song) = &self.state.current_song {
            self.state.used_songs.insert(song.uri.clone());
        }
        self.state.current_song = None;
        self.state.phase = GamePhase::Score;
    }

    /// Helper method to transition to GameOver state
    fn transition_to_game_over(&mut self, reason: String, outputs: &mut Vec<OutputEvent>) {
        self.state.phase = GamePhase::GameOver;
        let final_scores: Vec<(String, i32)> = self
            .state
            .players
            .values()
            .map(|p| (p.name.clone(), p.score))
            .collect();

        for &pid in self.state.players.keys() {
            outputs.push(OutputEvent {
                recipient: pid,
                data: OutputEventData::GameOver {
                    final_scores: final_scores.clone(),
                    reason: reason.clone(),
                },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_data() -> (Vec<Song>, Vec<ColorDef>) {
        let colors = vec![
            ColorDef {
                name: "Red".to_string(),
                rgb: "#FF0000".to_string(),
            },
            ColorDef {
                name: "Blue".to_string(),
                rgb: "#0000FF".to_string(),
            },
        ];

        let songs = vec![
            Song {
                id: 1,
                song_name: "Test Song 1".to_string(),
                artist: "Test Artist".to_string(),
                uri: "test:song:1".to_string(),
                colors: vec!["Red".to_string()],
            },
            Song {
                id: 2,
                song_name: "Test Song 2".to_string(),
                artist: "Test Artist".to_string(),
                uri: "test:song:2".to_string(),
                colors: vec!["Blue".to_string()],
            },
        ];

        (songs, colors)
    }

    #[test]
    fn test_game_initialization() {
        let (songs, colors) = setup_test_data();
        let lobby_id = Uuid::new_v4();
        let engine = GameEngine::new(lobby_id, songs, colors, 30);
        assert_eq!(engine.state.phase, GamePhase::Lobby);
    }

    #[test]
    fn test_player_join() {
        let (songs, colors) = setup_test_data();
        let lobby_id = Uuid::new_v4();
        let mut engine = GameEngine::new(lobby_id, songs, colors, 30);

        let player_id = Uuid::new_v4();
        let outputs = engine.update(
            InputEvent::Join {
                sender_id: player_id,
                name: "TestPlayer".to_string(),
                is_admin: true,
            },
            Instant::now(),
        );

        assert!(engine.state.players.contains_key(&player_id));
        assert!(!outputs.is_empty());
    }

    #[test]
    fn test_game_start() {
        let (songs, colors) = setup_test_data();
        let lobby_id = Uuid::new_v4();
        let mut engine = GameEngine::new(lobby_id, songs, colors, 30);

        let admin_id = Uuid::new_v4();
        engine.update(
            InputEvent::Join {
                sender_id: admin_id,
                name: "Admin".to_string(),
                is_admin: true,
            },
            Instant::now(),
        );

        let outputs = engine.update(
            InputEvent::ToggleState {
                sender_id: admin_id,
                specified_colors: None,
                operation: Some(StateOperation::StartGame),
            },
            Instant::now(),
        );

        assert_eq!(engine.state.phase, GamePhase::Score);
        assert!(!outputs.is_empty());
    }
}
