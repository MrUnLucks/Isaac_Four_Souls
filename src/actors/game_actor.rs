use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::game::game_coordinator::GameCoordinator;
use crate::{AppError, ConnectionCommand, TurnOrder};

#[derive(Debug, Clone)]
pub enum GameMessage {
    TurnPass { player_id: String },
    PriorityPass { player_id: String },
    TurnPassFromConnection { connection_id: String },
    PriorityPassFromConnection { connection_id: String },
}

pub struct GameActor {
    game_id: String,
    coordinator: GameCoordinator,
    connection_to_player_mapping: HashMap<String, String>, // connection_id -> player_id
    player_to_connection_mapping: HashMap<String, String>, // player_id -> connection_id
}

impl GameActor {
    pub fn new(
        game_id: String,
        players_id_to_connection_id: HashMap<String, String>,
        turn_order: TurnOrder,
    ) -> Self {
        // Reverse the mapping for quick lookup
        let connection_to_player_mapping: HashMap<String, String> = players_id_to_connection_id
            .iter()
            .map(|(player_id, conn_id)| (conn_id.clone(), player_id.clone()))
            .collect();

        let player_to_connection_mapping = players_id_to_connection_id.clone();

        let coordinator = GameCoordinator::new(players_id_to_connection_id, turn_order);

        Self {
            game_id,
            coordinator,
            connection_to_player_mapping,
            player_to_connection_mapping,
        }
    }

    pub async fn run(
        &mut self,
        mut receiver: mpsc::UnboundedReceiver<GameMessage>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        println!("🎮 Game actor started for game {}", self.game_id);

        // Initialize the game
        self.coordinator.initialize_game(&cmd_sender).await;

        // Main message loop
        while self.coordinator.is_running() {
            tokio::select! {
                // Handle incoming game messages
                message = receiver.recv() => {
                    match message {
                        Some(game_message) => {
                            if let Err(error) = self.handle_message(game_message, &cmd_sender).await {
                                eprintln!("Game actor error in {}: {:?}", self.game_id, error);
                                if let Some(connection_id) = self.get_connection_for_error(&error) {
                                    self.send_error_to_connection(&connection_id, error, &cmd_sender).await;
                                }
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

    async fn handle_message(
        &mut self,
        message: GameMessage,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Result<(), AppError> {
        println!("🎮 Game {} handling message: {:?}", self.game_id, message);
        println!(
            "🎮 Connection->Player mapping: {:?}",
            self.connection_to_player_mapping
        );

        let game_event = match message {
            GameMessage::TurnPass { player_id } => {
                println!("🎮 Direct TurnPass for player: {}", player_id);
                crate::game::event_handler::GameEvent::TurnPass { player_id }
            }
            GameMessage::PriorityPass { player_id } => {
                println!("🎮 Direct PriorityPass for player: {}", player_id);
                crate::game::event_handler::GameEvent::PriorityPass { player_id }
            }
            GameMessage::TurnPassFromConnection { connection_id } => {
                let player_id = self
                    .connection_to_player_mapping
                    .get(&connection_id)
                    .ok_or_else(|| {
                        println!(
                            "🚨 Connection {} not found in game {} player mapping",
                            connection_id, self.game_id
                        );
                        AppError::ConnectionNotInRoom
                    })?
                    .clone();
                println!(
                    "🎮 TurnPass from connection {} -> player {}",
                    connection_id, player_id
                );
                crate::game::event_handler::GameEvent::TurnPass { player_id }
            }
            GameMessage::PriorityPassFromConnection { connection_id } => {
                let player_id = self
                    .connection_to_player_mapping
                    .get(&connection_id)
                    .ok_or_else(|| {
                        println!(
                            "🚨 Connection {} not found in game {} player mapping",
                            connection_id, self.game_id
                        );
                        AppError::ConnectionNotInRoom
                    })?
                    .clone();
                println!(
                    "🎮 PriorityPass from connection {} -> player {}",
                    connection_id, player_id
                );
                crate::game::event_handler::GameEvent::PriorityPass { player_id }
            }
        };

        // Handle the event using existing coordinator
        self.coordinator
            .handle_event(game_event, cmd_sender)
            .await?;

        Ok(())
    }

    fn get_connection_for_error(&self, error: &AppError) -> Option<String> {
        match error {
            AppError::NotPlayerTurn => {
                // We'd need more context to determine which player caused this error
                // For now, return None - in a full implementation, we'd track the last action
                None
            }
            _ => None,
        }
    }

    async fn send_error_to_connection(
        &self,
        connection_id: &str,
        error: AppError,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        use crate::network::messages::{serialize_response, ServerResponse};

        let _ = cmd_sender.send(ConnectionCommand::SendToPlayer {
            connection_id: connection_id.to_string(),
            message: serialize_response(ServerResponse::from_app_error(&error)),
        });
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
