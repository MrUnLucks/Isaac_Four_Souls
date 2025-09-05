use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    game::{board::Player, cards_types::LootCard, game_state::TurnPhases},
    AppError,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ClientMessageCategory {
    LobbyMessage,
    GameMessage,
}

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
    PriorityPass,
}

impl ClientMessage {
    pub fn category(&self) -> ClientMessageCategory {
        match self {
            ClientMessage::Ping
            | ClientMessage::Chat { .. }
            | ClientMessage::CreateRoom { .. }
            | ClientMessage::DestroyRoom { .. }
            | ClientMessage::JoinRoom { .. }
            | ClientMessage::LeaveRoom
            | ClientMessage::PlayerReady => ClientMessageCategory::LobbyMessage,

            ClientMessage::TurnPass | ClientMessage::PriorityPass => {
                ClientMessageCategory::GameMessage
            }
        }
    }
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
    LobbyStartedGame {
        room_id: String,
    },
    //Broadcast on room enter
    RoomGameStart {
        turn_order: Vec<String>,
    },
    //Broadcast for all players
    TurnPhaseChange {
        player_id: String,
        phase: TurnPhases,
    },
    PublicBoardState {
        loot_deck_size: usize,
        loot_discard: Vec<LootCard>,
        current_phase: TurnPhases,
        active_player: String,
        players: HashMap<String, Player>,
    },
    PrivateBoardState {
        hand: Vec<LootCard>, // Only this player's hand
    },
    GameEnded {
        winner_id: String,
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
