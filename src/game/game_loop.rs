use tokio::sync::mpsc;

use crate::game::turn_order::TurnOrder;

pub struct GameLoop {
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
    const MAX_TURNS: u32 = 4;
    pub fn new() -> Self {
        Self {
            max_turns: Self::MAX_TURNS,
        }
    }

    pub async fn run(
        &mut self,
        mut turn_order: TurnOrder,
        mut event_receiver: mpsc::Receiver<GameEvent>,
    ) -> Result<(), GameError> {
        while let Some(event) = event_receiver.recv().await {
            match event {
                GameEvent::TurnPass { player_id } => {
                    if turn_order.is_player_turn(&player_id) {
                        let next_player = turn_order.advance_turn();

                        if turn_order.get_turn_counter() >= self.max_turns {
                            println!("Max turn Reached!");
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
