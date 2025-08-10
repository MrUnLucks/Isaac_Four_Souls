use crate::game::order::TurnOrder;
use tokio::sync::mpsc;

pub struct GameLoop {
    turn_order: TurnOrder,
    max_turns: u32,
}

pub enum GameEvent {
    TurnPass { player_id: String },
    GameOver,
}

#[derive(Debug)]
pub enum GameError {
    GameEndedUnexpectedly,
}

impl GameLoop {
    pub fn new(player_ids: Vec<String>) -> Self {
        Self {
            turn_order: TurnOrder::new(&player_ids),
            max_turns: 4,
        }
    }

    pub async fn run(
        &mut self,
        mut event_receiver: mpsc::Receiver<GameEvent>,
    ) -> Result<(), GameError> {
        while let Some(event) = event_receiver.recv().await {
            match event {
                GameEvent::TurnPass { player_id } => {
                    if self.turn_order.is_player_turn(&player_id) {
                        let next_player = self.turn_order.advance_turn();

                        if self.turn_order.turn_counter >= self.max_turns {
                            () // Need handling
                        }

                        println!("Turn passed to: {}", next_player);
                    }
                }
                GameEvent::GameOver => (), // Need handling
            }
        }
        Err(GameError::GameEndedUnexpectedly)
    }
}
