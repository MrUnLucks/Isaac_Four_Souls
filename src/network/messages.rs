use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::game::room_manager::{RoomManager, RoomManagerError};

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
        connection_id: String,
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

#[derive(Debug, Serialize)]
pub enum ServerResponse {
    ConnectionId {
        connection_id: String,
    },
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

pub fn handle_message(
    msg: ServerMessage,
    room_manager: &mut RoomManager,
    connection_id: &str,
) -> Result<ServerResponse, ServerError> {
    match msg {
        ServerMessage::Ping => Ok(ServerResponse::Pong),

        // This may need to be moved inside room_manager
        ServerMessage::Chat { message } => {
            match room_manager.connection_to_room_info.get(connection_id) {
                None => Err(ServerError::PlayerNotFound),
                Some(room_info) => Ok(ServerResponse::ChatMessage {
                    player_name: room_info.clone().player_name,
                    message: message,
                }),
            }
        }
        ServerMessage::CreateRoom {
            room_name,
            first_player_name,
        } => {
            let (room_id, player_id) = room_manager.create_room(
                room_name,
                connection_id.to_string(),
                first_player_name,
            )?;
            Ok(ServerResponse::RoomCreated { room_id, player_id })
        }

        ServerMessage::DestroyRoom {
            room_id,
            connection_id,
        } => {
            room_manager.destroy_room(&room_id, &connection_id)?;
            Ok(ServerResponse::RoomDestroyed)
        }
        ServerMessage::JoinRoom {
            connection_id,
            player_name,
            room_id,
        } => {
            let player_id = room_manager.join_room(&room_id, connection_id, player_name.clone())?;
            Ok(ServerResponse::PlayerJoined {
                player_id,
                player_name,
            })
        }
        ServerMessage::LeaveRoom { connection_id } => {
            let player_name = room_manager.leave_room(&connection_id)?;
            Ok(ServerResponse::PlayerLeft { player_name })
        }

        ServerMessage::PlayerReady { player_id } => {
            let ready_result = room_manager.ready_player(&player_id)?;
            Ok(if ready_result.game_started {
                ServerResponse::GameStarted
            } else {
                ServerResponse::PlayersReady {
                    players_ready: ready_result.players_ready,
                }
            })
        }
    }
}

pub fn deserialize_message(json: &str) -> Result<ServerMessage, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn serialize_response(response: &ServerResponse) -> Result<String, serde_json::Error> {
    serde_json::to_string(response)
}

#[derive(Debug, Serialize)]
pub enum ServerError {
    PlayerNotFound,
    RoomNotFound,
    RoomManagerError(RoomManagerError),
    UnknownResponse,
}
impl From<RoomManagerError> for ServerError {
    fn from(err: RoomManagerError) -> Self {
        ServerError::RoomManagerError(err)
    }
}
