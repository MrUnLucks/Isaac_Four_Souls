use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::AppError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ClientMessage {
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
        player_name: String,
        room_id: String,
    },
    LeaveRoom,
    PlayerReady,
    TurnPass,
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
    RoomCreatedBroadcast {
        room_id: String,
    },
    RoomCreated {
        room_id: String,
        player_id: String,
    },
    RoomDestroyed {
        room_id: String,
    },
    SelfJoined {
        player_name: String,
        player_id: String,
    },
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
    GameStarted {
        room_id: String,
        turn_order: Vec<String>,
    },
    //Broadcast on room enter
    TurnOrder {
        turn_order: Vec<String>,
    },
    //Broadcast for all players
    TurnChange {
        next_player_id: String,
    },
    Error {
        error_type: String, // "RoomFull", "PlayerNotFound" variant_name of errror
        message: String,
        code: u16,
        // details: Option<serde_json::Value>, //Feature for clear frontend error handling(?)
    },
}

impl ServerResponse {
    pub fn from_app_error(error: &AppError) -> Self {
        ServerResponse::Error {
            error_type: error.variant_name().to_string(),
            message: error.user_friendly_message(),
            code: error.status_code(),
        }
    }
}

pub fn deserialize_message(json: &str) -> Result<ClientMessage, serde_json::Error> {
    serde_json::from_str(json)
}

// If this fails something is broken in the response code so it's correct to crash with .expect
pub fn serialize_response(response: ServerResponse) -> String {
    serde_json::to_string(&response)
        .expect("Failed to serialize response - this should never happen with valid data")
}
