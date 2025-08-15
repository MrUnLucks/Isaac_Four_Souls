use crate::ConnectionManager;
use crate::{GameLoopRegistry, RoomManager};

pub struct LobbyState {
    pub room_manager: RoomManager,
    pub connection_manager: ConnectionManager,
    pub game_loop_registry: GameLoopRegistry,
}

impl LobbyState {
    pub fn new() -> Self {
        Self {
            room_manager: RoomManager::new(),
            connection_manager: ConnectionManager::new(),
            game_loop_registry: GameLoopRegistry::new(),
        }
    }
}
