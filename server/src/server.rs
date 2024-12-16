use axum::extract::{
    ws::{Message, WebSocket, WebSocketUpgrade},
    State,
};
use futures_util::{SinkExt, StreamExt};
use tokio::{
    io::{self, AsyncBufReadExt, BufReader},
    sync::mpsc,
};
use tracing::{debug, info, warn};

use crate::{
    game::GameLobby,
    models::{
        ClientMessage, ColorResult, GameStateMsg, LeaderboardEntry, PlayerAnsweredMsg,
        UpdateAnswerCount,
    },
    AppState,
};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: AppState) {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let mut player_name: Option<String> = None;

    loop {
        tokio::select! {
            Some(Ok(Message::Text(payload))) = socket.recv() => {
                if let Ok(msg) = serde_json::from_str::<ClientMessage>(&payload) {
                    match msg.action.as_str() {
                        "join" => {
                            if let Some(name) = msg.name {
                                {
                                    let mut lobby = state.lobby.lock().unwrap();
                                    lobby.add_player(&name, tx.clone());
                                    info!("Player {} joined the lobby", name);
                                    player_name = Some(name.clone());
                                }
                                send_game_state(&state, &name).await;
                                broadcast_answer_count(&state).await;
                            }
                        }
                        "select_color" => {
                            if let Some(name) = &player_name {
                                if let Some(color_name) = &msg.color {
                                    let (correct, new_score, total_score, all_answered) = {
                                        let mut lobby = state.lobby.lock().unwrap();
                                        if lobby.state == "question" {
                                            let (correct, new_score) = lobby.check_answer(name, color_name);
                                            let total_score = lobby.players[name].score;
                                            let all_answered = lobby.all_players_answered();
                                            (correct, new_score, total_score, all_answered)
                                        } else {
                                            (false, 0, lobby.players[name].score, false)
                                        }
                                    };

                                    let response = ColorResult {
                                        action: "color_result".to_string(),
                                        correct,
                                        score: new_score,
                                        total_score,
                                    };
                                    let json_msg = serde_json::to_string(&response).unwrap();
                                    tx.send(json_msg).ok();

                                    broadcast_player_answered(&state, name, correct).await;
                                    broadcast_answer_count(&state).await;

                                    if all_answered {
                                        info!("All players answered.");
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            },
            Some(server_msg) = rx.recv() => {
                let _ = socket.send(Message::Text(server_msg)).await;
            },
            else => {
                if let Some(name) = player_name.clone() {
                    let mut lobby = state.lobby.lock().unwrap();
                    lobby.remove_player(&name);
                    info!("Player {} left the lobby", name);
                }
                broadcast_game_state(&state).await;
                broadcast_answer_count(&state).await;
                break;
            }
        }
    }
}

pub async fn admin_input_loop(state: AppState) {
    let mut lines = BufReader::new(io::stdin()).lines();
    while let Ok(line_opt) = lines.next_line().await {
        if let Some(line) = line_opt {
            let trimmed = line.trim().to_string();
            info!("Admin input: {}", trimmed);

            if trimmed.to_lowercase().starts_with("toggle") {
                let mut specified_colors: Option<Vec<String>> = None;
                if let Some((_cmd, rest)) = trimmed.split_once(' ') {
                    let color_vec: Vec<String> =
                        rest.split(',').map(|s| s.trim().to_string()).collect();
                    if !color_vec.is_empty() {
                        specified_colors = Some(color_vec);
                    }
                }

                let (new_state, current_song_uri) = {
                    let mut lobby = state.lobby.lock().unwrap();
                    let new_state = lobby.toggle_state(specified_colors.clone());
                    info!("Game state changed to: {}", new_state);
                    if new_state == "question" {
                        debug!("Correct color(s): {}", lobby.correct_colors.join(", "));
                        debug!(
                            "All colors this round: {}",
                            lobby
                                .round_colors
                                .iter()
                                .map(|c| c.name.clone())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                        if let Some(song) = &lobby.current_song {
                            debug!(
                                "Selected track: {} by {} - {}",
                                song.song_name, song.artist, song.uri
                            );
                        }
                    }
                    let song_uri = lobby.current_song.as_ref().map(|s| s.uri.clone());
                    (new_state, song_uri)
                };

                if new_state == "question" {
                    if let Some(spotify) = &state.spotify {
                        if let Some(uri) = current_song_uri {
                            let mut ctrl = spotify.lock().unwrap().clone();
                            let success = ctrl.play_track(&uri).await;
                            if !success {
                                warn!("Could not start playback. Check Spotify setup.");
                            }
                        }
                    }
                } else if new_state == "score" {
                    if let Some(spotify) = &state.spotify {
                        let mut ctrl = spotify.lock().unwrap().clone();
                        let _ = ctrl.pause().await;
                    }
                }

                broadcast_game_state(&state).await;
                broadcast_answer_count(&state).await;
            }
        } else {
            break; // EOF
        }
    }
}

fn calc_time_left(lobby: &GameLobby) -> u64 {
    if let Some(start) = lobby.round_start_time {
        let elapsed_ms = start.elapsed().as_millis() as u64;
        let total_ms = lobby.round_duration * 1000;
        if elapsed_ms >= total_ms {
            0
        } else {
            total_ms - elapsed_ms
        }
    } else {
        0
    }
}

async fn send_game_state(state: &AppState, player_name: &str) {
    let (msg, tx) = {
        let lobby = state.lobby.lock().unwrap();
        let player = match lobby.players.get(player_name) {
            Some(p) => p,
            None => return,
        };

        let (answered_count, total) = lobby.get_answer_count();

        let lb = if lobby.state == "score" {
            Some(
                lobby
                    .players
                    .values()
                    .map(|p| LeaderboardEntry {
                        name: p.name.clone(),
                        score: p.score,
                    })
                    .collect(),
            )
        } else {
            None
        };

        let round_time_left = if lobby.state == "question" {
            Some(calc_time_left(&lobby))
        } else {
            None
        };

        let gm = GameStateMsg {
            action: "game_state".to_string(),
            state: lobby.state.clone(),
            score: player.score,
            colors: if lobby.state == "question" {
                Some(lobby.round_colors.clone())
            } else {
                None
            },
            leaderboard: lb,
            round_time_left,
            has_answered: player.has_answered,
            answer: player.answer.clone(),
            answered_count,
            total_players: total,
        };
        let json_msg = serde_json::to_string(&gm).unwrap();
        (json_msg, player.tx.clone())
    };
    let _ = tx.send(msg);
}

async fn broadcast_game_state(state: &AppState) {
    let names: Vec<String> = {
        let lobby = state.lobby.lock().unwrap();
        lobby.players.keys().cloned().collect()
    };
    for name in names {
        send_game_state(state, &name).await;
    }
}

async fn broadcast_answer_count(state: &AppState) {
    let (answered, total) = {
        let lobby = state.lobby.lock().unwrap();
        lobby.get_answer_count()
    };
    let msg = UpdateAnswerCount {
        action: "update_answer_count".into(),
        answered_count: answered,
        total_players: total,
    };
    let json_msg = serde_json::to_string(&msg).unwrap();

    let lobby = state.lobby.lock().unwrap();
    for p in lobby.players.values() {
        let _ = p.tx.send(json_msg.clone());
    }
}

async fn broadcast_player_answered(state: &AppState, player_name: &str, is_correct: bool) {
    let msg = PlayerAnsweredMsg {
        action: "player_answered".to_string(),
        player_name: player_name.to_string(),
        correct: is_correct,
    };
    let json_msg = serde_json::to_string(&msg).unwrap();

    let lobby = state.lobby.lock().unwrap();
    for p in lobby.players.values() {
        let _ = p.tx.send(json_msg.clone());
    }
}
