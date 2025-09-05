use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::game::game_coordinator::{GameCoordinator, GameEvent};
use crate::network::messages::{serialize_response, ServerResponse};
use crate::{AppError, AppResult, ConnectionCommand, TurnOrder};

#[derive(Debug, Clone)]
pub enum GameMessage {
    TurnPass { connection_id: String },
    // PriorityPass { connection_id: String },
}

pub struct GameActor {
    game_id: String,
    coordinator: GameCoordinator,
    connection_to_player_mapping: HashMap<String, String>, // connection_id -> player_id
    player_to_connection_mapping: HashMap<String, String>, // player_id -> connection_id
    cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
}

impl GameActor {
    pub fn new(
        game_id: String,
        players_id_to_connection_id: HashMap<String, String>,
        turn_order: TurnOrder,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Self {
        // Reverse the mapping for quick lookup
        let connection_to_player_mapping: HashMap<String, String> = players_id_to_connection_id
            .iter()
            .map(|(player_id, conn_id)| (conn_id.clone(), player_id.clone()))
            .collect();

        let player_to_connection_mapping = players_id_to_connection_id.clone();

        let coordinator =
            GameCoordinator::new(players_id_to_connection_id, turn_order, cmd_sender.clone());

        Self {
            game_id,
            coordinator,
            connection_to_player_mapping,
            player_to_connection_mapping,
            cmd_sender,
        }
    }

    pub async fn run(&mut self, mut receiver: mpsc::UnboundedReceiver<GameMessage>) {
        println!("🎮 Game actor started for game {}", self.game_id);

        self.coordinator.initialize_game().await;

        // Main message loop
        while self.coordinator.is_running() {
            tokio::select! {
                // Handle incoming game messages
                message = receiver.recv() => {
                    match message {
                        Some(game_message) => {
                            if let Err(error) = self.handle_message(game_message.clone()).await {
                                eprintln!("Game actor error in {}: {:?}", self.game_id, error);
                                // TODO: Need more friendly syntax
                                let connection_id = match &game_message {
                                    GameMessage::TurnPass { connection_id } => connection_id,
                                    // GameMessage::PriorityPass { connection_id } => connection_id,
                                };
                                let _ = self.cmd_sender.send(ConnectionCommand::SendToPlayer {
                                    connection_id: connection_id.to_string(),
                                    message: serialize_response(ServerResponse::from_app_error(&error)),
                                });
                            }
                        }
                        None => {
                            println!("🎮 Game actor {} receiver closed", self.game_id);
                            break;
                        }
                    }
                }

                // Future enhancements:
                // - Game tick timer
            }
        }

        println!("🎮 Game actor ended for game {}", self.game_id);
    }

    async fn handle_message(&mut self, message: GameMessage) -> AppResult<()> {
        println!("🎮 Game {} handling message: {:?}", self.game_id, message);
        println!(
            "🎮 Connection->Player mapping: {:?}",
            self.connection_to_player_mapping
        );

        let game_event = match message {
            GameMessage::TurnPass { connection_id } => {
                let player_id = self
                    .connection_to_player_mapping
                    .get(&connection_id)
                    .ok_or_else(|| AppError::ConnectionNotInRoom)?
                    .clone();
                GameEvent::TurnPass { player_id }
            } // GameMessage::PriorityPass { connection_id } => {
              //     let player_id = self
              //         .connection_to_player_mapping
              //         .get(&connection_id)
              //         .ok_or_else(|| AppError::ConnectionNotInRoom)?
              //         .clone();
              //     GameEvent::PriorityPass { player_id }
              // }
        };

        self.coordinator.handle_event(game_event).await?;
        Ok(())
    }

    pub fn get_player_id_from_connection(&self, connection_id: &str) -> Option<String> {
        self.connection_to_player_mapping
            .get(connection_id)
            .cloned()
    }

    pub fn get_connection_from_player(&self, player_id: &str) -> Option<String> {
        self.player_to_connection_mapping.get(player_id).cloned()
    }

    pub fn get_all_connections(&self) -> Vec<String> {
        self.connection_to_player_mapping.keys().cloned().collect()
    }
}
