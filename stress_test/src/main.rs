use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use rand::seq::SliceRandom;
use rand::Rng;
use rand::SeedableRng;
use serde_json::json;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;
use clap::{Parser, ValueEnum};

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

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum TestMode {
    #[default]
    ThroughputTest,
    GameplayTest,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Test mode to run
    #[arg(value_enum, default_value = "throughput-test")]
    mode: TestMode,

    /// Number of concurrent lobbies/games
    #[arg(short, long, default_value_t = 10)]
    num_lobbies: usize,

    /// Number of players per lobby
    #[arg(short, long, default_value_t = 10)]
    players_per_lobby: usize,

    /// Test duration in seconds (for throughput test) or rounds per game (for gameplay test)
    #[arg(short, long, default_value_t = 60)]
    duration_or_rounds: u64,
}

// Shared metrics for throughput testing
struct ThroughputMetrics {
    messages_sent: AtomicU64,
    messages_received: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
}

impl ThroughputMetrics {
    fn new() -> Self {
        Self {
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
        }
    }
}

// Shared metrics for gameplay testing
struct GameplayMetrics {
    active_games: AtomicUsize,
    completed_games: AtomicUsize,
    errors: AtomicUsize,
}

impl GameplayMetrics {
    fn new() -> Self {
        Self {
            active_games: AtomicUsize::new(0),
            completed_games: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
        }
    }
}

// Player implementation for gameplay testing
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
            self.write.send(Message::Text(answer_msg.to_string())).await?;
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
                        self.player_id = Some(Uuid::parse_str(data["player_id"].as_str().unwrap())?);
                        self.lobby_id = Some(Uuid::parse_str(data["lobby_id"].as_str().unwrap())?);
                    }
                    Some("StateChanged") => {
                        if data["phase"].as_str() == Some("question") {
                            let delay = self.rng.gen_range(0.0..3.0);
                            tokio::time::sleep(Duration::from_secs_f32(delay)).await;

                            if let Some(alternatives) = data["alternatives"].as_array() {
                                if let Some(answer) = alternatives.choose(&mut self.rng) {
                                    self.submit_answer(answer.as_str().unwrap().to_string()).await?;
                                }
                            }
                        }
                    }
                    Some("GameOver") | Some("GameClosed") => return Ok(()),
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

// Admin implementation for gameplay testing
struct TestAdmin {
    lobby_id: Uuid,
    write: SplitSink<WsStream, Message>,
    read: SplitStream<WsStream>,
}

impl TestAdmin {
    async fn new(join_code: String, admin_id: Uuid, lobby_id: Uuid) -> Result<Self, TestError> {
        let (ws_stream, _) = connect_async("ws://localhost:8765/ws").await?;
        let (write, read) = ws_stream.split();

        let mut admin = Self {
            lobby_id,
            write,
            read,
        };

        admin.join_lobby(join_code, admin_id).await?;
        Ok(admin)
    }

    async fn join_lobby(&mut self, join_code: String, admin_id: Uuid) -> Result<(), TestError> {
        let join_msg = json!({
            "type": "JoinLobby",
            "join_code": join_code,
            "admin_id": admin_id,
            "name": "Admin"
        });
        self.write.send(Message::Text(join_msg.to_string())).await?;
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

    async fn handle_messages(&mut self) -> Result<(), TestError> {
        while let Some(msg) = self.read.next().await {
            if let Message::Text(text) = msg? {
                let data: serde_json::Value = serde_json::from_str(&text)?;
                if data["type"].as_str() == Some("GameOver") {
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}

// Throughput test implementation
async fn run_throughput_player(
    join_code: String,
    name: String,
    metrics: Arc<ThroughputMetrics>,
    test_duration: Duration,
) -> Result<(), TestError> {
    let (ws_stream, _) = connect_async("ws://localhost:8765/ws").await?;
    let (mut write, mut read) = ws_stream.split();

    let join_msg = json!({
        "type": "JoinLobby",
        "join_code": join_code,
        "admin_id": null,
        "name": name
    });
    write.send(Message::Text(join_msg.to_string().into())).await?;

    let mut lobby_id = None;
    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            let data: serde_json::Value = serde_json::from_str(&text)?;
            if data["type"].as_str() == Some("JoinedLobby") {
                lobby_id = Some(Uuid::parse_str(data["lobby_id"].as_str().unwrap())?);
                break;
            }
        }
    }

    if let Some(lobby_id) = lobby_id {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        
        let metrics_clone = Arc::clone(&metrics);
        let mut write = write;
        let sender = tokio::spawn(async move {
            let start = Instant::now();
            while start.elapsed() < test_duration {
                let msg = json!({
                    "type": "Answer",
                    "lobby_id": lobby_id,
                    "answer": "stress_test"
                });
                let msg_str = msg.to_string();
                let size = msg_str.len();
                
                if write.send(Message::Text(msg_str.into())).await.is_ok() {
                    metrics_clone.messages_sent.fetch_add(1, Ordering::Relaxed);
                    metrics_clone.bytes_sent.fetch_add(size as u64, Ordering::Relaxed);
                } else {
                    break;
                }
            }
            let _ = tx.send(());
        });

        let metrics_clone = Arc::clone(&metrics);
        let receiver = tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                if let Ok(Message::Text(text)) = msg {
                    metrics_clone.messages_received.fetch_add(1, Ordering::Relaxed);
                    metrics_clone.bytes_received.fetch_add(text.len() as u64, Ordering::Relaxed);
                }
            }
        });

        let _ = rx.recv().await;
        sender.abort();
        receiver.abort();
    }

    Ok(())
}

async fn run_metrics_reporter(metrics: Arc<ThroughputMetrics>, start_time: Instant, test_duration: Duration) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut last_sent = 0u64;
    let mut last_received = 0u64;
    let mut last_bytes_sent = 0u64;
    let mut last_bytes_received = 0u64;

    println!("\nPerformance Metrics:");
    println!("{:<12} {:<12} {:<12} {:<12} {:<12} {:<12} {:<12} {:<12}", 
        "Msgs Sent", "Msgs/sec", "Msgs Rcvd", "Msgs/sec", 
        "MB Sent", "MB/sec", "MB Rcvd", "MB/sec");
    println!("{}", "-".repeat(96));

    while start_time.elapsed() < test_duration {
        interval.tick().await;

        let total_sent = metrics.messages_sent.load(Ordering::Relaxed);
        let total_received = metrics.messages_received.load(Ordering::Relaxed);
        let total_bytes_sent = metrics.bytes_sent.load(Ordering::Relaxed);
        let total_bytes_received = metrics.bytes_received.load(Ordering::Relaxed);

        let sent_rate = total_sent - last_sent;
        let received_rate = total_received - last_received;
        let bytes_sent_rate = (total_bytes_sent - last_bytes_sent) as f64 / 1_048_576.0;
        let bytes_received_rate = (total_bytes_received - last_bytes_received) as f64 / 1_048_576.0;

        println!("{:<12} {:<12} {:<12} {:<12} {:<12.2} {:<12.2} {:<12.2} {:<12.2}",
            total_sent,
            sent_rate,
            total_received,
            received_rate,
            total_bytes_sent as f64 / 1_048_576.0,
            bytes_sent_rate,
            total_bytes_received as f64 / 1_048_576.0,
            bytes_received_rate
        );

        last_sent = total_sent;
        last_received = total_received;
        last_bytes_sent = total_bytes_sent;
        last_bytes_received = total_bytes_received;
    }
}

async fn run_throughput_test(
    num_lobbies: usize,
    players_per_lobby: usize,
    test_duration: Duration,
) -> Result<(), TestError> {
    let metrics = Arc::new(ThroughputMetrics::new());
    let start_time = Instant::now();
    let mut handles = vec![];

    // Spawn metrics reporter
    let metrics_clone = Arc::clone(&metrics);
    let reporter_handle = tokio::spawn(run_metrics_reporter(metrics_clone, start_time, test_duration));

    // Create lobbies and spawn players
    for lobby_idx in 0..num_lobbies {
        let join_code = create_lobby().await?;
        println!("Created lobby {} with code {}", lobby_idx + 1, join_code);

        for player_idx in 0..players_per_lobby {
            let name = format!("Lobby{}Player{}", lobby_idx + 1, player_idx + 1);
            let metrics = Arc::clone(&metrics);
            let join_code = join_code.clone();
            
            let handle = tokio::spawn(async move {
                if let Err(e) = run_throughput_player(join_code, name, metrics, test_duration).await {
                    eprintln!("Player error: {}", e);
                }
            });
            handles.push(handle);
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    tokio::time::sleep(test_duration).await;

    // Print final statistics
    let duration = start_time.elapsed().as_secs_f64();
    let total_sent = metrics.messages_sent.load(Ordering::Relaxed);
    let total_received = metrics.messages_received.load(Ordering::Relaxed);

    println!("\nTest Complete");
    println!("Duration: {:.2} seconds", duration);
    println!("Total Messages Sent: {}", total_sent);
    println!("Total Messages Received: {}", total_received);
    println!("Average Send Rate: {:.2} msgs/sec", total_sent as f64 / duration);
    println!("Average Receive Rate: {:.2} msgs/sec", total_received as f64 / duration);
    println!("Total Data Sent: {:.2} MB", metrics.bytes_sent.load(Ordering::Relaxed) as f64 / 1_048_576.0);
    println!("Total Data Received: {:.2} MB", metrics.bytes_received.load(Ordering::Relaxed) as f64 / 1_048_576.0);

    reporter_handle.abort();
    Ok(())
}

// Gameplay test implementation
async fn run_game_batch(
    batch_size: usize,
    players_per_game: usize,
    rounds: usize,
) -> Result<(), TestError> {
    let metrics = Arc::new(GameplayMetrics::new());
    let mut game_handles = Vec::new();
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

    let admin_handle: tokio::task::JoinHandle<Result<(), _>> = tokio::spawn(async move {
        match async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            admin.run_game(rounds).await?;
            admin.handle_messages().await
        }
        .await
        {
            Ok(_) => Ok::<(), std::io::Error>(()),
            Err(e) => {
                eprintln!("Admin error in game {}: {}", game_idx, e);
                Ok(())
            }
        }
    });

    for handle in player_handles {
        handle.await.map_err(|e| TestError::Other(e.to_string()))?;
    }
    admin_handle.await.map_err(|e| TestError::Other(e.to_string()))?;

    Ok(())
}

// Shared utility functions
async fn create_lobby() -> Result<String, TestError> {
    let response = reqwest::Client::new()
        .post("http://localhost:8765/api/lobbies")
        .json(&json!({ "round_duration": 60 }))
        .send()
        .await?;

    let data: serde_json::Value = response.json().await?;
    Ok(data["join_code"].as_str().unwrap().to_string())
}

#[tokio::main]
async fn main() -> Result<(), TestError> {
    let args = Args::parse();

    match args.mode {
        TestMode::ThroughputTest => {
            println!("Starting throughput test with:");
            println!("Number of lobbies: {}", args.num_lobbies);
            println!("Players per lobby: {}", args.players_per_lobby);
            println!("Test duration: {} seconds", args.duration_or_rounds);

            run_throughput_test(
                args.num_lobbies,
                args.players_per_lobby,
                Duration::from_secs(args.duration_or_rounds),
            )
            .await
        }
        TestMode::GameplayTest => {
            println!("Starting gameplay test with:");
            println!("Batch size: {}", args.num_lobbies);
            println!("Players per game: {}", args.players_per_lobby);
            println!("Rounds per game: {}", args.duration_or_rounds);

            run_game_batch(args.num_lobbies, args.players_per_lobby, args.duration_or_rounds as usize).await
        }
    }
}
