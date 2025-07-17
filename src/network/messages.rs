use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};

use crate::game::room_manager::RoomManager;

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Ping,
    Chat {
        message: String,
    },
    CreateRoom {
        room_name: String,
        first_player_name: String,
    },
    DestroyRoom {
        room_id: String,
    },
    JoinRoom {
        connection_id: String,
        player_name: String,
        room_id: String,
    },
    LeaveRoom {
        connection_id: String,
    },
    PlayerReady {
        player_id: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerError {
    PlayerNotFound,
    RoomNotFound,
    UnknownResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerResponse {
    Pong,
    ChatMessage {
        player_name: String,
        message: String,
    },
    RoomCreated {
        room_id: String,
        player_id: String,
    },
    RoomDestroyed,
    PlayerJoined {
        player_name: String,
        player_id: String,
    },
    PlayerLeft {
        player_name: String,
    },
    PlayersReady {
        players_ready: HashSet<String>,
    },
    GameStarted,
    Error {
        message: ServerError,
    },
}

fn player_not_found_error() -> ServerResponse {
    ServerResponse::Error {
        message: ServerError::PlayerNotFound,
    }
}

pub fn handle_message(
    msg: ServerMessage,
    room_manager: &mut RoomManager,
    connection_id: &str,
) -> ServerResponse {
    match msg {
        ServerMessage::Ping => ServerResponse::Pong,

        // TODO: helper function inside room_manager
        ServerMessage::Chat { message } => {
            let room_info = room_manager.connection_to_room_info.get(connection_id);

            if let Some(room_info) = room_info {
                let room_id = room_info.room_id.clone();
                let room_player_id = room_info.room_player_id.clone();

                if let Some(room) = room_manager.get_room_mut(&room_id) {
                    if let Some(player_name) = room.get_player(&room_player_id) {
                        return ServerResponse::ChatMessage {
                            player_name: player_name.clone(),
                            message,
                        };
                    }
                }
            }

            ServerResponse::Error {
                message: ServerError::PlayerNotFound,
            }
        }

        ServerMessage::CreateRoom {
            room_name,
            first_player_name,
        } => {
            match room_manager.create_room(room_name, connection_id.to_string(), first_player_name)
            {
                Ok((room_id, player_id)) => ServerResponse::RoomCreated { room_id, player_id }, // Return both
                Err(_) => ServerResponse::Error {
                    message: ServerError::RoomNotFound,
                },
            }
        }

        ServerMessage::DestroyRoom { room_id } => match room_manager.destroy_room(&room_id) {
            Ok(()) => ServerResponse::RoomDestroyed,
            Err(_) => ServerResponse::Error {
                message: ServerError::RoomNotFound,
            },
        },

        ServerMessage::JoinRoom {
            connection_id,
            player_name,
            room_id,
        } => match room_manager.join_room(&room_id, connection_id, player_name.clone()) {
            Ok(player_id) => ServerResponse::PlayerJoined {
                // ✅ Capture the player_id here
                player_id,
                player_name,
            },
            Err(_) => ServerResponse::Error {
                message: ServerError::RoomNotFound,
            },
        },

        ServerMessage::LeaveRoom { connection_id } => {
            match room_manager.leave_room(&connection_id) {
                Ok(player_name) => ServerResponse::PlayerLeft { player_name },
                Err(_) => ServerResponse::Error {
                    // Actually both error (room and player) are incapsulated here, need better handling in the future
                    message: ServerError::PlayerNotFound,
                },
            }
        }

        ServerMessage::PlayerReady { player_id } => match room_manager.ready_player(&player_id) {
            Err(_) => ServerResponse::Error {
                message: ServerError::RoomNotFound,
            },
            Ok(ready_result) => {
                if ready_result.game_started {
                    ServerResponse::GameStarted
                } else {
                    ServerResponse::PlayersReady {
                        players_ready: ready_result.players_ready,
                    }
                }
            }
        },
        _ => ServerResponse::Error {
            message: ServerError::UnknownResponse,
        },
    }
}

pub fn deserialize_message(json: &str) -> Result<ServerMessage, serde_json::Error> {
    from_str(json)
}

pub fn serialize_response(response: &ServerResponse) -> Result<String, serde_json::Error> {
    to_string(response)
}
