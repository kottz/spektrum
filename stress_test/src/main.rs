use clap::{Parser, ValueEnum};
use futures_util::{SinkExt, StreamExt};
use rand::seq::SliceRandom;
use rand::Rng;
use rand::SeedableRng;
use serde_json::json;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Error type used by the stress test.
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

/// Command‐line options.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum TestMode {
    ThroughputTest,
    GameplayTest,
    #[default]
    UiTest,
}

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Test mode to run
    #[arg(value_enum, default_value = "ui-test")]
    mode: TestMode,

    /// Number of concurrent lobbies/games (or batch size for gameplay test)
    #[arg(short, long, default_value_t = 10)]
    num_lobbies: usize,

    /// Number of players per lobby (or per game)
    #[arg(short, long, default_value_t = 10)]
    players_per_lobby: usize,

    /// Test duration in seconds (for throughput test) or rounds per game (for gameplay/ui test)
    #[arg(short, long, default_value_t = 60)]
    duration_or_rounds: u64,

    /// Join code (required for UI test mode)
    #[arg(short, long)]
    join_code: Option<String>,

    /// Host address (e.g., "localhost:8765")
    #[arg(long, default_value = "localhost:8765")]
    host: String,
}

/// Shared metrics for throughput testing.
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

/// Shared metrics for gameplay/UI tests.
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

/// A test player simulates a client joining a lobby via HTTP and then opening a WebSocket.
/// After joining, it sends a Connect message (using the new protocol) and later submits answers.
struct TestPlayer {
    name: String,
    join_code: String,
    player_id: Uuid,
    ws_write: futures_util::stream::SplitSink<WsStream, Message>,
    ws_read: futures_util::stream::SplitStream<WsStream>,
    rng: rand::rngs::StdRng,
}

impl TestPlayer {
    /// Create a new test player.
    /// First it calls the HTTP endpoint `/api/join-lobby` with its name and join code to obtain a player ID.
    /// Then it connects to the WebSocket endpoint and sends a `Connect { player_id }` message.
    async fn new(name: String, join_code: String, host: &str) -> Result<Self, TestError> {
        // Join lobby via HTTP POST
        let join_url = format!("http://{}/api/join-lobby", host);
        let client = reqwest::Client::new();
        let res = client
            .post(&join_url)
            .json(&json!({
                "join_code": join_code,
                "name": name,
            }))
            .send()
            .await?;
        let join_response: serde_json::Value = res.json().await?;
        let player_id_str = join_response["player_id"]
            .as_str()
            .ok_or_else(|| TestError::Other("Missing player_id in join response".to_string()))?;
        let player_id = Uuid::parse_str(player_id_str)?;
        // Connect via WebSocket
        let ws_url = format!("ws://{}/ws", host);
        let (ws_stream, _) = connect_async(&ws_url).await?;
        let (mut write, read) = ws_stream.split();
        // Send the new protocol connect message
        let connect_msg = json!({
            "type": "Connect",
            "player_id": player_id.to_string(),
        });
        write.send(Message::Text(connect_msg.to_string())).await?;
        Ok(Self {
            name,
            join_code,
            player_id,
            ws_write: write,
            ws_read: read,
            rng: rand::rngs::StdRng::from_entropy(),
        })
    }

    /// Submit an answer over the WebSocket.
    async fn submit_answer(&mut self, answer: String) -> Result<(), TestError> {
        let answer_msg = json!({
            "type": "Answer",
            "answer": answer,
        });
        self.ws_write
            .send(Message::Text(answer_msg.to_string()))
            .await?;
        Ok(())
    }

    /// Process incoming WebSocket messages.
    /// For example, when a StateDelta is received with phase "question", wait a random delay
    /// then pick one of the alternatives and submit it as the answer.
    async fn handle_messages(&mut self) -> Result<(), TestError> {
        while let Some(msg) = self.ws_read.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                let data: serde_json::Value = serde_json::from_str(&text)?;
                match data["type"].as_str() {
                    Some("Connected") => {
                        // Connection acknowledgement received.
                    }
                    Some("StateDelta") => {
                        if data["phase"].as_str() == Some("question") {
                            // Simulate thinking time before answering.
                            let delay = self.rng.gen_range(0.0..40.0);
                            tokio::time::sleep(Duration::from_secs_f32(delay)).await;
                            if let Some(alternatives) = data["alternatives"].as_array() {
                                if let Some(answer_val) = alternatives.choose(&mut self.rng) {
                                    if let Some(answer_str) = answer_val.as_str() {
                                        self.submit_answer(answer_str.to_string()).await?;
                                    }
                                }
                            }
                        }
                    }
                    Some("GameOver") | Some("GameClosed") => break,
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

/// A test admin joins a lobby (using the HTTP create endpoint) and then opens a WebSocket connection
/// to send admin actions.
struct TestAdmin {
    player_id: Uuid,
    ws_write: futures_util::stream::SplitSink<WsStream, Message>,
    ws_read: futures_util::stream::SplitStream<WsStream>,
}

impl TestAdmin {
    /// Create a new lobby via HTTP POST to `/api/create-lobby` and then connect via WS.
    /// Returns both the TestAdmin instance and the lobby's join code.
    async fn new(host: &str) -> Result<(Self, String), TestError> {
        let create_url = format!("http://{}/api/create-lobby", host);
        let client = reqwest::Client::new();
        let res = client
            .post(&create_url)
            .json(&json!({ "round_duration": 60 }))
            .send()
            .await?;
        let create_response: serde_json::Value = res.json().await?;
        let join_code = create_response["join_code"]
            .as_str()
            .ok_or_else(|| {
                TestError::Other("Missing join_code in create lobby response".to_string())
            })?
            .to_string();
        let admin_id_str = create_response["player_id"].as_str().ok_or_else(|| {
            TestError::Other("Missing player_id in create lobby response".to_string())
        })?;
        let admin_id = Uuid::parse_str(admin_id_str)?;
        // Connect via WebSocket
        let ws_url = format!("ws://{}/ws", host);
        let (ws_stream, _) = connect_async(&ws_url).await?;
        let (mut write, read) = ws_stream.split();
        let connect_msg = json!({
            "type": "Connect",
            "player_id": admin_id.to_string(),
        });
        write.send(Message::Text(connect_msg.to_string())).await?;
        Ok((
            Self {
                player_id: admin_id,
                ws_write: write,
                ws_read: read,
            },
            join_code,
        ))
    }

    async fn start_game(&mut self) -> Result<(), TestError> {
        let msg = json!({
            "type": "AdminAction",
            "action": { "type": "StartGame" }
        });
        self.ws_write.send(Message::Text(msg.to_string())).await?;
        Ok(())
    }

    async fn start_round(&mut self) -> Result<(), TestError> {
        let msg = json!({
            "type": "AdminAction",
            "action": { "type": "StartRound" }
        });
        self.ws_write.send(Message::Text(msg.to_string())).await?;
        Ok(())
    }

    async fn end_round(&mut self) -> Result<(), TestError> {
        let msg = json!({
            "type": "AdminAction",
            "action": { "type": "EndRound" }
        });
        self.ws_write.send(Message::Text(msg.to_string())).await?;
        Ok(())
    }

    async fn end_game(&mut self) -> Result<(), TestError> {
        let msg = json!({
            "type": "AdminAction",
            "action": { "type": "EndGame", "reason": "Test complete" }
        });
        self.ws_write.send(Message::Text(msg.to_string())).await?;
        Ok(())
    }

    async fn handle_messages(&mut self) -> Result<(), TestError> {
        while let Some(msg) = self.ws_read.next().await {
            if let Message::Text(text) = msg? {
                let data: serde_json::Value = serde_json::from_str(&text)?;
                if data["type"].as_str() == Some("GameOver") {
                    break;
                }
            }
        }
        Ok(())
    }
}

/// Throughput test: Each player repeatedly sends answer messages (with a fixed “stress_test” answer)
/// and we count the messages and bytes.
async fn run_throughput_player(
    join_code: String,
    name: String,
    host: &str,
    metrics: Arc<ThroughputMetrics>,
    test_duration: Duration,
) -> Result<(), TestError> {
    let player = TestPlayer::new(name, join_code, host).await?;
    let sender_metrics = Arc::clone(&metrics);
    let mut ws_write = player.ws_write;
    let sender = tokio::spawn(async move {
        let start = Instant::now();
        while start.elapsed() < test_duration {
            let msg = json!({
                "type": "Answer",
                "answer": "stress_test"
            });
            let msg_str = msg.to_string();
            if ws_write.send(Message::Text(msg_str.clone())).await.is_ok() {
                sender_metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
                sender_metrics
                    .bytes_sent
                    .fetch_add(msg_str.len() as u64, Ordering::Relaxed);
            } else {
                break;
            }
        }
    });
    let receiver_metrics = Arc::clone(&metrics);
    let mut ws_read = player.ws_read;
    let receiver = tokio::spawn(async move {
        while let Some(msg) = ws_read.next().await {
            if let Ok(Message::Text(text)) = msg {
                receiver_metrics
                    .messages_received
                    .fetch_add(1, Ordering::Relaxed);
                receiver_metrics
                    .bytes_received
                    .fetch_add(text.len() as u64, Ordering::Relaxed);
            }
        }
    });
    tokio::time::sleep(test_duration).await;
    sender.abort();
    receiver.abort();
    Ok(())
}

/// Report throughput metrics every second.
async fn run_metrics_reporter(
    metrics: Arc<ThroughputMetrics>,
    start_time: Instant,
    test_duration: Duration,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut last_sent = 0u64;
    let mut last_received = 0u64;
    let mut last_bytes_sent = 0u64;
    let mut last_bytes_received = 0u64;

    println!("\nPerformance Metrics:");
    println!(
        "{:<12} {:<12} {:<12} {:<12} {:<12} {:<12} {:<12} {:<12}",
        "Msgs Sent", "Msgs/sec", "Msgs Rcvd", "Msgs/sec", "MB Sent", "MB/sec", "MB Rcvd", "MB/sec"
    );
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

        println!(
            "{:<12} {:<12} {:<12} {:<12} {:<12.2} {:<12.2} {:<12.2} {:<12.2}",
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

/// Run the throughput test: for each lobby (created via HTTP create),
/// spawn a set of players that repeatedly send answers.
async fn run_throughput_test(
    num_lobbies: usize,
    players_per_lobby: usize,
    test_duration: Duration,
    host: &str,
) -> Result<(), TestError> {
    let metrics = Arc::new(ThroughputMetrics::new());
    let start_time = Instant::now();
    let mut handles = vec![];

    let metrics_clone = Arc::clone(&metrics);
    let reporter_handle = tokio::spawn(run_metrics_reporter(
        metrics_clone,
        start_time,
        test_duration,
    ));

    // Create lobbies and spawn players
    for _ in 0..num_lobbies {
        // Create lobby via HTTP
        let create_url = format!("http://{}/api/create-lobby", host);
        let client = reqwest::Client::new();
        let res = client
            .post(&create_url)
            .json(&json!({ "round_duration": 60 }))
            .send()
            .await?;
        let create_response: serde_json::Value = res.json().await?;
        let join_code = create_response["join_code"]
            .as_str()
            .ok_or_else(|| {
                TestError::Other("Missing join_code in create lobby response".to_string())
            })?
            .to_string();

        for i in 0..players_per_lobby {
            let name = format!("LobbyPlayer{}", i + 1);
            let join_code_clone = join_code.clone();
            let host_clone = host.to_string();
            let metrics_clone = Arc::clone(&metrics);
            let handle = tokio::spawn(async move {
                if let Err(e) = run_throughput_player(
                    join_code_clone,
                    name,
                    &host_clone,
                    metrics_clone,
                    test_duration,
                )
                .await
                {
                    eprintln!("Player error: {}", e);
                }
            });
            handles.push(handle);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    tokio::time::sleep(test_duration).await;

    let duration = start_time.elapsed().as_secs_f64();
    let total_sent = metrics.messages_sent.load(Ordering::Relaxed);
    let total_received = metrics.messages_received.load(Ordering::Relaxed);

    println!("\nTest Complete");
    println!("Duration: {:.2} seconds", duration);
    println!("Total Messages Sent: {}", total_sent);
    println!("Total Messages Received: {}", total_received);
    println!(
        "Average Send Rate: {:.2} msgs/sec",
        total_sent as f64 / duration
    );
    println!(
        "Average Receive Rate: {:.2} msgs/sec",
        total_received as f64 / duration
    );
    println!(
        "Total Data Sent: {:.2} MB",
        metrics.bytes_sent.load(Ordering::Relaxed) as f64 / 1_048_576.0
    );
    println!(
        "Total Data Received: {:.2} MB",
        metrics.bytes_received.load(Ordering::Relaxed) as f64 / 1_048_576.0
    );

    reporter_handle.abort();
    Ok(())
}

/// Gameplay test: spawn several games in parallel. In each game, an admin (created via HTTP)
/// and a set of players join the lobby; then the admin drives rounds by sending admin actions.
async fn run_game_batch(
    batch_size: usize,
    players_per_game: usize,
    rounds: usize,
    host: &str,
) -> Result<(), TestError> {
    let metrics = Arc::new(GameplayMetrics::new());
    let mut game_handles = Vec::new();
    let batch_start = Instant::now();

    for game_idx in 0..batch_size {
        let host_clone = host.to_string();
        let metrics_clone = Arc::clone(&metrics);
        let game_handle = tokio::spawn(async move {
            metrics_clone.active_games.fetch_add(1, Ordering::SeqCst);
            if let Err(e) = run_single_game(game_idx, players_per_game, rounds, &host_clone).await {
                eprintln!("Game {} error: {}", game_idx, e);
                metrics_clone.errors.fetch_add(1, Ordering::SeqCst);
            }
            metrics_clone.active_games.fetch_sub(1, Ordering::SeqCst);
            metrics_clone.completed_games.fetch_add(1, Ordering::SeqCst);
        });
        game_handles.push(game_handle);

        if game_idx % 100 == 99 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

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

/// For a single game: create a lobby via HTTP, have the admin (via TestAdmin) join and drive the game,
/// and have several players join (via HTTP then WS). Then the admin sends start/end round actions.
async fn run_single_game(
    game_idx: usize,
    players_per_game: usize,
    rounds: usize,
    host: &str,
) -> Result<(), TestError> {
    // Create lobby via HTTP
    let create_url = format!("http://{}/api/create-lobby", host);
    let client = reqwest::Client::new();
    let res = client
        .post(&create_url)
        .json(&json!({ "round_duration": 60 }))
        .send()
        .await?;
    let lobby_data: serde_json::Value = res.json().await?;
    let join_code = lobby_data["join_code"].as_str().unwrap().to_string();
    // Admin's player_id is returned in the create response.
    let _admin_id = Uuid::parse_str(lobby_data["player_id"].as_str().unwrap())?;

    // Create admin via TestAdmin::new (which also connects via WS)
    let (mut admin, _) = TestAdmin::new(host).await?;

    // Create players (HTTP join then WS connect)
    let mut players = Vec::new();
    for i in 0..players_per_game {
        let player_name = format!("Game{}Player{}", game_idx, i + 1);
        let player = TestPlayer::new(player_name, join_code.clone(), host).await?;
        players.push(player);
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut player_handles = Vec::new();
    for mut player in players {
        player_handles.push(tokio::spawn(async move {
            if let Err(e) = player.handle_messages().await {
                eprintln!("Player error in game {}: {}", game_idx, e);
            }
        }));
    }
    let admin_handle = tokio::spawn(async move {
        if let Err(e) = async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            admin.start_game().await?;
            for _ in 0..rounds {
                admin.start_round().await?;
                tokio::time::sleep(Duration::from_secs(5)).await;
                admin.end_round().await?;
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            admin.end_game().await?;
            admin.handle_messages().await
        }
        .await
        {
            eprintln!("Admin error in game {}: {}", game_idx, e);
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

/// UI test: players join using a provided join code and then simply process incoming messages.
async fn run_ui_test(
    join_code: String,
    num_players: usize,
    rounds: usize,
    host: &str,
) -> Result<(), TestError> {
    println!("Starting UI test with:");
    println!("Join code: {}", join_code);
    println!("Number of players: {}", num_players);
    println!("Rounds: {}", rounds);

    let metrics = Arc::new(GameplayMetrics::new());
    let mut players = Vec::new();
    for i in 0..num_players {
        let player_name = format!("UITestPlayer{}", i + 1);
        let player = TestPlayer::new(player_name, join_code.clone(), host).await?;
        players.push(player);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    println!("All players joined successfully");

    let mut player_handles = Vec::new();
    for mut player in players {
        let metrics = Arc::clone(&metrics);
        let handle = tokio::spawn(async move {
            metrics.active_games.fetch_add(1, Ordering::SeqCst);
            if let Err(e) = player.handle_messages().await {
                eprintln!("Player error: {}", e);
                metrics.errors.fetch_add(1, Ordering::SeqCst);
            }
            metrics.active_games.fetch_sub(1, Ordering::SeqCst);
            metrics.completed_games.fetch_add(1, Ordering::SeqCst);
        });
        player_handles.push(handle);
    }
    let metrics_clone = Arc::clone(&metrics);
    let metrics_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            println!(
                "Active players: {}, Completed: {}, Errors: {}",
                metrics_clone.active_games.load(Ordering::SeqCst),
                metrics_clone.completed_games.load(Ordering::SeqCst),
                metrics_clone.errors.load(Ordering::SeqCst)
            );
        }
    });
    for handle in player_handles {
        handle.await.map_err(|e| TestError::Other(e.to_string()))?;
    }
    metrics_handle.abort();
    println!("UI test completed successfully");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), TestError> {
    let args = Args::parse();
    let host = args.host.clone();

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
                &host,
            )
            .await
        }
        TestMode::GameplayTest => {
            println!("Starting gameplay test with:");
            println!("Batch size: {}", args.num_lobbies);
            println!("Players per game: {}", args.players_per_lobby);
            println!("Rounds per game: {}", args.duration_or_rounds);
            run_game_batch(
                args.num_lobbies,
                args.players_per_lobby,
                args.duration_or_rounds as usize,
                &host,
            )
            .await
        }
        TestMode::UiTest => {
            let join_code = args.join_code.ok_or_else(|| {
                TestError::Other("Join code is required for UI test mode".to_string())
            })?;
            run_ui_test(
                join_code,
                args.players_per_lobby,
                args.duration_or_rounds as usize,
                &host,
            )
            .await
        }
    }
}
