use crate::question::QuestionStore;
use crate::server::{
    check_sessions_handler, create_lobby_handler, get_stored_data_handler, list_sets_handler,
    set_stored_data_handler, upload_character_image_handler, ws_handler, AppState,
};
use axum::{
    routing::{any, get, post},
    Router,
};
use config::Config;
use http::HeaderValue;
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::signal;
use tokio::time::Duration;
use tower_http::compression::{CompressionLayer, CompressionLevel};
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod game;
mod game_manager;
mod messages;
mod question;
mod server;

#[derive(Debug, Deserialize)]
struct ServerConfig {
    port: u16,
    cors_origins: Vec<String>,
}

#[derive(Default, Debug, Deserialize)]
struct LoggingConfig {
    json: bool,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum StorageConfig {
    #[serde(rename = "filesystem")]
    Filesystem {
        base_path: PathBuf,
        file_path: String,
    },
    #[serde(rename = "s3")]
    S3 {
        bucket: String,
        region: String,
        prefix: String,
        question_folder: String,
        question_file: String,
        access_key_id: String,
        secret_access_key: String,
    },
}

#[derive(Debug, Deserialize)]
struct AppConfig {
    server: ServerConfig,
    logging: LoggingConfig,
    storage: StorageConfig,
    admin_password: Vec<String>,
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
        .add_source(config::Environment::default().separator("__"))
        .add_source(config::File::with_name("config").required(false))
        .build()
        .map_err(|e| format!("Failed to build config: {}", e))?;
    let app_config: AppConfig = settings
        .try_deserialize()
        .map_err(|e| format!("Failed to parse config: {}", e))?;
    init_tracing(app_config.logging.json);

    // Parse all CORS origins
    let cors_origins: Vec<HeaderValue> = app_config
        .server
        .cors_origins
        .iter()
        .map(|origin| {
            origin
                .parse()
                .map_err(|e| format!("Invalid CORS origin '{}': {}", origin, e))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let cors = CorsLayer::new()
        .allow_methods(vec![http::Method::GET, http::Method::POST])
        .allow_origin(cors_origins)
        .allow_credentials(true)
        .allow_headers(vec![
            http::header::CONTENT_TYPE,
            http::header::AUTHORIZATION,
            http::header::ACCEPT,
        ]);

    let question_store = QuestionStore::new(&app_config.storage).await?;

    let state = AppState::new(question_store, app_config.admin_password);
    let app = Router::new()
        .route("/ws", any(ws_handler))
        .route("/api/list-sets", get(list_sets_handler))
        .route("/api/lobbies", post(create_lobby_handler))
        .route("/api/check-sessions", post(check_sessions_handler))
        .route("/api/questions", post(get_stored_data_handler))
        .route("/api/update-questions", post(set_stored_data_handler))
        .route(
            "/api/upload-character-image/{character_name}",
            post(upload_character_image_handler),
        )
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(
            CompressionLayer::new()
                .quality(CompressionLevel::Default)
                .gzip(true),
        )
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], app_config.server.port));
    info!("Starting server on {}", addr);

    // Create server handle for shutdown
    let handle = axum_server::Handle::new();
    let handle_clone = handle.clone();

    // Spawn shutdown signal handler
    tokio::spawn(async move {
        shutdown_signal().await;
        info!("Shutdown signal received, starting graceful shutdown");
        handle_clone.graceful_shutdown(Some(Duration::from_secs(3)));
    });

    // Create the server and configure HTTP/2
    let mut server = axum_server::bind(addr);
    server.http_builder().http2().enable_connect_protocol();

    // Run the server
    server
        .handle(handle)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}
