use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::game::game_loop::{GameEvent, GameLoop};
use crate::{AppError, AppResult, TurnOrder};

/// Handles pure game logic and game loop management
pub struct GameLoopRegistry {
    game_loops: HashMap<String, mpsc::Sender<GameEvent>>, // room_id -> game event sender
}

impl GameLoopRegistry {
    pub fn new() -> Self {
        Self {
            game_loops: HashMap::new(),
        }
    }

    pub fn start_game_loop(&mut self, room_id: &str, turn_order: &TurnOrder) -> AppResult<()> {
        let (sender, receiver) = mpsc::channel(32);

        self.game_loops.insert(room_id.to_string(), sender);
        println!("{:?}", self.game_loops);

        let mut game_loop = GameLoop::new();
        let turn_order_clone = turn_order.clone();
        let room_id_clone = room_id.to_string();

        tokio::spawn(async move {
            let result = game_loop.run(turn_order_clone, receiver).await;
            println!(
                "Game loop for room {} finished with result: {:?}",
                room_id_clone, result
            );
        });

        Ok(())
    }

    pub fn send_game_event(&self, room_id: &str, event: GameEvent) -> AppResult<()> {
        if let Some(sender) = self.game_loops.get(room_id) {
            sender
                .try_send(event)
                .map_err(|err| AppError::GameEventSendFailed {
                    reason: err.to_string(),
                })?;
            Ok(())
        } else {
            Err(AppError::GameLoopNotFound {
                room_id: room_id.to_string(),
            })
        }
    }

    pub fn cleanup_game_loop(&mut self, room_id: &str) {
        self.game_loops.remove(room_id);
    }

    pub fn has_game_loop(&self, room_id: &str) -> bool {
        self.game_loops.contains_key(room_id)
    }
}
