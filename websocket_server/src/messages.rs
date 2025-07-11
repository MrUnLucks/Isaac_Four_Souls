use serde::{Deserialize, Serialize};

use crate::player::Player;
use crate::player_manager::PlayerManager;

#[derive(Debug, Serialize, Deserialize)]
pub enum GameMessage {
    Join { player_name: String },
    Leave { player_id: String },
    Chat { player_id: String, message: String },
    Ping,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum GameResponse {
    Welcome {
        player_id: String,
    },
    PlayerJoined {
        player_name: String,
    },
    PlayerLeft {
        player_name: String,
    },
    ChatMessage {
        player_name: String,
        message: String,
    },
    Pong,
    Error {
        message: String,
    },
}

fn player_not_found_error() -> GameResponse {
    GameResponse::Error {
        message: "Player not found".to_string(),
    }
}

pub fn handle_message(msg: GameMessage, manager: &mut PlayerManager) -> GameResponse {
    match msg {
        GameMessage::Chat { player_id, message } => {
            if let Some(player) = manager.get_player(&player_id) {
                GameResponse::ChatMessage {
                    player_name: player.name.clone(),
                    message, // Shorthand when field name matches variable
                }
            } else {
                player_not_found_error()
            }
        }
        GameMessage::Join { player_name } => {
            let player = Player::new(&player_name);
            let player_id = player.id.clone();
            let added_player = manager.add_player(player);
            match added_player {
                Ok(()) => GameResponse::Welcome { player_id },
                Err(err) => GameResponse::Error { message: err },
            }
        }
        GameMessage::Leave { player_id } => match manager.remove_player(&player_id) {
            Some(player) => GameResponse::PlayerLeft {
                player_name: player.name,
            },
            None => player_not_found_error(),
        },
        GameMessage::Ping => GameResponse::Pong,
    }
}
