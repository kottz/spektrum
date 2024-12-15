use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    routing::any,
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use axum_server::Server;
use base64::Engine;
use clap::Parser;
use csv::ReaderBuilder;
use futures_util::{stream::StreamExt, SinkExt};
use rand::seq::SliceRandom;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};
use tokio::{
    io::{self, AsyncBufReadExt, BufReader},
    sync::mpsc,
    task,
};
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    lobby: String,
    #[arg(long, default_value_t = 8765)]
    port: u16,
    #[arg(long)]
    songs_csv: String,
    #[arg(long, default_value_t = false)]
    no_spotify: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
struct ColorDef {
    name: String,
    rgb: String,
}

#[derive(Serialize)]
struct PlayerAnsweredMsg {
    action: String,
    playerName: String,
    correct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Song {
    id: u32,
    song_name: String,
    artist: String,
    uri: String,
    colors: Vec<String>,
}

#[derive(Clone)]
struct Player {
    name: String,
    score: i32,
    has_answered: bool,
    answer: Option<String>,
    tx: mpsc::UnboundedSender<String>,
}

impl Player {
    fn new(name: &str, tx: mpsc::UnboundedSender<String>) -> Self {
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
struct GameLobby {
    name: String,
    players: HashMap<String, Player>,
    all_colors: Vec<ColorDef>,
    round_colors: Vec<ColorDef>,
    correct_colors: Vec<String>,
    state: String,
    round_start_time: Option<Instant>,
    round_duration: u64,
    songs: Vec<Song>,
    used_songs: HashSet<String>,
    current_song: Option<Song>,
}

impl GameLobby {
    fn new(name: String, songs: Vec<Song>) -> Self {
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
            state: "score".into(),
            round_start_time: None,
            round_duration: 50,
            songs,
            used_songs: HashSet::new(),
            current_song: None,
        }
    }

    fn add_player(&mut self, name: &str, tx: mpsc::UnboundedSender<String>) {
        if !self.players.contains_key(name) {
            self.players.insert(name.to_string(), Player::new(name, tx));
        }
    }

    fn remove_player(&mut self, name: &str) {
        self.players.remove(name);
    }

    fn get_player_list(&self) -> Vec<serde_json::Value> {
        self.players
            .values()
            .map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "score": p.score,
                })
            })
            .collect()
    }

    fn select_round_colors(&mut self, specified_colors: Option<Vec<String>>) -> bool {
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
            self.correct_colors = chosen_correct_colors
                .iter()
                .map(|c| c.name.clone())
                .collect();

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

    fn start_new_round(&mut self, specified_colors: Option<Vec<String>>) -> bool {
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

    fn end_round(&mut self) {
        if let Some(song) = &self.current_song {
            self.used_songs.insert(song.uri.clone());
        }
        self.current_song = None;
        self.state = "score".into();
    }

    fn check_answer(&mut self, player_name: &str, color_name: &str) -> (bool, i32) {
        if self.state != "question" {
            return (false, 0);
        }
        let player = match self.players.get_mut(player_name) {
            Some(p) => p,
            None => return (false, 0),
        };

        if player.has_answered {
            let already_correct = player
                .answer
                .as_ref()
                .map(|ans| self.correct_colors.contains(ans))
                .unwrap_or(false);
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

    fn toggle_state(&mut self, specified_colors: Option<Vec<String>>) -> String {
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

    fn all_players_answered(&self) -> bool {
        self.players.values().all(|p| p.has_answered)
    }

    fn get_answer_count(&self) -> (usize, usize) {
        let answered = self.players.values().filter(|p| p.has_answered).count();
        let total = self.players.len();
        (answered, total)
    }
}

#[derive(Clone)]
struct SpotifyController {
    client: Client,
    client_id: String,
    client_secret: String,
    refresh_token: String,
    token: Option<String>,
    token_expiry: Option<SystemTime>,
}

impl SpotifyController {
    async fn new() -> Option<Self> {
        let client_id = std::env::var("SPOTIFY_CLIENT_ID").ok()?;
        let client_secret = std::env::var("SPOTIFY_CLIENT_SECRET").ok()?;
        let refresh_token = std::env::var("SPOTIFY_REFRESH_TOKEN").ok()?;

        let mut ctrl = Self {
            client: Client::new(),
            client_id,
            client_secret,
            refresh_token,
            token: None,
            token_expiry: None,
        };
        if !ctrl.get_access_token().await {
            return None;
        }
        Some(ctrl)
    }

    async fn get_access_token(&mut self) -> bool {
        if let Some(expiry) = self.token_expiry {
            if let Ok(remaining) = expiry.duration_since(SystemTime::now()) {
                if remaining > Duration::from_secs(0) {
                    return true;
                }
            }
        }

        // Updated base64 usage:
        use base64::engine::general_purpose::STANDARD;
        let auth = STANDARD.encode(format!("{}:{}", self.client_id, self.client_secret));

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", &self.refresh_token),
        ];
        let res = match self
            .client
            .post("https://accounts.spotify.com/api/token")
            .header("Authorization", format!("Basic {}", auth))
            .form(&params)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(_) => {
                println!("Failed to send token request.");
                return false;
            }
        };
        if !res.status().is_success() {
            println!(
                "Failed to get Spotify access token. Status: {}",
                res.status()
            );
            return false;
        }
        // parse JSON
        let data: serde_json::Value = match res.json().await {
            Ok(d) => d,
            Err(_) => {
                println!("Failed to parse Spotify token response JSON.");
                return false;
            }
        };
        let access_token = match data.get("access_token").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => {
                println!("No access_token field in Spotify token response.");
                return false;
            }
        };
        let expires_in = data
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600);
        self.token = Some(access_token);
        self.token_expiry = Some(SystemTime::now() + Duration::from_secs(expires_in));
        true
    }

    async fn _check_token(&mut self) -> bool {
        if let Some(expiry) = self.token_expiry {
            if let Ok(remaining) = expiry.duration_since(SystemTime::now()) {
                if remaining > Duration::from_secs(0) {
                    return true;
                }
            }
        }
        self.get_access_token().await
    }

    async fn get_active_device(&mut self) -> Option<serde_json::Value> {
        if !self._check_token().await {
            return None;
        }
        let token = self.token.clone()?;
        let res = self
            .client
            .get("https://api.spotify.com/v1/me/player/devices")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .ok()?;

        if !res.status().is_success() {
            println!("Error retrieving devices from Spotify: {}", res.status());
            return None;
        }
        let data: serde_json::Value = res.json().await.ok()?;
        let devices = data.get("devices")?.as_array()?;
        if devices.is_empty() {
            println!("No devices found. Please open Spotify on a device.");
            return None;
        }
        let active_device = devices.iter().find(|dev| {
            dev.get("is_active")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        });
        active_device.cloned()
    }

    async fn play_track(&mut self, track_uri: &str) -> bool {
        if !self._check_token().await {
            return false;
        }
        let active_device = match self.get_active_device().await {
            Some(d) => d,
            None => {
                println!("No active Spotify device found. Cannot play track.");
                return false;
            }
        };
        let device_id = match active_device.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => {
                println!("No device id in active device info.");
                return false;
            }
        };
        let token = match &self.token {
            Some(t) => t.clone(),
            None => return false,
        };
        let url = format!(
            "https://api.spotify.com/v1/me/player/play?device_id={}",
            device_id
        );

        let body = serde_json::json!({ "uris": [track_uri] });
        let resp = match self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return false,
        };
        if ![204, 202].contains(&resp.status().as_u16()) {
            println!(
                "Failed to play track on Spotify. Status code: {}",
                resp.status()
            );
            return false;
        }
        true
    }

    async fn pause(&mut self) -> bool {
        if !self._check_token().await {
            return false;
        }
        let active_device = match self.get_active_device().await {
            Some(d) => d,
            None => {
                println!("No active device to pause.");
                return false;
            }
        };
        let device_id = match active_device.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return false,
        };
        let token = match &self.token {
            Some(t) => t.clone(),
            None => return false,
        };

        let url = format!(
            "https://api.spotify.com/v1/me/player/pause?device_id={}",
            device_id
        );
        let resp = match self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return false,
        };
        [204, 202].contains(&resp.status().as_u16())
    }
}

#[derive(Clone)]
struct AppState {
    lobby: Arc<Mutex<GameLobby>>,
    spotify: Option<Arc<Mutex<SpotifyController>>>,
}

fn load_songs_from_csv(filepath: &str) -> Vec<Song> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(false)
        .from_path(filepath)
        .unwrap();
    let mut songs = Vec::new();
    for result in rdr.records() {
        if let Ok(record) = result {
            if record.len() < 5 {
                continue;
            }
            let id = record[0].parse().unwrap_or(0);
            let song_name = record[1].trim().to_string();
            let artist = record[2].trim().to_string();
            let uri = record[3].trim().to_string();
            let colors_str = record[4].trim().to_string();
            let color_list: Vec<String> = colors_str
                .split(';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            songs.push(Song {
                id,
                song_name,
                artist,
                uri,
                colors: color_list,
            });
        }
    }
    songs
}

#[derive(Deserialize)]
struct ClientMessage {
    action: String,
    name: Option<String>,
    color: Option<String>,
}

#[derive(Serialize)]
struct ColorResult {
    action: String,
    correct: bool,
    score: i32,
    totalScore: i32,
}

#[derive(Serialize)]
struct LeaderboardEntry {
    name: String,
    score: i32,
}

#[derive(Serialize)]
struct GameStateMsg {
    action: String,
    state: String,
    score: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    colors: Option<Vec<ColorDef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    leaderboard: Option<Vec<LeaderboardEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    roundTimeLeft: Option<u64>,
    hasAnswered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    answer: Option<String>,
    answeredCount: usize,
    totalPlayers: usize,
}

#[derive(Serialize)]
struct UpdateAnswerCount {
    action: String,
    answeredCount: usize,
    totalPlayers: usize,
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: AppState) {
    // Channel from server -> client
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let mut player_name: Option<String> = None;

    loop {
        tokio::select! {
            // Read from client
            Some(Ok(Message::Text(payload))) = socket.recv() => {
                if let Ok(msg) = serde_json::from_str::<ClientMessage>(&payload) {
                    match msg.action.as_str() {
                        "join" => {
                            if let Some(name) = msg.name {
                                {
                                    // Lock only briefly
                                    let mut lobby = state.lobby.lock().unwrap();
                                    lobby.add_player(&name, tx.clone());
                                    println!("Player {} joined the lobby", name);
                                    player_name = Some(name.clone());
                                } // guard drops here
                                send_game_state(&state, &name).await;
                                broadcast_answer_count(&state).await;
                            }
                        }
                        "select_color" => {
                            if let Some(name) = &player_name {
                                let color_name_opt = msg.color.clone();
                                if let Some(color_name) = color_name_opt {
                                    // Lock only briefly
                                    let (correct, new_score, total_score, all_answered) = {
                                        let mut lobby = state.lobby.lock().unwrap();
                                        if lobby.state == "question" {
                                            let (correct, new_score) =
                                                lobby.check_answer(name, &color_name);
                                            let total_score = lobby.players[name].score;
                                            let answered_all = lobby.all_players_answered();
                                            (correct, new_score, total_score, answered_all)
                                        } else {
                                            (false, 0, lobby.players[name].score, false)
                                        }
                                    }; // guard drops here

                                    let response = ColorResult {
                                        action: "color_result".to_string(),
                                        correct,
                                        score: new_score,
                                        totalScore: total_score,
                                    };
                                    let json_msg = serde_json::to_string(&response).unwrap();
                                    tx.send(json_msg).ok();

                                    broadcast_player_answered(&state, name, correct).await;
                                    broadcast_answer_count(&state).await;

                                    if all_answered {
                                        println!("All players answered. Safe to toggle next phase.");
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            },
            // read server->client messages
            Some(server_msg) = rx.recv() => {
                let _ = socket.send(Message::Text(server_msg)).await;
            },
            else => {
                // client disconnected
                if let Some(name) = player_name.clone() {
                    let mut lobby = state.lobby.lock().unwrap();
                    lobby.remove_player(&name);
                    println!("Player {} left the lobby", name);
                }
                broadcast_game_state(&state).await;
                broadcast_answer_count(&state).await;
                break;
            }
        }
    }
}

fn calc_time_left(lobby: &GameLobby) -> u64 {
    if let Some(start) = lobby.round_start_time {
        let elapsed_ms = start.elapsed().as_millis() as u64;
        let total_ms = lobby.round_duration * 1000;
        if elapsed_ms >= total_ms {
            0
        } else {
            total_ms - elapsed_ms
        }
    } else {
        0
    }
}

async fn send_game_state(state: &AppState, player_name: &str) {
    let (msg, tx) = {
        let lobby = state.lobby.lock().unwrap();
        let player = match lobby.players.get(player_name) {
            Some(p) => p,
            None => return,
        };

        let (answered_count, total) = lobby.get_answer_count();
        // Build a leaderboard if we are in "score" state
        let lb = if lobby.state == "score" {
            Some(
                lobby
                    .players
                    .values()
                    .map(|p| LeaderboardEntry {
                        name: p.name.clone(),
                        score: p.score,
                    })
                    .collect(),
            )
        } else {
            None
        };

        let round_time_left = if lobby.state == "question" {
            Some(calc_time_left(&lobby))
        } else {
            None
        };

        let gm = GameStateMsg {
            action: "game_state".to_string(),
            state: lobby.state.clone(),
            score: player.score,
            colors: if lobby.state == "question" {
                Some(lobby.round_colors.clone())
            } else {
                None
            },
            leaderboard: lb,
            roundTimeLeft: round_time_left,
            hasAnswered: player.has_answered,
            answer: player.answer.clone(),
            answeredCount: answered_count,
            totalPlayers: total,
        };

        let json_msg = serde_json::to_string(&gm).unwrap();
        (json_msg, player.tx.clone())
    };

    let _ = tx.send(msg);
}

async fn broadcast_game_state(state: &AppState) {
    let names: Vec<String> = {
        let lobby = state.lobby.lock().unwrap();
        lobby.players.keys().cloned().collect()
    };
    for name in names {
        send_game_state(state, &name).await;
    }
}

async fn broadcast_answer_count(state: &AppState) {
    let (answered, total) = {
        let lobby = state.lobby.lock().unwrap();
        lobby.get_answer_count()
    };
    let msg = UpdateAnswerCount {
        action: "update_answer_count".into(),
        answeredCount: answered,
        totalPlayers: total,
    };
    let json_msg = serde_json::to_string(&msg).unwrap();

    let lobby = state.lobby.lock().unwrap();
    for p in lobby.players.values() {
        let _ = p.tx.send(json_msg.clone());
    }
}

async fn broadcast_player_answered(state: &AppState, player_name: &str, is_correct: bool) {
    let msg = PlayerAnsweredMsg {
        action: "player_answered".to_string(),
        playerName: player_name.to_string(),
        correct: is_correct,
    };
    let json_msg = serde_json::to_string(&msg).unwrap();

    let lobby = state.lobby.lock().unwrap();
    for p in lobby.players.values() {
        let _ = p.tx.send(json_msg.clone());
    }
}

async fn admin_input_loop(state: AppState) {
    let mut lines = BufReader::new(io::stdin()).lines();
    while let Ok(line_opt) = lines.next_line().await {
        if let Some(line) = line_opt {
            let trimmed = line.trim().to_string();
            println!("Admin input: {}", trimmed);
            if trimmed.to_lowercase().starts_with("toggle") {
                let mut specified_colors: Option<Vec<String>> = None;
                if let Some((_cmd, rest)) = trimmed.split_once(' ') {
                    let color_vec: Vec<String> =
                        rest.split(',').map(|s| s.trim().to_string()).collect();
                    if !color_vec.is_empty() {
                        specified_colors = Some(color_vec);
                    }
                }
                // Acquire the lock, do everything immediately, then drop it
                let (new_state, current_song_uri) = {
                    let mut lobby = state.lobby.lock().unwrap();
                    let new_state = lobby.toggle_state(specified_colors.clone());
                    println!("Game state changed to: {}", new_state);
                    if new_state == "question" {
                        println!("Correct color(s): {}", lobby.correct_colors.join(", "));
                        println!(
                            "All colors this round: {}",
                            lobby
                                .round_colors
                                .iter()
                                .map(|c| c.name.clone())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                        if let Some(song) = &lobby.current_song {
                            println!(
                                "Selected track: {} by {} - {}",
                                song.song_name, song.artist, song.uri
                            );
                        }
                    }
                    // If we need the song URI for Spotify, capture it outside the lock
                    let song_uri = lobby.current_song.as_ref().map(|s| s.uri.clone());

                    (new_state, song_uri)
                };
                if new_state == "question" {
                    if let Some(spotify) = &state.spotify {
                        if let Some(uri) = current_song_uri {
                            let mut ctrl = spotify.lock().unwrap().clone();
                            let success = ctrl.play_track(&uri).await;
                            if !success {
                                println!("Could not start playback. Check Spotify setup.");
                            }
                        }
                    }
                } else if new_state == "score" {
                    if let Some(spotify) = &state.spotify {
                        let mut ctrl = spotify.lock().unwrap().clone();
                        let _ = ctrl.pause().await;
                    }
                }
                broadcast_game_state(&state).await;
                broadcast_answer_count(&state).await;
            }
        } else {
            break; // EOF
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "spektrum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    let songs = load_songs_from_csv(&args.songs_csv);
    let lobby = GameLobby::new(args.lobby.clone(), songs);
    println!("Created lobby: {}", args.lobby);

    let spotify_controller = if !args.no_spotify {
        match SpotifyController::new().await {
            Some(ctrl) => {
                let mut c2 = ctrl.clone();
                match c2.get_active_device().await {
                    None => {
                        println!("No active Spotify device found at startup.");
                        std::process::exit(1);
                    }
                    Some(dev) => {
                        let device_name = dev
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown Device");
                        println!("Found active Spotify device: {}", device_name);
                    }
                }
                Some(Arc::new(Mutex::new(ctrl)))
            }
            None => {
                println!("Spotify integration failed to initialize. Exiting.");
                std::process::exit(1);
            }
        }
    } else {
        println!("Spotify integration disabled. Running without playback.");
        None
    };

    let state = AppState {
        lobby: Arc::new(Mutex::new(lobby)),
        spotify: spotify_controller,
    };

    // HTTPS version
    // let cert_path = PathBuf::from("./cert.pem");
    // let key_path = PathBuf::from("./key.pem");
    // let tls_config = match RustlsConfig::from_pem_file(cert_path, key_path).await {
    //     Ok(c) => c,
    //     Err(e) => {
    //         println!("Failed to load TLS config: {:?}", e);
    //         std::process::exit(1);
    //     }
    // };

    // let app = Router::new()
    //     .route("/ws", any(ws_handler))
    //     .fallback_service(ServeDir::new("./assets").append_index_html_on_directories(true))
    //     .with_state(state.clone());
    //
    // let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    // println!("WebSocket server started on {}", addr);
    //
    // let mut server = axum_server::bind_rustls(addr, tls_config);
    // // Enable HTTP/2
    // server.http_builder().http2().enable_connect_protocol();

    // Build the router
    let app = Router::new()
        .route("/ws", any(ws_handler))
        .fallback_service(ServeDir::new("./assets").append_index_html_on_directories(true))
        .with_state(state.clone());

    tokio::spawn(async move {
        admin_input_loop(state).await;
    });
    // Instead of binding with TLS config, just bind a normal SocketAddr
    let addr = SocketAddr::from(([0, 0, 0, 0], 8765));
    println!("Starting server on http://{}", addr);
    Server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    // // Plain HTTP server
    // let server = axum::Server::bind(&addr).serve(app.into_make_service());
    //
    // server.await.unwrap();
    // Admin input

    //server.serve(app.into_make_service()).await.unwrap();
}
