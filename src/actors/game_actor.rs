use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::game::game_coordinator::GameCoordinator;
use crate::{AppError, ConnectionCommand, TurnOrder};

#[derive(Debug, Clone)]
pub enum GameMessage {
    TurnPass { player_id: String },
    PriorityPass { player_id: String },
    // Future: PlayCard, UseItem, etc.
}

pub struct GameActor {
    game_id: String,
    coordinator: GameCoordinator,
    connection_to_player_mapping: HashMap<String, String>, // connection_id -> player_id
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

        let coordinator = GameCoordinator::new(players_id_to_connection_id, turn_order);

        Self {
            game_id,
            coordinator,
            connection_to_player_mapping,
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
                                // Send error to relevant player if possible
                                // Could break here if critical error
                            }
                        }
                        None => {
                            // Channel closed, end game
                            println!("🎮 Game actor {} receiver closed", self.game_id);
                            break;
                        }
                    }
                }

                // Could add other select branches here:
                // - Periodic game state updates
                // - Timeout handling
                // - Health checks
            }
        }

        println!("🎮 Game actor ended for game {}", self.game_id);
    }

    async fn handle_message(
        &mut self,
        message: GameMessage,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Result<(), AppError> {
        // Convert GameMessage to GameEvent (similar to old system)
        let game_event = match message {
            GameMessage::TurnPass { player_id } => {
                crate::game::event_handler::GameEvent::TurnPass { player_id }
            }
            GameMessage::PriorityPass { player_id } => {
                crate::game::event_handler::GameEvent::PriorityPass { player_id }
            }
        };

        // Handle the event using existing coordinator
        self.coordinator
            .handle_event(game_event, cmd_sender)
            .await?;

        Ok(())
    }

    pub fn get_player_id_from_connection(&self, connection_id: &str) -> Option<String> {
        self.connection_to_player_mapping
            .get(connection_id)
            .cloned()
    }
}
