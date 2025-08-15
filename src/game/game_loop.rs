use tokio::sync::mpsc;

use crate::game::decks::LootDeck;
use crate::TurnOrder;

pub struct GameLoop {
    loot_deck: LootDeck,
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
    pub fn new() -> Self {
        let loot_deck = LootDeck::new();
        Self { loot_deck }
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

                        println!("Turn passed to: {}", next_player);
                    }
                }
                GameEvent::GameOver => (), // Need handling
            }
        }
        Err(GameError::GameEndedUnexpectedly)
    }
}
