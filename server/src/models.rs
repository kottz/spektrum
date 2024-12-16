use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

#[derive(Debug, Deserialize)]
pub struct LobbyCreateRequest {
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LobbyInfo {
    pub id: Uuid,
    pub name: Option<String>,
    pub player_count: usize,
}

#[derive(Debug, Serialize)]
pub struct LobbyList {
    pub lobbies: Vec<LobbyInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
pub enum ClientMessage {
    #[serde(rename = "join")]
    Join { name: String },
    #[serde(rename = "select_color")]
    SelectColor { color: String },
    #[serde(rename = "create_lobby")]
    CreateLobby { name: Option<String> },
    #[serde(rename = "update_lobby_name")]
    UpdateLobbyName { name: String },
}

#[derive(Debug, Serialize)]
#[serde(tag = "action")]
pub enum ServerMessage {
    #[serde(rename = "game_state")]
    GameState(GameStateMsg),
    #[serde(rename = "color_result")]
    ColorResult(ColorResult),
    #[serde(rename = "player_answered")]
    PlayerAnswered(PlayerAnsweredMsg),
    #[serde(rename = "update_answer_count")]
    UpdateAnswerCount(UpdateAnswerCount),
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "lobby_created")]
    LobbyCreated(LobbyInfo),
    #[serde(rename = "lobby_updated")]
    LobbyUpdated(LobbyInfo),
}

#[derive(Debug, Serialize)]
pub struct PlayerAnsweredMsg {
    #[serde(rename = "playerName")]
    pub player_name: String,
    pub correct: bool,
}

#[derive(Debug, Serialize)]
pub struct ColorResult {
    pub correct: bool,
    pub score: i32,
    #[serde(rename = "totalScore")]
    pub total_score: i32,
}

#[derive(Debug, Serialize)]
pub struct LeaderboardEntry {
    pub name: String,
    pub score: i32,
}

#[derive(Debug, Serialize)]
pub struct GameStateMsg {
    pub state: String,
    pub score: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<Vec<ColorDef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leaderboard: Option<Vec<LeaderboardEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "roundTimeLeft")]
    pub round_time_left: Option<u64>,
    #[serde(rename = "hasAnswered")]
    pub has_answered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    #[serde(rename = "answeredCount")]
    pub answered_count: usize,
    #[serde(rename = "totalPlayers")]
    pub total_players: usize,
    pub lobby_id: Uuid,
    pub lobby_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateAnswerCount {
    #[serde(rename = "answeredCount")]
    pub answered_count: usize,
    #[serde(rename = "totalPlayers")]
    pub total_players: usize,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
pub enum AdminCommand {
    #[serde(rename = "toggle_state")]
    ToggleState {
        lobby_id: Uuid,
        colors: Option<Vec<String>>,
    },
    #[serde(rename = "delete_lobby")]
    DeleteLobby { lobby_id: Uuid },
    #[serde(rename = "list_lobbies")]
    ListLobbies,
}

#[derive(Debug, Serialize)]
#[serde(tag = "action")]
pub enum AdminResponse {
    #[serde(rename = "lobby_list")]
    LobbyList(LobbyList),
    #[serde(rename = "success")]
    Success { message: String },
    #[serde(rename = "error")]
    Error { message: String },
}

pub fn load_songs_from_csv(filepath: &str) -> Vec<Song> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(false)
        .from_path(filepath)
        .unwrap();

    let mut songs = Vec::new();
    for record in rdr.records().flatten() {
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
    songs
}
