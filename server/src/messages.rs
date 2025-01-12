use crate::game::{GamePhase, ResponsePayload};
use crate::question::GameQuestion;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    JoinLobby {
        join_code: String,
        admin_id: Option<Uuid>,
        name: String,
    },
    Reconnect {
        lobby_id: Uuid,
        player_id: Uuid,
    },
    Leave {
        lobby_id: Uuid,
    },
    Answer {
        lobby_id: Uuid,
        answer: String,
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
    StartRound {
        specified_alternatives: Option<Vec<String>>,
    },
    EndRound,
    SkipQuestion,
    EndGame {
        reason: String,
    },
    CloseGame {
        reason: String,
    },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub enum AdminQuestion {
    ColorQuestion {
        song_name: String,
        artist: String,
        youtube_id: String,
    },
    CharacterQuestion {
        song: String,
        youtube_id: String,
        character_context: String,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct GameState {
    pub phase: String,
    pub question_type: String,
    pub alternatives: Vec<String>,
    pub scoreboard: Vec<(String, i32)>,
    pub current_song: Option<CurrentSong>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CurrentSong {
    pub song_name: String,
    pub artist: String,
    pub youtube_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    JoinedLobby {
        player_id: Uuid,
        lobby_id: Uuid,
        name: String,
        round_duration: u64,
        players: Vec<(String, i32)>,
    },
    ReconnectSuccess {
        game_state: GameState,
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
    StateChanged {
        phase: String,
        question_type: String,
        alternatives: Vec<String>,
        scoreboard: Vec<(String, i32)>,
    },
    AdminInfo {
        question: AdminQuestion,
    },
    AdminNextQuestions {
        upcoming_questions: Vec<GameQuestion>,
    },
}

impl From<&GamePhase> for String {
    fn from(phase: &GamePhase) -> String {
        match phase {
            GamePhase::Lobby => "lobby".to_string(),
            GamePhase::Score => "score".to_string(),
            GamePhase::Question => "question".to_string(),
            GamePhase::GameOver => "gameover".to_string(),
        }
    }
}

/// Convert a generic `ResponsePayload` from the game logic into a `ServerMessage`.
pub fn convert_to_server_message(payload: &ResponsePayload) -> ServerMessage {
    match payload {
        ResponsePayload::Joined {
            player_id,
            lobby_id,
            name,
            round_duration,
            current_players,
        } => ServerMessage::JoinedLobby {
            player_id: *player_id,
            lobby_id: *lobby_id,
            name: name.clone(),
            round_duration: *round_duration,
            players: current_players.clone(),
        },
        ResponsePayload::Reconnected { game_state } => ServerMessage::ReconnectSuccess {
            game_state: GameState {
                phase: (&game_state.phase).into(),
                question_type: game_state.question_type.clone(),
                alternatives: game_state.alternatives.clone(),
                scoreboard: game_state.scoreboard.clone(),
                current_song: game_state.current_song.as_ref().map(|song| CurrentSong {
                    song_name: song.song_name.clone(),
                    artist: song.artist.clone(),
                    youtube_id: song.youtube_id.clone(),
                }),
            },
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
            question_type,
            alternatives,
            scoreboard,
        } => ServerMessage::StateChanged {
            phase: phase.into(),
            question_type: question_type.clone(),
            alternatives: alternatives.clone(),
            scoreboard: scoreboard.clone(),
        },
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
        ResponsePayload::AdminInfo { current_question } => {
            let question = match current_question {
                GameQuestion::Color(q) => AdminQuestion::ColorQuestion {
                    song_name: q.song.clone(),
                    artist: q.artist.clone(),
                    youtube_id: q.youtube_id.clone(),
                },
                GameQuestion::Character(q) => AdminQuestion::CharacterQuestion {
                    song: q.song.clone(),
                    youtube_id: q.youtube_id.clone(),
                    character_context: "TODO".to_string(),
                },
            };
            ServerMessage::AdminInfo { question }
        }
        ResponsePayload::AdminNextQuestions { upcoming_questions } => {
            ServerMessage::AdminNextQuestions {
                upcoming_questions: upcoming_questions.clone(),
            }
        }
        ResponsePayload::Error { code, message } => ServerMessage::Error {
            message: format!("{:?}: {}", code, message),
        },
    }
}
