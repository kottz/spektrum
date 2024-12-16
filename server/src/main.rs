use axum::{routing::any, Router};
use axum_server::Server;
use clap::Parser;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tower_http::services::ServeDir;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod game; // GameLobby + Player
mod models; // Shared structs/data, CSV loader
mod server;
mod spotify; // SpotifyController // WS + admin console logic

use crate::game::GameLobby;
use crate::server::{admin_input_loop, ws_handler};
use crate::spotify::SpotifyController;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long)]
    pub lobby: String,
    #[arg(long, default_value_t = 8765)]
    pub port: u16,
    #[arg(long)]
    pub songs_csv: String,
    #[arg(long, default_value_t = false)]
    pub no_spotify: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub lobby: Arc<Mutex<GameLobby>>,
    pub spotify: Option<Arc<Mutex<SpotifyController>>>,
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

    let songs = models::load_songs_from_csv(&args.songs_csv);
    let lobby = GameLobby::new(args.lobby.clone(), songs);
    info!("Created lobby: {}", args.lobby);

    let spotify_controller = if !args.no_spotify {
        match SpotifyController::new().await {
            Some(ctrl) => {
                let mut c2 = ctrl.clone();
                match c2.get_active_device().await {
                    None => {
                        error!("No active Spotify device found at startup.");
                        std::process::exit(1);
                    }
                    Some(dev) => {
                        let device_name = dev
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown Device");
                        info!("Found active Spotify device: {}", device_name);
                    }
                }
                Some(Arc::new(Mutex::new(ctrl)))
            }
            None => {
                error!("Spotify integration failed to initialize. Exiting.");
                std::process::exit(1);
            }
        }
    } else {
        info!("Spotify integration disabled. Running without playback.");
        None
    };

    let state = AppState {
        lobby: Arc::new(Mutex::new(lobby)),
        spotify: spotify_controller,
    };

    let app = Router::new()
        .route("/ws", any(ws_handler))
        .fallback_service(ServeDir::new("./assets").append_index_html_on_directories(true))
        .with_state(state.clone());

    tokio::spawn(async move {
        admin_input_loop(state).await;
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    info!("Starting server on http://{}", addr);
    Server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
