use axum::{
    routing::{get, post},
    Router,
};
use axum_server::Server;
use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod game;
mod game_manager;
mod models;
mod server;
mod spotify;

use crate::game_manager::GameLobbyManager;
use crate::server::{create_lobby_handler, list_lobbies_handler, ws_handler, ServerState};
use crate::spotify::{SpotifyController, SpotifyError};

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long)]
    pub songs_csv: String,
    #[arg(long, default_value_t = 8765)]
    pub port: u16,
    #[arg(long, default_value_t = false)]
    pub no_spotify: bool,
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
    let game_manager = Arc::new(GameLobbyManager::new());

    let spotify_controller = if !args.no_spotify {
        match SpotifyController::new().await {
            Ok(ctrl) => {
                let mut c2 = ctrl.clone();
                match c2.get_active_device().await {
                    Ok(dev) => {
                        let device_name = dev
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown Device");
                        info!("Found active Spotify device: {}", device_name);
                        Some(Arc::new(Mutex::new(ctrl)))
                    }
                    Err(SpotifyError::NoActiveDevice) => {
                        error!("No active Spotify device found at startup.");
                        std::process::exit(1);
                    }
                    Err(e) => {
                        error!("Failed to get active device: {}. Exiting.", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                error!("Spotify integration failed to initialize: {}. Exiting.", e);
                std::process::exit(1);
            }
        }
    } else {
        info!("Spotify integration disabled. Running without playback.");
        None
    };

    let songs = match models::load_songs_from_csv(&args.songs_csv) {
        Ok(songs) => Arc::new(songs),
        Err(e) => {
            error!("Failed to load songs from CSV: {}. Exiting.", e);
            std::process::exit(1);
        }
    };

    let state = Arc::new(ServerState {
        game_manager,
        spotify: spotify_controller,
        songs: songs.clone(),
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route(
            "/api/lobbies",
            get(list_lobbies_handler).post(create_lobby_handler),
        )
        .fallback_service(ServeDir::new("../web").append_index_html_on_directories(true))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    info!("Starting server on http://{}", addr);
    if let Err(e) = Server::bind(addr).serve(app.into_make_service()).await {
        error!("Failed to start server: {}. Exiting.", e);
        std::process::exit(1);
    }
}
