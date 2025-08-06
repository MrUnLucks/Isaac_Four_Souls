use crate::{network::connection_manager::ConnectionManager, RoomManager};

pub struct LobbyState {
    pub room_manager: RoomManager,
    pub connection_manager: ConnectionManager,
}

impl LobbyState {
    pub fn new() -> Self {
        Self {
            room_manager: RoomManager::new(),
            connection_manager: ConnectionManager::new(),
        }
    }
}
