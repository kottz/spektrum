use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};
use rand::seq::SliceRandom;
use tokio::sync::mpsc;

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

#[derive(Clone)]
pub struct GameLobby {
    pub name: String,
    pub players: HashMap<String, Player>,
    pub all_colors: Vec<ColorDef>,
    pub round_colors: Vec<ColorDef>,
    pub correct_colors: Vec<String>,
    pub state: String,
    pub round_start_time: Option<Instant>,
    pub round_duration: u64,
    pub songs: Vec<Song>,
    pub used_songs: HashSet<String>,
    pub current_song: Option<Song>,
}

impl GameLobby {
    pub fn new(name: String, songs: Vec<Song>) -> Self {
        let all_colors = vec![
            ColorDef { name: "Red".into(),    rgb: "#FF0000".into() },
            ColorDef { name: "Green".into(),  rgb: "#00FF00".into() },
            ColorDef { name: "Blue".into(),   rgb: "#0000FF".into() },
            ColorDef { name: "Yellow".into(), rgb: "#FFFF00".into() },
            ColorDef { name: "Purple".into(), rgb: "#800080".into() },
            ColorDef { name: "Gold".into(),   rgb: "#FFD700".into() },
            ColorDef { name: "Silver".into(), rgb: "#C0C0C0".into() },
            ColorDef { name: "Pink".into(),   rgb: "#FFC0CB".into() },
            ColorDef { name: "Black".into(),  rgb: "#000000".into() },
            ColorDef { name: "White".into(),  rgb: "#FFFFFF".into() },
            ColorDef { name: "Brown".into(),  rgb: "#3D251E".into() },
            ColorDef { name: "Orange".into(), rgb: "#FFA500".into() },
            ColorDef { name: "Gray".into(),   rgb: "#808080".into() },
        ];

        Self {
            name,
            players: HashMap::new(),
            all_colors,
            round_colors: Vec::new(),
            correct_colors: Vec::new(),
            state: "score".into(),
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

    pub fn toggle_state(&mut self, specified_colors: Option<Vec<String>>) -> String {
        if self.state == "score" {
            let success = self.start_new_round(specified_colors);
            if !success {
                return self.state.clone();
            }
        } else {
            self.end_round();
        }
        self.state.clone()
    }

    pub fn start_new_round(&mut self, specified_colors: Option<Vec<String>>) -> bool {
        let success = self.select_round_colors(specified_colors);
        if !success {
            return false;
        }
        self.round_start_time = Some(Instant::now());
        self.state = "question".into();
        for p in self.players.values_mut() {
            p.has_answered = false;
            p.answer = None;
        }
        true
    }

    pub fn end_round(&mut self) {
        if let Some(song) = &self.current_song {
            self.used_songs.insert(song.uri.clone());
        }
        self.current_song = None;
        self.state = "score".into();
    }

    pub fn all_players_answered(&self) -> bool {
        self.players.values().all(|p| p.has_answered)
    }

    pub fn check_answer(&mut self, player_name: &str, color_name: &str) -> (bool, i32) {
        if self.state != "question" {
            return (false, 0);
        }
        let player = match self.players.get_mut(player_name) {
            Some(p) => p,
            None => return (false, 0),
        };

        if player.has_answered {
            let already_correct = player.answer.as_ref().map(|ans| self.correct_colors.contains(ans)).unwrap_or(false);
            return (already_correct, player.score);
        }

        let elapsed = if let Some(start) = self.round_start_time {
            start.elapsed().as_secs_f64()
        } else {
            0.0
        };
        if elapsed > self.round_duration as f64 {
            return (false, 0);
        }

        let calc_score = 5000_f64 - (elapsed * 100.0);
        let round_score = calc_score.max(0.0) as i32;
        let is_correct = self.correct_colors.contains(&color_name.to_string());
        if is_correct {
            player.score += round_score;
        }
        player.has_answered = true;
        player.answer = Some(color_name.to_string());

        (is_correct, round_score)
    }

    pub fn select_round_colors(&mut self, specified_colors: Option<Vec<String>>) -> bool {
        self.round_colors.clear();
        self.correct_colors.clear();

        if let Some(color_list) = specified_colors {
            let mut chosen_correct_colors = Vec::new();
            for c in color_list {
                if let Some(cd) = self
                    .all_colors
                    .iter()
                    .find(|col| col.name.eq_ignore_ascii_case(&c))
                {
                    chosen_correct_colors.push(cd.clone());
                }
            }
            if chosen_correct_colors.is_empty() {
                println!("No valid specified colors found.");
                return false;
            }
            self.round_colors.extend(chosen_correct_colors.clone());
            self.correct_colors = chosen_correct_colors.iter().map(|c| c.name.clone()).collect();

            let mut excluded = std::collections::HashSet::new();
            if self.correct_colors.iter().any(|cc| ["Yellow", "Gold", "Orange"].contains(&cc.as_str())) {
                excluded.extend(["Yellow", "Gold", "Orange"]);
            }
            if self.correct_colors.iter().any(|cc| ["Silver", "Gray"].contains(&cc.as_str())) {
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
                let idx = rand::random::<usize>() % available.len();
                let chosen_color = available.swap_remove(idx);
                self.round_colors.push(chosen_color.clone());

                if ["Yellow", "Gold", "Orange"].contains(&chosen_color.name.as_str()) {
                    available.retain(|c| !["Yellow", "Gold", "Orange"].contains(&c.name.as_str()));
                } else if ["Silver", "Gray"].contains(&chosen_color.name.as_str()) {
                    available.retain(|c| !["Silver", "Gray"].contains(&c.name.as_str()));
                }
            }
            self.round_colors.shuffle(&mut rand::thread_rng());
            true
        } else {
            // Random from CSV
            let available_songs: Vec<_> = self
                .songs
                .iter()
                .filter(|s| !self.used_songs.contains(&s.uri))
                .cloned()
                .collect();
            if available_songs.is_empty() {
                println!("No more songs available. The game ends here.");
                return false;
            }
            let idx = rand::random::<usize>() % available_songs.len();
            let chosen_song = available_songs[idx].clone();

            let mut chosen_correct_colors = Vec::new();
            for c_name in chosen_song.colors.iter() {
                if let Some(cd) = self
                    .all_colors
                    .iter()
                    .find(|col| col.name.eq_ignore_ascii_case(c_name))
                {
                    chosen_correct_colors.push(cd.clone());
                } else {
                    println!("Color {} not found in all_colors list.", c_name);
                }
            }
            if chosen_correct_colors.is_empty() {
                return false;
            }

            self.current_song = Some(chosen_song.clone());
            self.round_colors.extend(chosen_correct_colors.clone());
            self.correct_colors = chosen_correct_colors.iter().map(|c| c.name.clone()).collect();

            let mut excluded = std::collections::HashSet::new();
            if chosen_correct_colors.iter().any(|cc| ["Yellow", "Gold", "Orange"].contains(&cc.name.as_str())) {
                excluded.extend(["Yellow", "Gold", "Orange"]);
            }
            if chosen_correct_colors.iter().any(|cc| ["Silver", "Gray"].contains(&cc.name.as_str())) {
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
                let idx2 = rand::random::<usize>() % available.len();
                let chosen_color_obj = available.swap_remove(idx2);
                self.round_colors.push(chosen_color_obj.clone());

                if ["Yellow", "Gold", "Orange"].contains(&chosen_color_obj.name.as_str()) {
                    available.retain(|c| !["Yellow", "Gold", "Orange"].contains(&c.name.as_str()));
                } else if ["Silver", "Gray"].contains(&chosen_color_obj.name.as_str()) {
                    available.retain(|c| !["Silver", "Gray"].contains(&c.name.as_str()));
                }
            }
            self.round_colors.shuffle(&mut rand::thread_rng());
            true
        }
    }
}
