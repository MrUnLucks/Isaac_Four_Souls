use std::collections::HashMap;

use crate::game::event_handler::GameEvent;
use crate::game::game_coordinator::GameCoordinator;
use crate::{ConnectionCommand, TurnOrder};
use tokio::sync::mpsc;

pub struct GameMessageLoop {
    coordinator: GameCoordinator,
}

impl GameMessageLoop {
    pub fn new(
        players_id_to_connection_id: HashMap<String, String>,
        turn_order: TurnOrder,
    ) -> Self {
        let coordinator = GameCoordinator::new(players_id_to_connection_id, turn_order);

        Self { coordinator }
    }

    pub async fn run(
        &mut self,
        mut event_receiver: mpsc::Receiver<GameEvent>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        // Initialize the game
        self.coordinator.initialize_game(&cmd_sender).await;

        // Main event loop
        while self.coordinator.is_running() {
            if let Some(event) = event_receiver.recv().await {
                if let Err(error) = self.coordinator.handle_event(event, &cmd_sender).await {
                    eprintln!("Game event error: {:?}", error);
                    // Could send error to relevant player here
                }
            } else {
                // Channel closed, end game loop
                break;
            }
        }

        println!("🎮 Game loop ended");
    }
}
