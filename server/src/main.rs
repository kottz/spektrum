use crate::question::QuestionStore;
use crate::server::{
    AppState, add_no_store_headers, check_sessions_handler, create_lobby_handler,
    get_stored_data_handler, join_lobby_handler, list_sets_handler, set_stored_data_handler,
    upload_character_image_handler, ws_handler,
};
use axum::{
    Router,
    extract::Request,
    middleware::{self, Next},
    response::Response,
    routing::{any, get, post},
};
use config::Config;
use http::HeaderValue;
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::signal;
use tower_http::compression::{CompressionLayer, CompressionLevel};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(test)]
mod bindings;
mod db;
mod game;
mod question;
mod server;
mod uuid;

async fn no_store_response_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    add_no_store_headers(response.headers_mut());
    response
}

#[derive(Debug, Deserialize)]
struct ServerConfig {
    port: u16,
    cors_origins: Vec<String>,
}

#[derive(Default, Debug, Deserialize)]
struct LoggingConfig {
    text: bool,
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

/// Initialize tracing with configurable filters.
///
/// Default filter: `spektrum=info,tower_http=info` (limits dependency noise)
///
/// Example RUST_LOG filters:
/// - `RUST_LOG=spektrum=info` - default application logs
/// - `RUST_LOG=spektrum=info,lock=debug` - enable lock contention debugging
/// - `RUST_LOG=spektrum=info,ws=trace` - verbose WebSocket debugging
/// - `RUST_LOG=spektrum=info,storage=debug` - storage/S3 operation debugging
/// - `RUST_LOG=spektrum=info,maintenance=debug` - cleanup task debugging
fn init_tracing(text_logging: bool) {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "spektrum=info,tower_http=info".into());
    let registry = tracing_subscriber::registry().with(env_filter);

    if text_logging {
        registry.with(tracing_subscriber::fmt::layer()).init();
    } else {
        registry
            .with(tracing_subscriber::fmt::layer().json())
            .init();
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

fn build_app(state: AppState, cors: CorsLayer) -> Router {
    Router::new()
        .route("/ws", any(ws_handler))
        .route("/api/list-sets", get(list_sets_handler))
        .route("/api/create-lobby", post(create_lobby_handler))
        .route("/api/join-lobby", post(join_lobby_handler))
        .route("/api/check-sessions", post(check_sessions_handler))
        .route("/api/questions", post(get_stored_data_handler))
        .route("/api/update-questions", post(set_stored_data_handler))
        .route(
            "/api/upload-character-image/{character_name}",
            post(upload_character_image_handler),
        )
        .with_state(state)
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &http::Request<_>| {
                let request_id = fastrand::u64(..);
                tracing::info_span!(
                    "http_request",
                    request_id = %request_id,
                    method = %request.method(),
                    uri = %request.uri(),
                )
            }),
        )
        .layer(
            CompressionLayer::new()
                .quality(CompressionLevel::Default)
                .gzip(true),
        )
        .layer(middleware::from_fn(no_store_response_middleware))
        .layer(cors)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Config::builder()
        .add_source(
            config::Environment::with_prefix("SPEKTRUM")
                .separator("__")
                .list_separator(",")
                .with_list_parse_key("admin_password")
                .with_list_parse_key("server.cors_origins")
                .try_parsing(true),
        )
        .add_source(config::File::with_name("config").required(false))
        .build()
        .map_err(|e| format!("Failed to build config: {e}"))?;

    let app_config: AppConfig = settings
        .try_deserialize()
        .map_err(|e| format!("Failed to parse config: {e}"))?;

    init_tracing(app_config.logging.text);

    let cors_origins: Vec<HeaderValue> = app_config
        .server
        .cors_origins
        .iter()
        .map(|origin| {
            origin
                .parse()
                .map_err(|e| format!("Invalid CORS origin '{origin}': {e}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let cors = CorsLayer::new()
        .allow_methods(vec![http::Method::GET, http::Method::POST])
        .allow_origin(cors_origins)
        .allow_headers(vec![
            http::header::CONTENT_TYPE,
            http::header::AUTHORIZATION,
            http::header::ACCEPT,
        ]);

    let question_store = QuestionStore::new(&app_config.storage).await?;
    let state = AppState::new(question_store, app_config.admin_password);

    let app = build_app(state, cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], app_config.server.port));
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::{CreateLobbyRequest, create_lobby};
    use axum::body::Body;
    use futures_util::future::join_all;
    use http::{Method, Request, StatusCode, header};
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;
    use tower::ServiceExt;

    async fn test_state() -> (AppState, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("questions.json");
        let mut file = File::create(&file_path).unwrap();
        writeln!(
            file,
            r#"{{
                "media": [{{"id": 1, "title": "Test Song", "artist": "Test Artist", "release_year": null, "spotify_uri": null, "youtube_id": "test123"}}],
                "characters": [],
                "questions": [{{"id": 1, "media_id": 1, "question_type": "color", "question_text": null, "image_url": null, "is_active": true}}],
                "options": [{{"id": 1, "question_id": 1, "option_text": "Red", "is_correct": true}}],
                "sets": []
            }}"#
        )
        .unwrap();

        let storage = StorageConfig::Filesystem {
            base_path: dir.path().to_path_buf(),
            file_path: "questions.json".to_string(),
        };
        let question_store = QuestionStore::new(&storage).await.unwrap();
        (AppState::new(question_store, vec![]), dir)
    }

    #[tokio::test]
    async fn many_players_can_join_at_once() {
        let (state, _dir) = test_state().await;
        let lobby = create_lobby(
            &state,
            CreateLobbyRequest {
                round_duration: Some(60),
                set_id: None,
            },
        )
        .await
        .unwrap();
        let app = build_app(state, CorsLayer::new());

        let joins = (0..100).map(|number| {
            let body = serde_json::json!({
                "join_code": lobby.join_code,
                "name": format!("Player{number}"),
            });
            let request = Request::builder()
                .method(Method::POST)
                .uri("/api/join-lobby")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap();
            app.clone().oneshot(request)
        });

        for response in join_all(joins).await {
            assert_eq!(response.unwrap().status(), StatusCode::OK);
        }
    }
}
