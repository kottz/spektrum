use crate::question::{load_questions_from_csv, GameQuestion};
use crate::server::{create_lobby_handler, ws_handler, AppState};
use axum::{
    routing::{get, post},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use config::Config;
use http::HeaderValue;
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::signal;
use tokio::sync::oneshot;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod game;
mod game_manager;
mod messages;
mod question;
mod server;

#[derive(Debug, Deserialize)]
struct ServerConfig {
    port: u16,
    https: bool,
}

#[derive(Default, Debug, Deserialize)]
struct LoggingConfig {
    json: bool,
}

#[derive(Debug, Deserialize)]
struct QuestionConfig {
    color_questions_csv: String,
    character_questions_csv: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SpotifyConfig {
    no_spotify: bool,
}

#[derive(Debug, Deserialize)]
struct AppConfig {
    server: ServerConfig,
    questions: QuestionConfig,
    spotify: SpotifyConfig,
    logging: LoggingConfig,
    cors_origin: String,
    https_cert_path: String,
    https_key_path: String,
}

fn init_tracing(json_logging: bool) {
    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());

    let registry = tracing_subscriber::registry().with(env_filter);

    if json_logging {
        registry
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        registry.with(tracing_subscriber::fmt::layer()).init();
    }
}

fn load_all_questions(
    config: &QuestionConfig,
) -> Result<Vec<GameQuestion>, Box<dyn std::error::Error>> {
    let mut questions = load_questions_from_csv(&config.color_questions_csv)
        .map_err(|e| format!("Failed to load color questions: {}", e))?;

    if let Some(character_path) = &config.character_questions_csv {
        let character_questions = load_questions_from_csv(character_path)
            .map_err(|e| format!("Failed to load character questions: {}", e))?;
        questions.extend(character_questions);
    }

    Ok(questions)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C signal"),
        _ = terminate => info!("Received terminate signal"),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Config::builder()
        .add_source(config::Environment::default())
        .add_source(config::File::with_name("config").required(false))
        .build()
        .map_err(|e| format!("Failed to build config: {}", e))?;

    let app_config: AppConfig = settings
        .try_deserialize()
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    init_tracing(app_config.logging.json);

    let cors_origin: HeaderValue = app_config
        .cors_origin
        .parse()
        .map_err(|e| format!("Invalid CORS origin: {}", e))?;

    let cors = CorsLayer::new()
        .allow_methods(vec![http::Method::GET, http::Method::POST])
        .allow_origin(cors_origin)
        .allow_credentials(true)
        .allow_headers(vec![
            http::header::CONTENT_TYPE,
            http::header::AUTHORIZATION,
            http::header::ACCEPT,
        ]);

    let questions = load_all_questions(&app_config.questions)?;
    info!("Loaded {} questions successfully", questions.len());

    let state = AppState::new(questions);
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/api/lobbies", post(create_lobby_handler))
        // .fallback_service(
        //     ServeDir::new(&app_config.static_dir).append_index_html_on_directories(true),
        // )
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], app_config.server.port));
    info!("Starting server on {}", addr);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let shutdown_signal_task = tokio::spawn(async move {
        shutdown_signal().await;
        let _ = shutdown_tx.send(());
    });

    // Run server with or without TLS
    let server_result = if app_config.server.https {
        let tls_config = RustlsConfig::from_pem_file(
            PathBuf::from(&app_config.https_cert_path),
            PathBuf::from(&app_config.https_key_path),
        )
        .await
        .map_err(|e| format!("TLS config error: {}", e))?;

        let server = axum_server::bind_rustls(addr, tls_config).serve(app.into_make_service());

        tokio::select! {
            result = server => result.map_err(|e| format!("Server error: {}", e).into()),
            _ = shutdown_rx => {
                info!("Shutdown signal received, starting graceful shutdown");
                Ok(())
            }
        }
    } else {
        let server = axum_server::bind(addr).serve(app.into_make_service());

        tokio::select! {
            result = server => result.map_err(|e| format!("Server error: {}", e).into()),
            _ = shutdown_rx => {
                info!("Shutdown signal received, starting graceful shutdown");
                Ok(())
            }
        }
    };

    if let Err(e) = server_result {
        error!("Server error: {}", e);
        return Err(e);
    }

    // Wait for shutdown signal handler to complete
    let _ = shutdown_signal_task.await;
    info!("Server shutdown complete");
    Ok(())
}
