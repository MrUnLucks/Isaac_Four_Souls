use std::collections::HashSet;

use serde::{Deserialize, Serialize};

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
    RoomCreated {
        room_id: String,
    },
    FirstPlayerRoomCreated {
        room_id: String,
        player_id: String,
    },
    RoomDestroyed,
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
        message: String,
        code: u16,
    },
}

pub fn deserialize_message(json: &str) -> Result<ClientMessage, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn serialize_response(response: &ServerResponse) -> Result<String, serde_json::Error> {
    serde_json::to_string(response)
}
