use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn};
use uuid::Uuid;

use crate::models::{ColorDef, Song};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
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
    #[error("Channel send error")]
    ChannelError,
    #[error("Lobby not found: {0}")]
    LobbyNotFound(String),
}

#[derive(Debug)]
pub enum GameCommand {
    AddPlayer {
        name: String,
        tx: mpsc::UnboundedSender<String>,
        reply: oneshot::Sender<Result<(), GameError>>,
    },
    RemovePlayer {
        name: String,
    },
    CloseLobby {
        reply: oneshot::Sender<Result<(), GameError>>,
    },
    AnswerColor {
        player_name: String,
        color: String,
        reply: oneshot::Sender<Result<(bool, i32), GameError>>,
    },
    ToggleState {
        specified_colors: Option<Vec<String>>,
        reply: oneshot::Sender<Result<GameState, GameError>>,
    },
    GetState {
        reply: oneshot::Sender<GameStateSnapshot>,
    },
    UpdateLobbyName {
        name: String,
        reply: oneshot::Sender<Result<(), GameError>>,
    },
    Broadcast {
        message: String,
    },
}

#[derive(Clone, Debug)]
pub enum GameEvent {
    PlayerJoined {
        name: String,
    },
    PlayerLeft {
        name: String,
    },
    ColorSelected {
        player: String,
        correct: bool,
        score: i32,
    },
    StateChanged {
        state: GameState,
        colors: Option<Vec<ColorDef>>,
        current_song: Option<Song>,
    },
    RoundEnded,
    NameUpdated {
        name: String,
    },
    LobbyClosed {
        reason: String,
        players: HashMap<String, Player>,
    },
}

#[derive(Clone, Debug)]
pub struct GameStateSnapshot {
    pub id: Uuid,
    pub name: Option<String>,
    pub state: GameState,
    pub players: HashMap<String, PlayerSnapshot>,
    pub round_colors: Vec<ColorDef>,
    pub round_time_left: Option<u64>,
    pub current_song: Option<Song>,
    pub correct_colors: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct PlayerSnapshot {
    pub name: String,
    pub score: i32,
    pub has_answered: bool,
    pub answer: Option<String>,
    pub tx: mpsc::UnboundedSender<String>,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub name: String,
    score: i32,
    has_answered: bool,
    answer: Option<String>,
    pub tx: mpsc::UnboundedSender<String>,
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

    fn to_snapshot(&self) -> PlayerSnapshot {
        PlayerSnapshot {
            name: self.name.clone(),
            score: self.score,
            has_answered: self.has_answered,
            answer: self.answer.clone(),
            tx: self.tx.clone(),
        }
    }
}

pub struct GameCore {
    id: Uuid,
    name: Option<String>,
    players: HashMap<String, Player>,
    all_colors: Vec<ColorDef>,
    round_colors: Vec<ColorDef>,
    correct_colors: Vec<String>,
    state: GameState,
    round_start_time: Option<Instant>,
    round_duration: u64,
    songs: Vec<Song>,
    used_songs: HashSet<String>,
    current_song: Option<Song>,
}

#[derive(Clone)]
pub struct GameHandle {
    id: Uuid,
    tx: mpsc::UnboundedSender<GameCommand>,
}

impl GameHandle {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub async fn add_player(
        &self,
        name: String,
        tx: mpsc::UnboundedSender<String>,
    ) -> Result<(), GameError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(GameCommand::AddPlayer {
                name,
                tx,
                reply: reply_tx,
            })
            .map_err(|_| GameError::ChannelError)?;
        reply_rx.await.map_err(|_| GameError::ChannelError)?
    }

    pub async fn remove_player(&self, name: String) -> Result<(), GameError> {
        self.tx
            .send(GameCommand::RemovePlayer { name })
            .map_err(|_| GameError::ChannelError)
    }

    pub async fn close_lobby(&self) -> Result<(), GameError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(GameCommand::CloseLobby { reply: reply_tx })
            .map_err(|_| GameError::ChannelError)?;
        reply_rx.await.map_err(|_| GameError::ChannelError)?
    }

    pub async fn answer_color(
        &self,
        player_name: String,
        color: String,
    ) -> Result<(bool, i32), GameError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(GameCommand::AnswerColor {
                player_name,
                color,
                reply: reply_tx,
            })
            .map_err(|_| GameError::ChannelError)?;
        reply_rx.await.map_err(|_| GameError::ChannelError)?
    }

    pub async fn toggle_state(
        &self,
        specified_colors: Option<Vec<String>>,
    ) -> Result<GameState, GameError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(GameCommand::ToggleState {
                specified_colors,
                reply: reply_tx,
            })
            .map_err(|_| GameError::ChannelError)?;
        reply_rx.await.map_err(|_| GameError::ChannelError)?
    }

    pub async fn update_name(&self, name: String) -> Result<(), GameError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(GameCommand::UpdateLobbyName {
                name,
                reply: reply_tx,
            })
            .map_err(|_| GameError::ChannelError)?;
        reply_rx.await.map_err(|_| GameError::ChannelError)?
    }

    pub async fn get_state(&self) -> Result<GameStateSnapshot, GameError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(GameCommand::GetState { reply: reply_tx })
            .map_err(|_| GameError::ChannelError)?;
        Ok(reply_rx.await.map_err(|_| GameError::ChannelError)?)
    }

    pub async fn broadcast(&self, message: String) -> Result<(), GameError> {
        self.tx
            .send(GameCommand::Broadcast { message })
            .map_err(|_| GameError::ChannelError)
    }
}

impl GameCore {
    pub fn new(id: Uuid, name: Option<String>, songs: Vec<Song>) -> Self {
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
            id,
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

    pub fn spawn(
        name: Option<String>,
        songs: Vec<Song>,
    ) -> (GameHandle, mpsc::UnboundedReceiver<GameEvent>) {
        let id = Uuid::new_v4();
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let mut core = GameCore::new(id, name, songs);

        tokio::spawn(async move {
            core.run(cmd_rx, event_tx).await;
        });

        (GameHandle { id, tx: cmd_tx }, event_rx)
    }

    async fn run(
        &mut self,
        mut cmd_rx: mpsc::UnboundedReceiver<GameCommand>,
        event_tx: mpsc::UnboundedSender<GameEvent>,
    ) {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                GameCommand::AddPlayer { name, tx, reply } => {
                    if !self.players.contains_key(&name) {
                        self.players.insert(name.clone(), Player::new(&name, tx));
                        if let Err(e) =
                            event_tx.send(GameEvent::PlayerJoined { name: name.clone() })
                        {
                            warn!("Failed to send PlayerJoined event: {:?}", e);
                        }
                        if let Err(e) = reply.send(Ok(())) {
                            warn!("Failed to send reply for AddPlayer command: {:?}", e);
                        }
                    }
                }
                GameCommand::RemovePlayer { name } => {
                    if self.players.remove(&name).is_some() {
                        if let Err(e) = event_tx.send(GameEvent::PlayerLeft { name }) {
                            warn!("Failed to send PlayerLeft event: {:?}", e);
                        }
                    }
                }
                GameCommand::CloseLobby { reply } => {
                    let players = self.players.clone();
                    if let Err(e) = event_tx.send(GameEvent::LobbyClosed {
                        reason: "Admin closed the lobby".to_string(),
                        players,
                    }) {
                        warn!("Failed to send lobby closed event: {:?}", e);
                    }
                    info!("game core close lobby event fired.");
                    if let Err(e) = reply.send(Ok(())) {
                        warn!("Failed to send reply for CloseLobby command: {:?}", e);
                    }
                    break;
                }
                GameCommand::AnswerColor {
                    player_name,
                    color,
                    reply,
                } => {
                    let result = self.check_answer(&player_name, &color);
                    if let Ok((correct, score)) = result {
                        if let Err(e) = event_tx.send(GameEvent::ColorSelected {
                            player: player_name,
                            correct,
                            score,
                        }) {
                            warn!("Failed to send ColorSelected event: {:?}", e);
                        }
                    }
                    if let Err(e) = reply.send(result) {
                        warn!("Failed to send reply for AnswerColor command: {:?}", e);
                    }
                }
                GameCommand::ToggleState {
                    specified_colors,
                    reply,
                } => {
                    let result = self.toggle_state(specified_colors);
                    if let Ok(ref new_state) = result {
                        if let Err(e) = event_tx.send(GameEvent::StateChanged {
                            state: new_state.clone(),
                            colors: Some(self.round_colors.clone()),
                            current_song: self.current_song.clone(),
                        }) {
                            warn!("Failed to send StateChanged event: {:?}", e);
                        }
                    }
                    if let Err(e) = reply.send(result) {
                        warn!("Failed to send reply for ToggleState command: {:?}", e);
                    }
                }
                GameCommand::GetState { reply } => {
                    if let Err(e) = reply.send(GameStateSnapshot {
                        id: self.id,
                        name: self.name.clone(),
                        state: self.state.clone(),
                        players: self
                            .players
                            .iter()
                            .map(|(k, v)| (k.clone(), v.to_snapshot()))
                            .collect(),
                        round_colors: self.round_colors.clone(),
                        round_time_left: self.round_start_time.map(|start| {
                            self.round_duration
                                .saturating_sub(start.elapsed().as_secs())
                        }),
                        current_song: self.current_song.clone(),
                        correct_colors: self.correct_colors.clone(),
                    }) {
                        warn!("Failed to send reply for GetState command: {:?}", e);
                    }
                }
                GameCommand::UpdateLobbyName { name, reply } => {
                    self.name = Some(name.clone());
                    if let Err(e) = event_tx.send(GameEvent::NameUpdated { name }) {
                        warn!("Failed to send NameUpdated event: {:?}", e);
                    }
                    if let Err(e) = reply.send(Ok(())) {
                        warn!("Failed to send reply for UpdateLobbyName command: {:?}", e);
                    }
                }
                GameCommand::Broadcast { message } => {
                    for player in self.players.values() {
                        if let Err(e) = player.tx.send(message.clone()) {
                            warn!(
                                "Failed to broadcast message to player {}: {:?}",
                                player.name, e
                            );
                        }
                    }
                }
            }
        }
        info!("Game loop ended for game {}", self.id);
    }

    pub fn check_answer(
        &mut self,
        player_name: &str,
        color_name: &str,
    ) -> Result<(bool, i32), GameError> {
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

    pub fn toggle_state(
        &mut self,
        specified_colors: Option<Vec<String>>,
    ) -> Result<GameState, GameError> {
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

    fn start_new_round(&mut self, specified_colors: Option<Vec<String>>) -> Result<(), GameError> {
        self.select_round_colors(specified_colors)?;
        self.round_start_time = Some(Instant::now());
        self.state = GameState::Question;
        for p in self.players.values_mut() {
            p.has_answered = false;
            p.answer = None;
        }
        Ok(())
    }

    fn end_round(&mut self) {
        if let Some(song) = &self.current_song {
            self.used_songs.insert(song.uri.clone());
        }
        self.current_song = None;
        self.state = GameState::Score;
    }

    fn select_round_colors(
        &mut self,
        specified_colors: Option<Vec<String>>,
    ) -> Result<(), GameError> {
        self.round_colors.clear();
        self.correct_colors.clear();

        match specified_colors {
            Some(colors) => self.select_specified_colors(colors),
            None => self.select_random_colors(),
        }
    }

    fn select_specified_colors(&mut self, colors: Vec<String>) -> Result<(), GameError> {
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

    fn select_random_colors(&mut self) -> Result<(), GameError> {
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

    fn setup_round_colors(
        &mut self,
        chosen_correct_colors: Vec<ColorDef>,
    ) -> Result<(), GameError> {
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
