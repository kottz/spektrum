use rand::distributions::{Distribution, WeightedIndex};
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};
use uuid::Uuid;

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

#[derive(Debug, Clone)]
struct ColorStats {
    name: String,
    frequency: f64,
    song_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Song {
    pub id: u32,
    pub song_name: String,
    pub artist: String,
    pub colors: Vec<String>,
    pub spotify_uri: String,
    pub youtube_id: String,
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
    SkipSong,
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
    AdminInfo {
        current_song: Song,
    },
    AdminNextSongs {
        upcoming_songs: Vec<Song>,
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
    pub current_song_index: usize,
}

pub struct GameEngine {
    state: GameState,
}

impl GameEngine {
    pub fn new(
        admin_id: Uuid,
        mut all_songs: Vec<Song>,
        all_colors: Vec<ColorDef>,
        round_duration: u64,
    ) -> Self {
        let mut rng = rand::thread_rng();
        all_songs.shuffle(&mut rng);
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
                current_song_index: 0,
            },
        }
    }

    pub fn process_event(&mut self, event: GameEvent) -> Vec<GameResponse> {
        let GameEvent { context, action } = event;

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
            GameAction::SkipSong => self.handle_skip_song(context),
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
                    message: "You can't join a game that has already started.".into(),
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
            let score_delta = ((self.state.round_duration as f64 * 100.0
                - (elapsed.as_secs_f64() * 100.0))
                .max(0.0)) as i32;
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

        let mut responses = vec![GameResponse {
            recipients: Recipients::All,
            payload: ResponsePayload::StateChanged {
                phase: GamePhase::Score,
                colors: Vec::new(),
                scoreboard: self.get_scoreboard(),
            },
        }];

        // Send next songs to admin after transitioning to Score phase
        let upcoming = self.get_upcoming_songs(3);
        if !upcoming.is_empty() {
            responses.push(GameResponse {
                recipients: Recipients::Single(self.state.admin_id),
                payload: ResponsePayload::AdminNextSongs {
                    upcoming_songs: upcoming,
                },
            });
        }

        responses
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

        for player in self.state.players.values_mut() {
            player.has_answered = false;
            player.answer = None;
        }

        match self.setup_round_colors(specified_colors) {
            Ok(()) => {
                self.state.phase = GamePhase::Question;
                self.state.round_start_time = Some(ctx.timestamp);
                let mut outputs = Vec::new();
                outputs.push(GameResponse {
                    recipients: Recipients::All,
                    payload: ResponsePayload::StateChanged {
                        phase: GamePhase::Question,
                        colors: self.state.colors.clone(),
                        scoreboard: self.get_scoreboard(),
                    },
                });

                if let Some(song) = &self.state.current_song {
                    outputs.push(GameResponse {
                        recipients: Recipients::Single(self.state.admin_id),
                        payload: ResponsePayload::AdminInfo {
                            current_song: song.clone(),
                        },
                    });
                }

                outputs
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
            self.state.used_songs.insert(song.spotify_uri.clone());
        }

        // Move to next song for the next round
        self.state.current_song = None;
        self.state.current_song_index += 1; // Increment index to use next song next time
        self.state.colors.clear();
        self.state.correct_colors.clear();
        self.state.phase = GamePhase::Score;

        let mut responses = vec![GameResponse {
            recipients: Recipients::All,
            payload: ResponsePayload::StateChanged {
                phase: GamePhase::Score,
                colors: Vec::new(),
                scoreboard: self.get_scoreboard(),
            },
        }];

        // Send next 3 upcoming songs to admin
        let upcoming = self.get_upcoming_songs(3);
        if !upcoming.is_empty() {
            responses.push(GameResponse {
                recipients: Recipients::Single(self.state.admin_id),
                payload: ResponsePayload::AdminNextSongs {
                    upcoming_songs: upcoming,
                },
            });
        }

        responses
    }

    fn handle_skip_song(&mut self, ctx: EventContext) -> Vec<GameResponse> {
        if self.state.phase != GamePhase::Score {
            return vec![GameResponse {
                recipients: Recipients::Single(ctx.sender_id),
                payload: ResponsePayload::Error {
                    code: ErrorCode::InvalidPhase,
                    message: "Can only skip song during scoreboard phase".into(),
                },
            }];
        }

        if let Some(song) = &self.state.current_song {
            self.state.used_songs.insert(song.spotify_uri.clone());
        }

        self.state.current_song = None;
        self.state.current_song_index += 1; // Increment index to use next song next time
        self.state.colors.clear();
        self.state.correct_colors.clear();

        let mut responses = vec![GameResponse {
            recipients: Recipients::All,
            payload: ResponsePayload::StateChanged {
                phase: GamePhase::Score,
                colors: Vec::new(),
                scoreboard: self.get_scoreboard(),
            },
        }];

        // Send next 3 upcoming songs to admin
        let upcoming = self.get_upcoming_songs(3);
        if !upcoming.is_empty() {
            responses.push(GameResponse {
                recipients: Recipients::Single(self.state.admin_id),
                payload: ResponsePayload::AdminNextSongs {
                    upcoming_songs: upcoming,
                },
            });
        }

        responses
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
            // Handle manually specified colors
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
            // Handle song-based colors
            if self.state.current_song_index >= self.state.all_songs.len() {
                return Err("No available songs".to_string());
            }

            let chosen_song = self.state.all_songs[self.state.current_song_index].clone();
            if self.state.used_songs.contains(&chosen_song.spotify_uri) {
                return Err("No available songs".to_string());
            }

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

    fn calculate_color_weights(&self) -> HashMap<String, f64> {
        let mut color_counts: HashMap<String, usize> = HashMap::new();
        let total_songs = self.state.all_songs.len() as f64;

        // Count how many songs each color appears in
        for song in &self.state.all_songs {
            for color in &song.colors {
                *color_counts.entry(color.clone()).or_insert(0) += 1;
            }
        }

        // Convert counts to adjusted weights using square root to boost rare colors
        let mut color_weights: HashMap<String, f64> = HashMap::new();
        for (color, count) in color_counts {
            // Calculate base proportion
            let base_proportion = count as f64 / total_songs;

            // Apply square root transformation and add a minimum boost
            // This compresses the range between common and rare colors
            let adjusted_weight = base_proportion.sqrt() + 0.15;

            color_weights.insert(color, adjusted_weight);
        }

        // Also ensure any colors in all_colors that haven't appeared in songs get a minimum weight
        for color in &self.state.all_colors {
            color_weights.entry(color.name.clone()).or_insert(0.15); // Minimum weight for colors that never appear
        }

        color_weights
    }

    fn generate_round_colors(&self, correct_colors: Vec<ColorDef>) -> Vec<ColorDef> {
        let mut round_colors = correct_colors.clone();
        let mut rng = rand::thread_rng();

        if round_colors.len() >= 6 {
            round_colors.shuffle(&mut rng);
            return round_colors;
        }

        // Calculate weights for all colors
        let color_weights = self.calculate_color_weights();

        // Get available colors (not in correct_colors)
        let mut available: Vec<ColorDef> = self
            .state
            .all_colors
            .iter()
            .filter(|col| !correct_colors.contains(col))
            .cloned()
            .collect();

        // Create initial weights for available colors
        let mut weights: Vec<f64> = available
            .iter()
            .map(|color| color_weights.get(&color.name).copied().unwrap_or(0.15))
            .collect();

        // Select remaining colors
        while round_colors.len() < 6 && !available.is_empty() {
            // Create distribution for current weights
            if let Ok(dist) = WeightedIndex::new(&weights) {
                let idx = dist.sample(&mut rng);
                round_colors.push(available.remove(idx));
                weights.remove(idx);
            } else {
                // Fallback to simple random if weights are invalid
                let idx = rng.gen_range(0..available.len());
                round_colors.push(available.remove(idx));
                weights.remove(idx);
            }
        }

        round_colors.shuffle(&mut rng);
        round_colors
    }

    // Helper function to view the actual weights being used
    fn get_color_statistics(&self) -> Vec<ColorStats> {
        let color_weights = self.calculate_color_weights();
        let mut stats: Vec<ColorStats> = color_weights
            .into_iter()
            .map(|(name, weight)| {
                let song_count = self
                    .state
                    .all_songs
                    .iter()
                    .filter(|song| song.colors.contains(&name))
                    .count();

                ColorStats {
                    name,
                    frequency: weight,
                    song_count,
                }
            })
            .collect();

        stats.sort_by(|a, b| b.frequency.partial_cmp(&a.frequency).unwrap());
        stats
    }

    fn get_upcoming_songs(&self, count: usize) -> Vec<Song> {
        let start = self.state.current_song_index;
        let end = std::cmp::min(start + count, self.state.all_songs.len());
        let mut upcoming = Vec::new();
        for i in start..end {
            if !self
                .state
                .used_songs
                .contains(&self.state.all_songs[i].spotify_uri)
            {
                upcoming.push(self.state.all_songs[i].clone());
            }
        }
        upcoming
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
                youtube_id: "Xyzzzxyzz34".to_string(),
                spotify_uri: "test:song:1".to_string(),
                colors: vec!["Red".to_string()],
            },
            Song {
                id: 2,
                song_name: "Test Song 2".to_string(),
                artist: "Test Artist".to_string(),
                youtube_id: "Xyzzzxyzz34".to_string(),
                spotify_uri: "test:song:1".to_string(),
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
