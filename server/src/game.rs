use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};
use uuid::Uuid;

// Core types
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum GamePhase {
    Lobby,
    Score,
    Question,
    GameOver,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ColorDef {
    pub name: String,
    pub rgb: String,
}

#[derive(Clone, Debug)]
pub struct Song {
    pub id: u32,
    pub song_name: String,
    pub artist: String,
    pub uri: String,
    pub colors: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct PlayerState {
    pub name: String,
    pub score: i32,
    pub has_answered: bool,
    pub answer: Option<String>,
}

impl PlayerState {
    pub fn new(name: String) -> Self {
        Self {
            name,
            score: 0,
            has_answered: false,
            answer: None,
        }
    }
}

// Event system
#[derive(Clone, Debug)]
pub struct EventContext {
    pub lobby_id: Uuid,
    pub sender_id: Uuid,
    pub timestamp: Instant,
    pub is_admin: bool,
}

#[derive(Clone, Debug)]
pub enum Recipients {
    Single(Uuid),
    Multiple(Vec<Uuid>),
    AllExcept(Vec<Uuid>),
    All,
}

#[derive(Clone, Debug)]
pub struct GameEvent {
    pub context: EventContext,
    pub action: GameAction,
}

#[derive(Clone, Debug)]
pub enum GameAction {
    Join {
        name: String,
    },
    Leave,
    Answer {
        color: String,
    },
    StartGame,
    StartRound {
        specified_colors: Option<Vec<String>>,
    },
    EndRound,
    EndGame {
        reason: String,
    },
    CloseGame {
        reason: String,
    },
}

#[derive(Clone, Debug)]
pub struct GameResponse {
    pub recipients: Recipients,
    pub payload: ResponsePayload,
}

#[derive(Clone, Debug)]
pub enum ResponsePayload {
    Joined {
        player_id: Uuid,
        name: String,
        round_duration: u64,
        current_players: Vec<(String, i32)>,
    },
    PlayerLeft {
        name: String,
    },
    PlayerAnswered {
        name: String,
        correct: bool,
        new_score: i32,
    },
    StateChanged {
        phase: GamePhase,
        colors: Vec<ColorDef>,
        scoreboard: Vec<(String, i32)>,
    },
    GameOver {
        final_scores: Vec<(String, i32)>,
        reason: String,
    },
    GameClosed {
        reason: String,
    },
    Error {
        code: ErrorCode,
        message: String,
    },
}

#[derive(Clone, Debug)]
pub enum ErrorCode {
    NotAuthorized,
    InvalidPhase,
    InvalidAction,
    GameClosed,
    PlayerNotFound,
    AlreadyAnswered,
    TimeExpired,
    LobbyNotFound,
}

// Game state and engine
pub struct GameState {
    pub phase: GamePhase,
    pub players: HashMap<Uuid, PlayerState>,
    pub admin_id: Uuid,
    pub round_start_time: Option<Instant>,
    pub round_duration: u64,
    pub colors: Vec<ColorDef>,
    pub correct_colors: Vec<String>,
    pub current_song: Option<Song>,
    pub used_songs: HashSet<String>,
    pub all_songs: Vec<Song>,
    pub all_colors: Vec<ColorDef>,
}

pub struct GameEngine {
    state: GameState,
}

impl GameEngine {
    pub fn new(
        admin_id: Uuid,
        all_songs: Vec<Song>,
        all_colors: Vec<ColorDef>,
        round_duration: u64,
    ) -> Self {
        Self {
            state: GameState {
                phase: GamePhase::Lobby,
                players: HashMap::new(),
                admin_id,
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

    pub fn process_event(&mut self, event: GameEvent) -> Vec<GameResponse> {
        let GameEvent { context, action } = event;

        // Admin check for admin-only actions
        match &action {
            GameAction::StartGame
            | GameAction::StartRound { .. }
            | GameAction::EndRound
            | GameAction::EndGame { .. }
            | GameAction::CloseGame { .. } => {
                if context.sender_id != self.state.admin_id {
                    return vec![GameResponse {
                        recipients: Recipients::Single(context.sender_id),
                        payload: ResponsePayload::Error {
                            code: ErrorCode::NotAuthorized,
                            message: "Admin action requires authorization".into(),
                        },
                    }];
                }
            }
            _ => {}
        }

        match action {
            GameAction::Join { name } => self.handle_join(context, name),
            GameAction::Leave => self.handle_leave(context),
            GameAction::Answer { color } => self.handle_answer(context, color),
            GameAction::StartGame => self.handle_start_game(context),
            GameAction::StartRound { specified_colors } => {
                self.handle_start_round(context, specified_colors)
            }
            GameAction::EndRound => self.handle_end_round(context),
            GameAction::EndGame { reason } => self.handle_end_game(context, reason),
            GameAction::CloseGame { reason } => self.handle_close_game(context, reason),
        }
    }

    fn handle_join(&mut self, ctx: EventContext, name: String) -> Vec<GameResponse> {
        if self.state.admin_id == ctx.sender_id {
            return vec![];
        }
        if self.state.phase != GamePhase::Lobby {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidPhase,
                    message: "Can only join during lobby phase".into(),
                },
            }];
        }

        let current_players: Vec<(String, i32)> = self
            .state
            .players
            .values()
            .map(|p| (p.name.clone(), p.score))
            .collect();

        self.state
            .players
            .insert(ctx.sender_id, PlayerState::new(name.clone()));

        vec![
            GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Joined {
                    player_id: ctx.sender_id,
                    name: name.clone(),
                    round_duration: self.state.round_duration,
                    current_players,
                },
            },
            GameResponse {
                recipients: Recipients::AllExcept(vec![ctx.sender_id]),
                payload: ResponsePayload::StateChanged {
                    phase: self.state.phase,
                    colors: self.state.colors.clone(),
                    scoreboard: self.get_scoreboard(),
                },
            },
        ]
    }

    fn handle_leave(&mut self, ctx: EventContext) -> Vec<GameResponse> {
        if let Some(player) = self.state.players.remove(&ctx.sender_id) {
            if ctx.sender_id == self.state.admin_id {
                vec![GameResponse {
                    recipients: Recipients::All,
                    payload: ResponsePayload::GameClosed {
                        reason: "Host left the game".into(),
                    },
                }]
            } else {
                vec![GameResponse {
                    recipients: Recipients::All,
                    payload: ResponsePayload::PlayerLeft { name: player.name },
                }]
            }
        } else {
            vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::PlayerNotFound,
                    message: "Player not found".into(),
                },
            }]
        }
    }

    fn handle_answer(&mut self, ctx: EventContext, color: String) -> Vec<GameResponse> {
        if self.state.phase != GamePhase::Question {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidPhase,
                    message: "Not in Question phase".into(),
                },
            }];
        }

        let player = match self.state.players.get_mut(&ctx.sender_id) {
            Some(p) => p,
            None => {
                return vec![GameResponse {
                    recipients: Recipients::Single(ctx.sender_id),
                    payload: ResponsePayload::Error {
                        code: ErrorCode::PlayerNotFound,
                        message: "Player not found".into(),
                    },
                }]
            }
        };

        if player.has_answered {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::AlreadyAnswered,
                    message: "Already answered this round".into(),
                },
            }];
        }

        let elapsed = ctx.timestamp.duration_since(
            self.state
                .round_start_time
                .expect("Round start time should be set"),
        );

        if elapsed.as_secs() > self.state.round_duration {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::TimeExpired,
                    message: "Time expired for this round".into(),
                },
            }];
        }

        let correct = self.state.correct_colors.contains(&color);
        let new_score = if correct {
            let score_delta = ((self.state.round_duration as f64 * 100.0 - (elapsed.as_secs_f64() * 100.0)).max(0.0)) as i32;
            player.score += score_delta;
            player.score
        } else {
            player.score
        };

        player.has_answered = true;
        player.answer = Some(color);

        vec![GameResponse {
            recipients: Recipients::All,
            payload: ResponsePayload::PlayerAnswered {
                name: player.name.clone(),
                correct,
                new_score,
            },
        }]
    }

    fn handle_start_game(&mut self, ctx: EventContext) -> Vec<GameResponse> {
        if self.state.phase != GamePhase::Lobby {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidPhase,
                    message: "Can only start game from lobby".into(),
                },
            }];
        }

        self.state.phase = GamePhase::Score;

        vec![GameResponse {
            recipients: Recipients::All,
            payload: ResponsePayload::StateChanged {
                phase: GamePhase::Score,
                colors: Vec::new(),
                scoreboard: self.get_scoreboard(),
            },
        }]
    }

    fn handle_start_round(
        &mut self,
        ctx: EventContext,
        specified_colors: Option<Vec<String>>,
    ) -> Vec<GameResponse> {
        if self.state.phase != GamePhase::Score {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidPhase,
                    message: "Can only start round from score phase".into(),
                },
            }];
        }

        // Reset player states
        for player in self.state.players.values_mut() {
            player.has_answered = false;
            player.answer = None;
        }

        match self.setup_round_colors(specified_colors) {
            Ok(()) => {
                self.state.phase = GamePhase::Question;
                self.state.round_start_time = Some(ctx.timestamp);

                vec![GameResponse {
                    recipients: Recipients::All,
                    payload: ResponsePayload::StateChanged {
                        phase: GamePhase::Question,
                        colors: self.state.colors.clone(),
                        scoreboard: self.get_scoreboard(),
                    },
                }]
            }
            Err(msg) => vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidAction,
                    message: msg,
                },
            }],
        }
    }

    fn handle_end_round(&mut self, ctx: EventContext) -> Vec<GameResponse> {
        if self.state.phase != GamePhase::Question {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidPhase,
                    message: "Can only end round from question phase".into(),
                },
            }];
        }

        if let Some(song) = &self.state.current_song {
            self.state.used_songs.insert(song.uri.clone());
        }

        self.state.phase = GamePhase::Score;
        self.state.current_song = None;
        self.state.colors.clear();
        self.state.correct_colors.clear();

        vec![GameResponse {
            recipients: Recipients::All,
            payload: ResponsePayload::StateChanged {
                phase: GamePhase::Score,
                colors: Vec::new(),
                scoreboard: self.get_scoreboard(),
            },
        }]
    }

    fn handle_end_game(&mut self, _ctx: EventContext, reason: String) -> Vec<GameResponse> {
        self.state.phase = GamePhase::GameOver;

        vec![GameResponse {
            recipients: Recipients::All,
            payload: ResponsePayload::GameOver {
                final_scores: self.get_scoreboard(),
                reason,
            },
        }]
    }

    fn handle_close_game(&mut self, _ctx: EventContext, reason: String) -> Vec<GameResponse> {
        vec![GameResponse {
            recipients: Recipients::All,
            payload: ResponsePayload::GameClosed { reason },
        }]
    }

    // Helper methods
    fn get_scoreboard(&self) -> Vec<(String, i32)> {
        self.state
            .players
            .values()
            .map(|p| (p.name.clone(), p.score))
            .collect()
    }

    fn setup_round_colors(&mut self, specified_colors: Option<Vec<String>>) -> Result<(), String> {
        self.state.colors.clear();
        self.state.correct_colors.clear();

        if let Some(specs) = specified_colors {
            // Handle admin-specified colors
            let chosen: Vec<ColorDef> = specs
                .iter()
                .filter_map(|name| {
                    self.state
                        .all_colors
                        .iter()
                        .find(|c| c.name.eq_ignore_ascii_case(name))
                        .cloned()
                })
                .collect();

            if chosen.is_empty() {
                return Err("No valid specified colors".to_string());
            }
            self.state.colors = chosen.clone();
            self.state.correct_colors = chosen.iter().map(|c| c.name.clone()).collect();
            self.state.current_song = None;
            Ok(())
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

            let chosen_song = available_songs
                .choose(&mut rand::thread_rng())
                .ok_or_else(|| "Failed to choose song".to_string())?
                .clone();

            // Gather correct colors from the chosen song
            let chosen_correct_colors: Vec<ColorDef> = chosen_song
                .colors
                .iter()
                .filter_map(|cname| {
                    self.state
                        .all_colors
                        .iter()
                        .find(|c| c.name.eq_ignore_ascii_case(cname))
                        .cloned()
                })
                .collect();

            if chosen_correct_colors.is_empty() {
                return Err("Song has no valid colors".to_string());
            }

            self.state.current_song = Some(chosen_song);
            self.state.correct_colors = chosen_correct_colors
                .iter()
                .map(|c| c.name.clone())
                .collect();

            self.state.colors = self.generate_round_colors(chosen_correct_colors);
            Ok(())
        }
    }

    fn generate_round_colors(&self, correct_colors: Vec<ColorDef>) -> Vec<ColorDef> {
        let mut round_colors = correct_colors.clone();
        let mut excluded = HashSet::new();

        // Handle similar color groups
        if correct_colors
            .iter()
            .any(|cc| ["Yellow", "Gold", "Orange"].contains(&cc.name.as_str()))
        {
            excluded.extend(["Yellow", "Gold", "Orange"]);
        }
        if correct_colors
            .iter()
            .any(|cc| ["Silver", "Gray"].contains(&cc.name.as_str()))
        {
            excluded.extend(["Silver", "Gray"]);
        }

        // Get available colors (not in correct colors or excluded groups)
        let mut available: Vec<_> = self
            .state
            .all_colors
            .iter()
            .filter(|col| !correct_colors.contains(col))
            .filter(|col| !excluded.contains(col.name.as_str()))
            .cloned()
            .collect();

        // Add random colors until we reach 6 or run out
        while round_colors.len() < 6 && !available.is_empty() {
            let idx = rand::random::<usize>() % available.len();
            let chosen = available.remove(idx);

            // Update excluded colors based on what we just added
            if ["Yellow", "Gold", "Orange"].contains(&chosen.name.as_str()) {
                available.retain(|c| !["Yellow", "Gold", "Orange"].contains(&c.name.as_str()));
            } else if ["Silver", "Gray"].contains(&chosen.name.as_str()) {
                available.retain(|c| !["Silver", "Gray"].contains(&c.name.as_str()));
            }

            round_colors.push(chosen);
        }

        // Shuffle before returning
        let mut rng = rand::thread_rng();
        round_colors.shuffle(&mut rng);
        round_colors
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
        let admin_id = Uuid::new_v4();
        let engine = GameEngine::new(admin_id, songs, colors, 30);
        assert_eq!(engine.state.phase, GamePhase::Lobby);
    }

    #[test]
    fn test_join_game() {
        let (songs, colors) = setup_test_data();
        let admin_id = Uuid::new_v4();
        let mut engine = GameEngine::new(admin_id, songs, colors, 30);
        let player_id = Uuid::new_v4();

        let ctx = EventContext {
            lobby_id: Uuid::new_v4(),
            sender_id: player_id,
            timestamp: Instant::now(),
            is_admin: false,
        };

        let event = GameEvent {
            context: ctx,
            action: GameAction::Join {
                name: "TestPlayer".to_string(),
            },
        };

        let responses = engine.process_event(event);
        assert!(!responses.is_empty());
        assert!(engine.state.players.contains_key(&player_id));
    }

    #[test]
    fn test_game_start() {
        let (songs, colors) = setup_test_data();
        let admin_id = Uuid::new_v4();
        let mut engine = GameEngine::new(admin_id, songs, colors, 30);

        let ctx = EventContext {
            lobby_id: Uuid::new_v4(),
            sender_id: admin_id,
            timestamp: Instant::now(),
            is_admin: true,
        };

        let event = GameEvent {
            context: ctx,
            action: GameAction::StartGame,
        };

        let responses = engine.process_event(event);
        assert!(!responses.is_empty());
        assert_eq!(engine.state.phase, GamePhase::Score);
    }
}
