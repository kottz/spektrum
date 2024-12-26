use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use rand::seq::SliceRandom;
use rand::Rng;
use rand::SeedableRng;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug, thiserror::Error)]
enum TestError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("UUID error: {0}")]
    Uuid(#[from] uuid::Error),
    #[error("Generic error: {0}")]
    Other(String),
}

struct TestMetrics {
    active_games: AtomicUsize,
    completed_games: AtomicUsize,
    errors: AtomicUsize,
}

#[derive(Debug)]
struct TestPlayer {
    name: String,
    join_code: String,
    player_id: Option<Uuid>,
    lobby_id: Option<Uuid>,
    write: SplitSink<WsStream, Message>,
    read: SplitStream<WsStream>,
    rng: rand::rngs::StdRng,
}

impl TestPlayer {
    async fn new(name: String, join_code: String) -> Result<Self, TestError> {
        let (ws_stream, _) = connect_async("ws://localhost:8765/ws").await?;
        let (write, read) = ws_stream.split();

        Ok(Self {
            name,
            join_code,
            player_id: None,
            lobby_id: None,
            write,
            read,
            rng: rand::rngs::StdRng::from_entropy(),
        })
    }

    async fn join_lobby(&mut self) -> Result<(), TestError> {
        let join_msg = json!({
            "type": "JoinLobby",
            "join_code": self.join_code,
            "admin_id": null,
            "name": self.name
        });

        self.write.send(Message::Text(join_msg.to_string())).await?;
        Ok(())
    }

    async fn submit_answer(&mut self, answer: String) -> Result<(), TestError> {
        if let Some(lobby_id) = self.lobby_id {
            let answer_msg = json!({
                "type": "Answer",
                "lobby_id": lobby_id,
                "answer": answer
            });
            self.write
                .send(Message::Text(answer_msg.to_string()))
                .await?;
        }
        Ok(())
    }

    async fn handle_messages(&mut self) -> Result<(), TestError> {
        while let Some(msg) = self.read.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                let data: serde_json::Value = serde_json::from_str(&text)?;

                match data["type"].as_str() {
                    Some("JoinedLobby") => {
                        self.player_id =
                            Some(Uuid::parse_str(data["player_id"].as_str().unwrap())?);
                        self.lobby_id = Some(Uuid::parse_str(data["lobby_id"].as_str().unwrap())?);
                    }
                    Some("StateChanged") => {
                        if data["phase"].as_str() == Some("question") {
                            let delay = self.rng.gen_range(0.0..3.0);
                            tokio::time::sleep(Duration::from_secs_f32(delay)).await;

                            if let Some(alternatives) = data["alternatives"].as_array() {
                                if let Some(answer) = alternatives.choose(&mut self.rng) {
                                    self.submit_answer(answer.as_str().unwrap().to_string())
                                        .await?;
                                }
                            }
                        }
                    }
                    Some("GameOver") | Some("GameClosed") => {
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

struct TestAdmin {
    name: String,
    admin_id: Uuid,
    lobby_id: Uuid,
    write: SplitSink<WsStream, Message>,
    read: SplitStream<WsStream>,
}

impl TestAdmin {
    async fn new(join_code: String, admin_id: Uuid, lobby_id: Uuid) -> Result<Self, TestError> {
        let (ws_stream, _) = connect_async("ws://localhost:8765/ws").await?;
        let (write, read) = ws_stream.split();

        let mut admin = Self {
            name: "Admin".to_string(),
            admin_id,
            lobby_id,
            write,
            read,
        };

        admin.join_lobby(join_code).await?;
        Ok(admin)
    }

    async fn join_lobby(&mut self, join_code: String) -> Result<(), TestError> {
        let join_msg = json!({
            "type": "JoinLobby",
            "join_code": join_code,
            "admin_id": self.admin_id,
            "name": self.name
        });
        self.write.send(Message::Text(join_msg.to_string())).await?;
        Ok(())
    }

    async fn start_game(&mut self) -> Result<(), TestError> {
        let msg = json!({
            "type": "AdminAction",
            "lobby_id": self.lobby_id,
            "action": {
                "type": "StartGame"
            }
        });
        self.write.send(Message::Text(msg.to_string())).await?;
        Ok(())
    }

    async fn start_round(&mut self) -> Result<(), TestError> {
        let msg = json!({
            "type": "AdminAction",
            "lobby_id": self.lobby_id,
            "action": {
                "type": "StartRound",
                "specified_alternatives": null
            }
        });
        self.write.send(Message::Text(msg.to_string())).await?;
        Ok(())
    }

    async fn end_round(&mut self) -> Result<(), TestError> {
        let msg = json!({
            "type": "AdminAction",
            "lobby_id": self.lobby_id,
            "action": {
                "type": "EndRound"
            }
        });
        self.write.send(Message::Text(msg.to_string())).await?;
        Ok(())
    }

    async fn end_game(&mut self) -> Result<(), TestError> {
        let msg = json!({
            "type": "AdminAction",
            "lobby_id": self.lobby_id,
            "action": {
                "type": "EndGame",
                "reason": "Test complete"
            }
        });
        self.write.send(Message::Text(msg.to_string())).await?;
        Ok(())
    }

    async fn run_game(&mut self, rounds: usize) -> Result<(), TestError> {
        self.start_game().await?;

        for _ in 0..rounds {
            self.start_round().await?;
            tokio::time::sleep(Duration::from_secs(5)).await;
            self.end_round().await?;
            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        self.end_game().await?;
        Ok(())
    }

    async fn handle_messages(&mut self) -> Result<(), TestError> {
        while let Some(msg) = self.read.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                let data: serde_json::Value = serde_json::from_str(&text)?;
                if data["type"].as_str() == Some("GameOver") {
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}

async fn run_game_batch(
    batch_size: usize,
    players_per_game: usize,
    rounds: usize,
) -> Result<(), TestError> {
    let metrics = Arc::new(TestMetrics {
        active_games: AtomicUsize::new(0),
        completed_games: AtomicUsize::new(0),
        errors: AtomicUsize::new(0),
    });

    let mut game_handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    let batch_start = Instant::now();

    for game_idx in 0..batch_size {
        let metrics = Arc::clone(&metrics);
        let game_handle = tokio::spawn(async move {
            metrics.active_games.fetch_add(1, Ordering::SeqCst);

            if let Err(e) = run_single_game(game_idx, players_per_game, rounds).await {
                eprintln!("Game {} error: {}", game_idx, e);
                metrics.errors.fetch_add(1, Ordering::SeqCst);
            }

            metrics.active_games.fetch_sub(1, Ordering::SeqCst);
            metrics.completed_games.fetch_add(1, Ordering::SeqCst);
        });
        game_handles.push(game_handle);

        // Rate limit game creation to avoid overwhelming the server
        if game_idx % 100 == 99 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    // Print metrics every second
    let metrics_clone = Arc::clone(&metrics);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            println!(
                "Active: {}, Completed: {}, Errors: {}",
                metrics_clone.active_games.load(Ordering::SeqCst),
                metrics_clone.completed_games.load(Ordering::SeqCst),
                metrics_clone.errors.load(Ordering::SeqCst)
            );
        }
    });

    for handle in game_handles {
        handle.await.map_err(|e| TestError::Other(e.to_string()))?;
    }

    let duration = batch_start.elapsed();
    println!(
        "Batch complete - Total games: {}, Duration: {:.2}s, Games/sec: {:.2}",
        batch_size,
        duration.as_secs_f64(),
        batch_size as f64 / duration.as_secs_f64()
    );

    Ok(())
}

async fn run_single_game(
    game_idx: usize,
    players_per_game: usize,
    rounds: usize,
) -> Result<(), TestError> {
    let response = reqwest::Client::new()
        .post("http://localhost:8765/api/lobbies")
        .json(&json!({ "round_duration": 60 }))
        .send()
        .await?;

    let lobby_data: serde_json::Value = response.json().await?;
    let join_code = lobby_data["join_code"].as_str().unwrap().to_string();
    let admin_id = Uuid::parse_str(lobby_data["admin_id"].as_str().unwrap())?;
    let lobby_id = Uuid::parse_str(lobby_data["lobby_id"].as_str().unwrap())?;

    let mut admin = TestAdmin::new(join_code.clone(), admin_id, lobby_id).await?;

    // Create all players first
    let mut players = Vec::new();
    for i in 0..players_per_game {
        let mut player = TestPlayer::new(
            format!("Game{}Player{}", game_idx, i + 1),
            join_code.clone(),
        )
        .await?;
        player.join_lobby().await?;
        players.push(player);
    }

    // Wait for joins to be processed
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut player_handles: Vec<tokio::task::JoinHandle<Result<(), ()>>> = Vec::new();

    for mut player in players {
        player_handles.push(tokio::spawn(async move {
            match player.handle_messages().await {
                Ok(_) => Ok(()),
                Err(e) => {
                    eprintln!("Player error in game {}: {}", game_idx, e);
                    Ok(())
                }
            }
        }));
    }

    let admin_handle: tokio::task::JoinHandle<Result<(), ()>> = tokio::spawn(async move {
        match async {
            // Wait a bit before starting game
            tokio::time::sleep(Duration::from_millis(200)).await;
            admin.run_game(rounds).await?;
            admin.handle_messages().await
        }
        .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("Admin error in game {}: {}", game_idx, e);
                Ok(())
            }
        }
    });

    for handle in player_handles {
        handle.await.map_err(|e| TestError::Other(e.to_string()))?;
    }
    admin_handle
        .await
        .map_err(|e| TestError::Other(e.to_string()))?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), TestError> {
    let total_games = 10_000;
    let batch_size = 4000;
    let players_per_game = 5;
    let rounds_per_game = 3;

    for batch in 0..(total_games / batch_size) {
        println!("Starting batch {}", batch);
        run_game_batch(batch_size, players_per_game, rounds_per_game).await?;
    }

    Ok(())
}
