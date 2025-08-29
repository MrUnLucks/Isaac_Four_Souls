use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::actors::connection_actor::ConnectionMessage;
use crate::actors::game_actor::{GameActor, GameMessage};
use crate::actors::lobby_actor::LobbyMessage;
use crate::{AppError, AppResult, ConnectionCommand, TurnOrder};

pub struct ActorRegistry {
    lobby_sender: mpsc::UnboundedSender<LobbyMessage>,
    game_actors: DashMap<String, mpsc::UnboundedSender<GameMessage>>, // game_id -> sender
    connection_actors: DashMap<String, mpsc::UnboundedSender<ConnectionMessage>>, // connection_id -> sender
    connection_to_game_mapping: DashMap<String, String>,
}

impl ActorRegistry {
    pub fn new(lobby_sender: mpsc::UnboundedSender<LobbyMessage>) -> Self {
        Self {
            lobby_sender,
            game_actors: DashMap::new(),
            connection_to_game_mapping: DashMap::new(),
            connection_actors: DashMap::new(),
        }
    }

    pub fn send_lobby_message(&self, message: LobbyMessage) -> AppResult<()> {
        self.lobby_sender
            .send(message)
            .map_err(|_| AppError::Internal {
                message: "Failed to send message to lobby actor".to_string(),
            })
    }

    pub fn register_connection_actor(
        &self,
        connection_id: String,
        sender: mpsc::UnboundedSender<ConnectionMessage>,
    ) {
        self.connection_actors.insert(connection_id, sender);
    }

    pub fn send_to_connection_actor(
        &self,
        connection_id: &str,
        message: ConnectionMessage,
    ) -> AppResult<()> {
        let sender =
            self.connection_actors
                .get(connection_id)
                .ok_or(AppError::ConnectionNotFound {
                    connection_id: connection_id.to_string(),
                })?;

        sender
            .send(message)
            .map_err(|_| AppError::MessageSendFailed {
                connection_id: connection_id.to_string(),
            })
    }

    pub fn disconnect_connection_actor(&self, connection_id: &str) -> AppResult<()> {
        self.send_to_connection_actor(connection_id, ConnectionMessage::Disconnect)
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
        for (player_id, connection_id) in &players_id_to_connection_id {
            self.connection_to_game_mapping
                .insert(connection_id.clone(), game_id.clone());

            // NEW: Notify connection actors about game state transition
            if let Some(conn_sender) = self.connection_actors.get(connection_id) {
                // We can't directly call methods on the connection actor, but we could
                // send a state transition message if we had that message type
                // For now, we'll handle this through the connection actor's internal logic
            }
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

    pub fn notify_connection_game_start(
        &self,
        connection_id: &str,
        game_id: String,
        player_id: String,
    ) -> AppResult<()> {
        use crate::actors::connection_actor::ConnectionMessage;

        let message = ConnectionMessage::TransitionToGame { game_id, player_id };
        self.send_to_connection_actor(connection_id, message)
    }

    // NEW: Notify connection actor of return to lobby
    pub fn notify_connection_lobby_return(&self, connection_id: &str) -> AppResult<()> {
        use crate::actors::connection_actor::ConnectionMessage;

        let message = ConnectionMessage::TransitionToLobby;
        self.send_to_connection_actor(connection_id, message)
    }

    // ENHANCED: Better game message routing with debug info
    pub fn send_game_message(&self, connection_id: &str, message: GameMessage) -> AppResult<()> {
        // DEBUG: Check if connection has game mapping
        let game_id = self
            .connection_to_game_mapping
            .get(connection_id)
            .ok_or_else(|| AppError::ConnectionNotInRoom)?
            .clone();

        println!(
            "🎯 Routing game message from connection {} to game {}: {:?}",
            connection_id, game_id, message
        );

        let game_sender =
            self.game_actors
                .get(&game_id)
                .ok_or_else(|| AppError::GameMessageLoopNotFound {
                    room_id: game_id.clone(),
                })?;

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
        // Remove connection actor
        self.connection_actors.remove(connection_id);

        // Remove game mapping if exists
        self.connection_to_game_mapping
            .remove(connection_id)
            .map(|(_, game_id)| game_id)
    }
    pub fn get_connection_game(&self, connection_id: &str) -> Option<String> {
        self.connection_to_game_mapping
            .get(connection_id)
            .map(|entry| entry.value().clone())
    }

    pub fn is_connection_in_game(&self, connection_id: &str) -> bool {
        self.connection_to_game_mapping.contains_key(connection_id)
    }

    // NEW: Broadcast to multiple connection actors
    pub fn broadcast_to_connections(
        &self,
        connection_ids: &[String],
        message_fn: impl Fn(&str) -> ConnectionMessage,
    ) -> Vec<AppError> {
        let mut errors = Vec::new();

        for connection_id in connection_ids {
            let message = message_fn(connection_id);
            if let Err(error) = self.send_to_connection_actor(connection_id, message) {
                errors.push(error);
            }
        }

        errors
    }
}
