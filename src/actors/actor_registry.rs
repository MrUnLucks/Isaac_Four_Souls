use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::actors::lobby_actor::LobbyMessage;
use crate::{AppError, AppResult};

pub struct ActorRegistry {
    lobby_sender: mpsc::UnboundedSender<LobbyMessage>,
    // Future: game_actors: DashMap<String, mpsc::UnboundedSender<GameMessage>>,
}

impl ActorRegistry {
    pub fn new(lobby_sender: mpsc::UnboundedSender<LobbyMessage>) -> Self {
        Self { lobby_sender }
    }

    pub fn send_lobby_message(&self, message: LobbyMessage) -> AppResult<()> {
        self.lobby_sender
            .send(message)
            .map_err(|_| AppError::Internal {
                message: "Failed to send message to lobby actor".to_string(),
            })
    }
}
