use axum::{routing::get, Router};
use axum_server::Server;
use clap::Parser;
use csv::{Error as CsvError, ReaderBuilder, StringRecord};
use std::net::SocketAddr;
use std::str::FromStr;
use thiserror::Error;
use tower_http::services::ServeDir;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod game;
mod game_manager;
mod messages;
mod server;

use crate::game::{Song, ColorDef};
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
    #[error("CSV Error: {0}")]
    CsvError(#[from] csv::Error),
    #[error("Parse Int Error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Invalid field count: Expected 5 fields, found {0}")]
    InvalidFieldCount(usize),
    #[error("Invalid difficulty: {0}")]
    InvalidDifficulty(String),
}

#[derive(Debug, PartialEq)]
pub struct QuizItem {
    pub id: u32,
    pub difficulty: Difficulty,
    pub song: String,
    pub correct_character: String,
    pub other_characters: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum Difficulty {
    Easy,
    Medium,
    Challenging,
    VeryChallenging,
    Expert,
    UltraHard,
}

impl FromStr for Difficulty {
    type Err = SongFileError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "easy" => Ok(Difficulty::Easy),
            "medium" => Ok(Difficulty::Medium),
            "challenging" => Ok(Difficulty::Challenging),
            "very challenging" => Ok(Difficulty::VeryChallenging),
            "expert" => Ok(Difficulty::Expert),
            "ultra hard" => Ok(Difficulty::UltraHard),
            _ => Err(SongFileError::InvalidDifficulty(s.to_string())),
        }
    }
}

pub fn load_songs_from_csv(filepath: &str) -> Result<Vec<Song>, SongFileError> {
    let mut rdr = ReaderBuilder::new().has_headers(true).from_path(filepath)?;

    let mut songs = Vec::new();
    for result in rdr.records() {
        let record = result?;
        if record.len() != 6 {
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
    let colors_str = record[3].trim().to_string();
    let spotify_uri = record[4].trim().to_string();
    let youtube_id = record[5].trim().to_string();

    let color_list: Vec<String> = colors_str
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(Song {
        id,
        song_name,
        artist,
        colors: color_list,
        spotify_uri,
        youtube_id,
    })
}

pub fn load_quiz_items_from_csv(filepath: &str) -> Result<Vec<QuizItem>, SongFileError> {
    let mut rdr = ReaderBuilder::new().has_headers(true).from_path(filepath)?;

    let mut quiz_items = Vec::new();
    for result in rdr.records() {
        let record = result?;
        if record.len() != 5 {
            return Err(SongFileError::InvalidFieldCount(record.len()));
        }
        let quiz_item = parse_quiz_item_record(&record)?;
        quiz_items.push(quiz_item);
    }
    Ok(quiz_items)
}

fn parse_quiz_item_record(record: &StringRecord) -> Result<QuizItem, SongFileError> {
    let id: u32 = record[0].parse()?;
    let difficulty: Difficulty = record[1].parse()?;
    let song = record[2].trim().to_string();
    let correct_character = record[3].trim().to_string();
    let other_characters_str = record[4].trim().to_string();

    let other_characters: Vec<String> = other_characters_str
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    Ok(QuizItem {
        id,
        difficulty,
        song,
        correct_character,
        other_characters,
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

    let all_colors = vec![
        ColorDef {
            name: "Red".into(),
            rgb: "#FF0000".into(),
        },
        ColorDef {
            name: "Green".into(),
            rgb: "#00FF00".into(),
        },
        ColorDef {
            name: "Blue".into(),
            rgb: "#0000FF".into(),
        },
        ColorDef {
            name: "Yellow".into(),
            rgb: "#FFFF00".into(),
        },
        ColorDef {
            name: "Purple".into(),
            rgb: "#800080".into(),
        },
        ColorDef {
            name: "Gold".into(),
            rgb: "#FFD700".into(),
        },
        ColorDef {
            name: "Silver".into(),
            rgb: "#C0C0C0".into(),
        },
        ColorDef {
            name: "Pink".into(),
            rgb: "#FFC0CB".into(),
        },
        ColorDef {
            name: "Black".into(),
            rgb: "#000000".into(),
        },
        ColorDef {
            name: "White".into(),
            rgb: "#FFFFFF".into(),
        },
        ColorDef {
            name: "Brown".into(),
            rgb: "#3D251E".into(),
        },
        ColorDef {
            name: "Orange".into(),
            rgb: "#FFA500".into(),
        },
        ColorDef {
            name: "Gray".into(),
            rgb: "#808080".into(),
        },
    ];
    let state = AppState::new(songs, all_colors);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_difficulty_enum() {
        assert_eq!(Difficulty::from_str("easy").unwrap(), Difficulty::Easy);
        assert_eq!(Difficulty::from_str("Medium").unwrap(), Difficulty::Medium);
        assert_eq!(
            Difficulty::from_str("Challenging").unwrap(),
            Difficulty::Challenging
        );
        assert_eq!(
            Difficulty::from_str("VeryChallenging").unwrap(),
            Difficulty::VeryChallenging
        );
        assert_eq!(Difficulty::from_str("Expert").unwrap(), Difficulty::Expert);
        assert_eq!(
            Difficulty::from_str("UltraHard").unwrap(),
            Difficulty::UltraHard
        );
        assert!(Difficulty::from_str("invalid").is_err());
    }
    #[test]
    fn test_parse_quiz_item() {
        let record = StringRecord::from(vec![
            "1",
            "easy",
            "Hedwig's Theme",
            "Dumbledore",
            "Gandalf;Merlin (BBC's Merlin);Doctor Strange;Saruman;Rasputin (Anastasia)",
        ]);
        let quiz_item = parse_quiz_item_record(&record).unwrap();
        assert_eq!(quiz_item.id, 1);
        assert_eq!(quiz_item.difficulty, Difficulty::Easy);
        assert_eq!(quiz_item.song, "Hedwig's Theme");
        assert_eq!(quiz_item.correct_character, "Dumbledore");
        assert_eq!(
            quiz_item.other_characters,
            vec![
                "Gandalf",
                "Merlin (BBC's Merlin)",
                "Doctor Strange",
                "Saruman",
                "Rasputin (Anastasia)",
            ]
        );
    }

    #[test]
    fn test_load_quiz_items_from_csv() {
        let csv_data = "id,difficulty,song,correct_character,other_characters\n1,easy,\"Hedwig's Theme\",Dumbledore,\"Gandalf;Merlin (BBC's Merlin);Doctor Strange;Saruman;Rasputin (Anastasia)\"\n2,medium,\"Concerning Hobbits\",Samwise Gamgee,\"Tyrion Lannister;Willow Ufgood;Griphook;Professor Flitwick;Gimli\"";

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(csv_data.as_bytes());
        let mut quiz_items = Vec::new();

        for result in rdr.records() {
            let record = result.unwrap();
            let quiz_item = parse_quiz_item_record(&record).unwrap();
            quiz_items.push(quiz_item);
        }
        assert_eq!(quiz_items.len(), 2);
        assert_eq!(quiz_items[0].id, 1);
        assert_eq!(quiz_items[0].difficulty, Difficulty::Easy);
        assert_eq!(quiz_items[0].song, "Hedwig's Theme");
        assert_eq!(quiz_items[0].correct_character, "Dumbledore");
        assert_eq!(
            quiz_items[0].other_characters,
            vec![
                "Gandalf",
                "Merlin (BBC's Merlin)",
                "Doctor Strange",
                "Saruman",
                "Rasputin (Anastasia)",
            ]
        );
        assert_eq!(quiz_items[1].id, 2);
        assert_eq!(quiz_items[1].difficulty, Difficulty::Medium);
        assert_eq!(quiz_items[1].song, "Concerning Hobbits");
        assert_eq!(quiz_items[1].correct_character, "Samwise Gamgee");
        assert_eq!(
            quiz_items[1].other_characters,
            vec![
                "Tyrion Lannister",
                "Willow Ufgood",
                "Griphook",
                "Professor Flitwick",
                "Gimli",
            ]
        );
    }
}
