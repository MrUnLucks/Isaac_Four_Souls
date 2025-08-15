use crate::game::decks::LootDeck;
use crate::game::outbound_handler::GameOutboundEvent;
use crate::TurnOrder;
use tokio::sync::mpsc;

pub struct GameLoop {
    loot_deck: LootDeck,
}

pub enum GameEvent {
    TurnPass { player_id: String },
    GameOver,
}

#[derive(Debug, Clone)]
pub enum GameError {
    GameEndedUnexpectedly,
    NotPlayerTurn,
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
        outbound_sender: mpsc::Sender<GameOutboundEvent>,
    ) -> Result<(), GameError> {
        while let Some(event) = event_receiver.recv().await {
            match event {
                GameEvent::TurnPass { player_id } => {
                    if turn_order.is_player_turn(&player_id) {
                        let next_player = turn_order.advance_turn();
                        println!("Turn passed to: {}", next_player);

                        if turn_order.get_turn_counter() >= 4 {
                            let _ = outbound_sender
                                .send(GameOutboundEvent::GameEnded {
                                    winner_id: player_id,
                                })
                                .await;
                        }

                        let _ = outbound_sender
                            .send(GameOutboundEvent::TurnChanged {
                                next_player_id: next_player,
                            })
                            .await;
                    } else {
                        let _ = outbound_sender
                            .send(GameOutboundEvent::GameError {
                                error: GameError::NotPlayerTurn,
                            })
                            .await;
                    }
                }
                GameEvent::GameOver => return Ok(()),
            }
        }
        Err(GameError::GameEndedUnexpectedly)
    }
}
