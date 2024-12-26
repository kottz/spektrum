use axum::{
    routing::{get, post},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use axum_server::Server;
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use http::{HeaderValue, Method};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod game;
mod game_manager;
mod messages;
mod question;
mod server;

use crate::question::{load_questions_from_csv, GameQuestion, QuestionResult};
use crate::server::{create_lobby_handler, ws_handler, AppState};

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long)]
    pub color_questions_csv: String,
    #[arg(long)]
    pub character_questions_csv: Option<String>,
    #[arg(long, default_value_t = 8765)]
    pub port: u16,
    #[arg(long, default_value_t = false)]
    pub no_spotify: bool,
    #[arg(long, default_value_t = false)]
    pub https: bool,
}

fn load_all_questions(args: &Args) -> QuestionResult<Vec<GameQuestion>> {
    let mut questions = load_questions_from_csv(&args.color_questions_csv)?;

    if let Some(character_path) = &args.character_questions_csv {
        let character_questions = load_questions_from_csv(character_path)?;
        questions.extend(character_questions);
    }

    Ok(questions)
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

    let questions = match load_all_questions(&args) {
        Ok(questions) => questions,
        Err(e) => {
            error!("Failed to load questions from CSV: {}. Exiting.", e);
            std::process::exit(1);
        }
    };

    let state = AppState::new(questions);

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_credentials(true)
        .allow_headers([
            http::header::CONTENT_TYPE,
            http::header::AUTHORIZATION,
            http::header::ACCEPT,
        ]);

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/api/lobbies", post(create_lobby_handler))
        .fallback_service(ServeDir::new("../web").append_index_html_on_directories(true))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    info!("Starting server on http://{}", addr);

    if args.https {
        // configure certificate and private key used by https
        let config = RustlsConfig::from_pem_file(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("localhost.pem"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("localhost-key.pem"),
        )
        .await
        .unwrap();
        let mut server = axum_server::bind_rustls(addr, config);
        server.http_builder();

        server.serve(app.into_make_service()).await.unwrap();
    } else {
        if let Err(e) = Server::bind(addr).serve(app.into_make_service()).await {
            error!("Failed to start server: {}. Exiting.", e);
            std::process::exit(1);
        }
    }
}
