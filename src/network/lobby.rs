use crate::game::game_loop::GameEvent;
use crate::{network::websocket::connection_manager::ConnectionManager, RoomManager};
use std::collections::HashMap;
use tokio::sync::mpsc;
pub struct LobbyState {
    pub room_manager: RoomManager,
    pub connection_manager: ConnectionManager,
    pub game_loops: HashMap<String, mpsc::Sender<GameEvent>>,
}

impl LobbyState {
    pub fn new() -> Self {
        Self {
            room_manager: RoomManager::new(),
            connection_manager: ConnectionManager::new(),
            game_loops: HashMap::new(),
        }
    }
}
