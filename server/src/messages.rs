use crate::game::{ColorDef, GamePhase, ResponsePayload};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    JoinLobby {
        lobby_id: Uuid,
        admin_id: Option<Uuid>,
        name: String,
    },
    Leave {
        lobby_id: Uuid,
    },
    Answer {
        lobby_id: Uuid,
        color: String,
    },
    AdminAction {
        lobby_id: Uuid,
        action: AdminAction,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum AdminAction {
    StartGame,
    StartRound { colors: Option<Vec<String>> },
    EndRound,
    EndGame { reason: String },
    CloseGame { reason: String },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    JoinedLobby {
        player_id: Uuid,
        name: String,
        round_duration: u64,
        players: Vec<(String, i32)>,
    },
    PlayerLeft {
        name: String,
    },
    PlayerAnswered {
        name: String,
        correct: bool,
        new_score: i32,
    },
    GameOver {
        scores: Vec<(String, i32)>,
        reason: String,
    },
    GameClosed {
        reason: String,
    },
    Error {
        message: String,
    },
    // Variants unique to second snippet
    InitialPlayerList {
        players: Vec<(String, i32)>,
    },
    PlayerJoined {
        player_name: String,
        current_score: i32,
    },
    StateChanged {
        phase: String,
        colors: Vec<ColorDef>,
        scoreboard: Vec<(String, i32)>,
    },
    AdminInfo {
        current_song_name: String,
        current_song_artist: String,
    }
}

/// Convert a generic `ResponsePayload` from the game logic into a `ServerMessage`.
pub fn convert_to_server_message(payload: &ResponsePayload) -> ServerMessage {
    match payload {
        ResponsePayload::Joined {
            player_id,
            name,
            round_duration,
            current_players,
        } => ServerMessage::JoinedLobby {
            player_id: *player_id,
            name: name.clone(),
            round_duration: *round_duration,
            players: current_players.clone(),
        },
        ResponsePayload::PlayerLeft { name } => ServerMessage::PlayerLeft { name: name.clone() },
        ResponsePayload::PlayerAnswered {
            name,
            correct,
            new_score,
        } => ServerMessage::PlayerAnswered {
            name: name.clone(),
            correct: *correct,
            new_score: *new_score,
        },
        ResponsePayload::StateChanged {
            phase,
            colors,
            scoreboard,
        } => {
            let phase_str = match phase {
                GamePhase::Lobby => "lobby",
                GamePhase::Score => "score",
                GamePhase::Question => "question",
                GamePhase::GameOver => "gameover",
            };
            ServerMessage::StateChanged {
                phase: phase_str.to_string(),
                colors: colors.clone(),
                scoreboard: scoreboard.clone(),
            }
        }
        ResponsePayload::GameOver {
            final_scores,
            reason,
        } => ServerMessage::GameOver {
            scores: final_scores.clone(),
            reason: reason.clone(),
        },
        ResponsePayload::GameClosed { reason } => ServerMessage::GameClosed {
            reason: reason.clone(),
        },
        ResponsePayload::AdminInfo { current_song } => ServerMessage::AdminInfo {
            current_song_name: current_song.song_name.clone(),
            current_song_artist: current_song.artist.clone(),
        },
        ResponsePayload::Error { code, message } => ServerMessage::Error {
            message: format!("{:?}: {}", code, message),
        },
    }
}
