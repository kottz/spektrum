use axum::{
    routing::{get, post},
    Router,
};
use axum_server::Server;
use clap::Parser;
use csv::{Error as CsvError, ReaderBuilder, StringRecord};
use std::net::SocketAddr;
use thiserror::Error;
use tower_http::services::ServeDir;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod game;
mod game_manager;
mod server;

use crate::game::Song;
use crate::server::{create_lobby_handler, list_lobbies_handler, ws_handler, AppState};

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long)]
    pub songs_csv: String,
    #[arg(long, default_value_t = 8765)]
    pub port: u16,
    #[arg(long, default_value_t = false)]
    pub no_spotify: bool,
}

#[derive(Debug, Error)]
pub enum SongFileError {
    #[error("CSV error: {0}")]
    CsvError(#[from] CsvError),
    #[error("Invalid CSV record: {0}")]
    InvalidRecord(String),
    #[error("Failed to parse song ID: {0}")]
    ParseSongIdError(#[from] std::num::ParseIntError),
    #[error("Invalid number of fields in the record. Expected 5, found {0}")]
    InvalidFieldCount(usize),
}

pub fn load_songs_from_csv(filepath: &str) -> Result<Vec<Song>, SongFileError> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(false)
        .from_path(filepath)?;

    let mut songs = Vec::new();
    for result in rdr.records() {
        let record = result?;
        if record.len() != 5 {
            return Err(SongFileError::InvalidFieldCount(record.len()));
        }
        let song = parse_song_record(&record)?;
        songs.push(song);
    }
    Ok(songs)
}

fn parse_song_record(record: &StringRecord) -> Result<Song, SongFileError> {
    let id: u32 = record[0].parse()?;
    let song_name = record[1].trim().to_string();
    let artist = record[2].trim().to_string();
    let uri = record[3].trim().to_string();
    let colors_str = record[4].trim().to_string();

    let color_list: Vec<String> = colors_str
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(Song {
        id,
        song_name,
        artist,
        uri,
        colors: color_list,
    })
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

    let songs = match load_songs_from_csv(&args.songs_csv) {
        Ok(songs) => songs,
        Err(e) => {
            error!("Failed to load songs from CSV: {}. Exiting.", e);
            std::process::exit(1);
        }
    };
    let state = AppState::new(songs);

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route(
            "/api/lobbies",
            get(list_lobbies_handler).post(create_lobby_handler),
        )
        .fallback_service(ServeDir::new("../web").append_index_html_on_directories(true))
        .with_state(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], 8765));
    info!("Starting server on http://{}", addr);

    if let Err(e) = Server::bind(addr).serve(app.into_make_service()).await {
        error!("Failed to start server: {}. Exiting.", e);
        std::process::exit(1);
    }
}
