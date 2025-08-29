use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::actors::game_actor::{GameActor, GameMessage};
use crate::actors::lobby_actor::LobbyMessage;
use crate::{AppError, AppResult, ConnectionCommand, TurnOrder};

pub struct ActorRegistry {
    lobby_sender: mpsc::UnboundedSender<LobbyMessage>,
    game_actors: DashMap<String, mpsc::UnboundedSender<GameMessage>>, // game_id -> sender
    connection_to_game_mapping: DashMap<String, String>,
}

impl ActorRegistry {
    pub fn new(lobby_sender: mpsc::UnboundedSender<LobbyMessage>) -> Self {
        Self {
            lobby_sender,
            game_actors: DashMap::new(),
            connection_to_game_mapping: DashMap::new(),
        }
    }

    pub fn send_lobby_message(&self, message: LobbyMessage) -> AppResult<()> {
        self.lobby_sender
            .send(message)
            .map_err(|_| AppError::Internal {
                message: "Failed to send message to lobby actor".to_string(),
            })
    }

    pub fn start_game_actor(
        &self,
        game_id: String,
        players_id_to_connection_id: HashMap<String, String>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) -> AppResult<TurnOrder> {
        let turn_order = TurnOrder::new(players_id_to_connection_id.keys().cloned().collect());

        let (game_sender, game_receiver) = mpsc::unbounded_channel::<GameMessage>();

        // Store connection -> game mapping
        for (_, connection_id) in &players_id_to_connection_id {
            self.connection_to_game_mapping
                .insert(connection_id.clone(), game_id.clone());
        }

        let mut game_actor = GameActor::new(
            game_id.clone(),
            players_id_to_connection_id,
            turn_order.clone(),
        );

        // Store the sender for routing messages
        self.game_actors.insert(game_id.clone(), game_sender);

        // Spawn the game actor task
        tokio::spawn(async move {
            game_actor.run(game_receiver, cmd_sender).await;
        });

        Ok(turn_order)
    }

    pub fn send_game_message(&self, connection_id: &str, message: GameMessage) -> AppResult<()> {
        let game_id = self
            .connection_to_game_mapping
            .get(connection_id)
            .ok_or(AppError::ConnectionNotInRoom)?
            .clone();

        let game_sender = self
            .game_actors
            .get(&game_id)
            .ok_or(AppError::GameMessageLoopNotFound { room_id: game_id })?;

        game_sender
            .send(message)
            .map_err(|_| AppError::GameEventSendFailed {
                reason: "Game actor receiver closed".to_string(),
            })
    }

    pub fn cleanup_game_actor(&self, game_id: &str) {
        println!("🛑 Cleaning up game actor: {}", game_id);

        // Remove game actor sender
        if let Some((_, sender)) = self.game_actors.remove(game_id) {
            drop(sender); // This will close the channel and stop the actor
        }

        // Remove connection mappings for this game
        self.connection_to_game_mapping
            .retain(|_, mapped_game_id| mapped_game_id != game_id);
    }

    // Remove player connection mapping
    pub fn remove_player_connection(&self, connection_id: &str) -> Option<String> {
        self.connection_to_game_mapping
            .remove(connection_id)
            .map(|(_, game_id)| game_id)
    }
}
