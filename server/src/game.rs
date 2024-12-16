use rand::seq::SliceRandom;
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::models::{ColorDef, Song};

#[derive(Clone)]
pub struct Player {
    pub name: String,
    pub score: i32,
    pub has_answered: bool,
    pub answer: Option<String>,
    pub tx: mpsc::UnboundedSender<String>,
}

impl Player {
    pub fn new(name: &str, tx: mpsc::UnboundedSender<String>) -> Self {
        Self {
            name: name.to_string(),
            score: 0,
            has_answered: false,
            answer: None,
            tx,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum GameState {
    Score,
    Question,
}

#[derive(Debug, thiserror::Error)]
pub enum GameError {
    #[error("No valid colors specified")]
    NoValidColors,
    #[error("No available songs")]
    NoAvailableSongs,
    #[error("Player not found: {0}")]
    PlayerNotFound(String),
    #[error("Invalid game state")]
    InvalidGameState,
    #[error("Round already in progress")]
    RoundInProgress,
}

pub type GameResult<T> = Result<T, GameError>;

#[derive(Clone)]
pub struct GameLobby {
    pub name: String,
    pub players: HashMap<String, Player>,
    pub all_colors: Vec<ColorDef>,
    pub round_colors: Vec<ColorDef>,
    pub correct_colors: Vec<String>,
    pub state: GameState,
    pub round_start_time: Option<Instant>,
    pub round_duration: u64,
    pub songs: Vec<Song>,
    pub used_songs: HashSet<String>,
    pub current_song: Option<Song>,
}

impl GameLobby {
    pub fn new(name: String, songs: Vec<Song>) -> Self {
        let all_colors = vec![
            ColorDef {
                name: "Red".into(),
                rgb: "#FF0000".into(),
            },
            ColorDef {
                name: "Green".into(),
                rgb: "#00FF00".into(),
            },
            ColorDef {
                name: "Blue".into(),
                rgb: "#0000FF".into(),
            },
            ColorDef {
                name: "Yellow".into(),
                rgb: "#FFFF00".into(),
            },
            ColorDef {
                name: "Purple".into(),
                rgb: "#800080".into(),
            },
            ColorDef {
                name: "Gold".into(),
                rgb: "#FFD700".into(),
            },
            ColorDef {
                name: "Silver".into(),
                rgb: "#C0C0C0".into(),
            },
            ColorDef {
                name: "Pink".into(),
                rgb: "#FFC0CB".into(),
            },
            ColorDef {
                name: "Black".into(),
                rgb: "#000000".into(),
            },
            ColorDef {
                name: "White".into(),
                rgb: "#FFFFFF".into(),
            },
            ColorDef {
                name: "Brown".into(),
                rgb: "#3D251E".into(),
            },
            ColorDef {
                name: "Orange".into(),
                rgb: "#FFA500".into(),
            },
            ColorDef {
                name: "Gray".into(),
                rgb: "#808080".into(),
            },
        ];

        Self {
            name,
            players: HashMap::new(),
            all_colors,
            round_colors: Vec::new(),
            correct_colors: Vec::new(),
            state: GameState::Score,
            round_start_time: None,
            round_duration: 50,
            songs,
            used_songs: HashSet::new(),
            current_song: None,
        }
    }

    pub fn add_player(&mut self, name: &str, tx: mpsc::UnboundedSender<String>) {
        if !self.players.contains_key(name) {
            self.players.insert(name.to_string(), Player::new(name, tx));
        }
    }

    pub fn remove_player(&mut self, name: &str) {
        self.players.remove(name);
    }

    pub fn get_answer_count(&self) -> (usize, usize) {
        let answered = self.players.values().filter(|p| p.has_answered).count();
        let total = self.players.len();
        (answered, total)
    }

    pub fn toggle_state(&mut self, specified_colors: Option<Vec<String>>) -> GameResult<GameState> {
        match self.state {
            GameState::Score => {
                self.start_new_round(specified_colors)?;
                Ok(self.state.clone())
            }
            GameState::Question => {
                self.end_round();
                Ok(self.state.clone())
            }
        }
    }

    pub fn start_new_round(&mut self, specified_colors: Option<Vec<String>>) -> GameResult<()> {
        self.select_round_colors(specified_colors)?;
        self.round_start_time = Some(Instant::now());
        self.state = GameState::Question;
        for p in self.players.values_mut() {
            p.has_answered = false;
            p.answer = None;
        }
        Ok(())
    }

    pub fn end_round(&mut self) {
        if let Some(song) = &self.current_song {
            self.used_songs.insert(song.uri.clone());
        }
        self.current_song = None;
        self.state = GameState::Score;
    }

    pub fn all_players_answered(&self) -> bool {
        self.players.values().all(|p| p.has_answered)
    }

    pub fn check_answer(&mut self, player_name: &str, color_name: &str) -> GameResult<(bool, i32)> {
        if let GameState::Question = self.state {
            let player = self
                .players
                .get_mut(player_name)
                .ok_or_else(|| GameError::PlayerNotFound(player_name.to_string()))?;

            if player.has_answered {
                return Ok((
                    player
                        .answer
                        .as_ref()
                        .map(|ans| self.correct_colors.contains(ans))
                        .unwrap_or(false),
                    player.score,
                ));
            }

            let elapsed = self
                .round_start_time
                .map(|start| start.elapsed().as_secs_f64())
                .unwrap_or(0.0);

            if elapsed > self.round_duration as f64 {
                return Ok((false, 0));
            }

            let calc_score = 5000_f64 - (elapsed * 100.0);
            let round_score = calc_score.max(0.0) as i32;
            let is_correct = self.correct_colors.contains(&color_name.to_string());

            if is_correct {
                player.score += round_score;
            }
            player.has_answered = true;
            player.answer = Some(color_name.to_string());

            Ok((is_correct, round_score))
        } else {
            Err(GameError::InvalidGameState)
        }
    }

    pub fn select_round_colors(&mut self, specified_colors: Option<Vec<String>>) -> GameResult<()> {
        self.round_colors.clear();
        self.correct_colors.clear();

        match specified_colors {
            Some(colors) => self.select_specified_colors(colors),
            None => self.select_random_colors(),
        }
    }

    fn select_specified_colors(&mut self, colors: Vec<String>) -> GameResult<()> {
        let chosen_correct_colors: Vec<_> = colors
            .iter()
            .filter_map(|c| {
                self.all_colors
                    .iter()
                    .find(|col| col.name.eq_ignore_ascii_case(c))
                    .cloned()
            })
            .collect();

        if chosen_correct_colors.is_empty() {
            return Err(GameError::NoValidColors);
        }

        self.setup_round_colors(chosen_correct_colors)
    }

    fn select_random_colors(&mut self) -> GameResult<()> {
        let available_songs: Vec<_> = self
            .songs
            .iter()
            .filter(|s| !self.used_songs.contains(&s.uri))
            .cloned()
            .collect();

        if available_songs.is_empty() {
            return Err(GameError::NoAvailableSongs);
        }

        let chosen_song = available_songs
            .choose(&mut rand::thread_rng())
            .ok_or(GameError::NoAvailableSongs)?;

        let chosen_correct_colors: Vec<_> = chosen_song
            .colors
            .iter()
            .filter_map(|c| {
                self.all_colors
                    .iter()
                    .find(|col| col.name.eq_ignore_ascii_case(c))
                    .cloned()
            })
            .collect();

        if chosen_correct_colors.is_empty() {
            return Err(GameError::NoValidColors);
        }

        self.current_song = Some(chosen_song.clone());
        self.setup_round_colors(chosen_correct_colors)
    }

    fn setup_round_colors(&mut self, chosen_correct_colors: Vec<ColorDef>) -> GameResult<()> {
        self.round_colors.extend(chosen_correct_colors.clone());
        self.correct_colors = chosen_correct_colors
            .iter()
            .map(|c| c.name.clone())
            .collect();

        let mut excluded = HashSet::new();
        if self
            .correct_colors
            .iter()
            .any(|cc| ["Yellow", "Gold", "Orange"].contains(&cc.as_str()))
        {
            excluded.extend(["Yellow", "Gold", "Orange"]);
        }
        if self
            .correct_colors
            .iter()
            .any(|cc| ["Silver", "Gray"].contains(&cc.as_str()))
        {
            excluded.extend(["Silver", "Gray"]);
        }

        let mut available: Vec<_> = self
            .all_colors
            .iter()
            .filter(|col| !excluded.contains(col.name.as_str()))
            .filter(|col| !self.round_colors.contains(col))
            .cloned()
            .collect();

        while self.round_colors.len() < 6 && !available.is_empty() {
            let chosen_color = available.remove(rand::random::<usize>() % available.len());
            self.round_colors.push(chosen_color.clone());

            if ["Yellow", "Gold", "Orange"].contains(&chosen_color.name.as_str()) {
                available.retain(|c| !["Yellow", "Gold", "Orange"].contains(&c.name.as_str()));
            } else if ["Silver", "Gray"].contains(&chosen_color.name.as_str()) {
                available.retain(|c| !["Silver", "Gray"].contains(&c.name.as_str()));
            }
        }

        self.round_colors.shuffle(&mut rand::thread_rng());
        Ok(())
    }
}
