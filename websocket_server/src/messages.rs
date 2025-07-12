use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};

use crate::player::Player;
use crate::player_manager::PlayerManager;

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Join { player_name: String },
    Leave { player_id: String },
    Chat { player_id: String, message: String },
    Ping,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerResponse {
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

fn player_not_found_error() -> ServerResponse {
    ServerResponse::Error {
        message: "Player not found".to_string(),
    }
}

pub fn handle_message(msg: ServerMessage, manager: &mut PlayerManager) -> ServerResponse {
    match msg {
        ServerMessage::Chat { player_id, message } => {
            if let Some(player) = manager.get_player(&player_id) {
                ServerResponse::ChatMessage {
                    player_name: player.name.clone(),
                    message, // Shorthand when field name matches variable
                }
            } else {
                player_not_found_error()
            }
        }
        ServerMessage::Join { player_name } => {
            let player = Player::new(&player_name);
            let player_id = player.id.clone();
            let added_player = manager.add_player(player);
            match added_player {
                Ok(()) => ServerResponse::Welcome { player_id },
                Err(err) => ServerResponse::Error { message: err },
            }
        }
        ServerMessage::Leave { player_id } => match manager.remove_player(&player_id) {
            Some(player) => ServerResponse::PlayerLeft {
                player_name: player.name,
            },
            None => player_not_found_error(),
        },
        ServerMessage::Ping => ServerResponse::Pong,
    }
}

pub fn deserialize_message(json: &str) -> Result<ServerMessage, serde_json::Error> {
    from_str(json)
}

pub fn serialize_response(response: &ServerResponse) -> Result<String, serde_json::Error> {
    to_string(response)
}
