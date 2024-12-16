use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};


#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ColorDef {
    pub name: String,
    pub rgb: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub id: u32,
    pub song_name: String,
    pub artist: String,
    pub uri: String,
    pub colors: Vec<String>,
}

#[derive(Deserialize)]
pub struct ClientMessage {
    pub action: String,
    pub name: Option<String>,
    pub color: Option<String>,
}

#[derive(Serialize)]
pub struct PlayerAnsweredMsg {
    pub action: String,
    pub playerName: String,
    pub correct: bool,
}

#[derive(Serialize)]
pub struct ColorResult {
    pub action: String,
    pub correct: bool,
    pub score: i32,
    pub totalScore: i32,
}

#[derive(Serialize)]
pub struct LeaderboardEntry {
    pub name: String,
    pub score: i32,
}

#[derive(Serialize)]
pub struct GameStateMsg {
    pub action: String,
    pub state: String,
    pub score: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<Vec<ColorDef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leaderboard: Option<Vec<LeaderboardEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roundTimeLeft: Option<u64>,
    pub hasAnswered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    pub answeredCount: usize,
    pub totalPlayers: usize,
}

#[derive(Serialize)]
pub struct UpdateAnswerCount {
    pub action: String,
    pub answeredCount: usize,
    pub totalPlayers: usize,
}

pub fn load_songs_from_csv(filepath: &str) -> Vec<Song> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(false)
        .from_path(filepath)
        .unwrap();

    let mut songs = Vec::new();
    for result in rdr.records() {
        if let Ok(record) = result {
            if record.len() < 5 {
                continue;
            }
            let id = record[0].parse().unwrap_or(0);
            let song_name = record[1].trim().to_string();
            let artist = record[2].trim().to_string();
            let uri = record[3].trim().to_string();
            let colors_str = record[4].trim().to_string();
            let color_list: Vec<String> = colors_str
                .split(';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            songs.push(Song {
                id,
                song_name,
                artist,
                uri,
                colors: color_list,
            });
        }
    }
    songs
}
