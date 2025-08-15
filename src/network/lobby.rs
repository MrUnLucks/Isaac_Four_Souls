use crate::{
    network::websocket::connection_manager::ConnectionManager, GameLoopRegistry, RoomManager,
};

pub struct LobbyState {
    pub room_manager: RoomManager,
    pub connection_manager: ConnectionManager,
    pub game_engine: GameLoopRegistry,
}

impl LobbyState {
    pub fn new() -> Self {
        Self {
            room_manager: RoomManager::new(),
            connection_manager: ConnectionManager::new(),
            game_engine: GameLoopRegistry::new(),
        }
    }
}
