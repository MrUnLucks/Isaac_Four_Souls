use std::collections::HashMap;

use crate::game::decks::LootDeck;
use crate::network::messages::{serialize_response, ServerResponse};
use crate::{ConnectionCommand, TurnOrder};
use tokio::sync::mpsc;

pub enum GameEvent {
    TurnPass { player_id: String },
    GameOver { winner_id: String },
}

#[derive(Debug, Clone)]
pub enum GameError {
    GameEndedUnexpectedly,
    NotPlayerTurn,
}

pub struct GameLoop {
    loot_deck: LootDeck,
    turn_order: TurnOrder,
    players_id_to_connection_id: HashMap<String, String>,
}

impl GameLoop {
    pub fn new(
        players_id_to_connection_id: HashMap<String, String>,
        turn_order: TurnOrder,
    ) -> Self {
        let loot_deck = LootDeck::new();
        Self {
            loot_deck,
            turn_order,
            players_id_to_connection_id,
        }
    }

    pub async fn run(
        &mut self,
        mut event_receiver: mpsc::Receiver<GameEvent>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        while let Some(event) = event_receiver.recv().await {
            match event {
                GameEvent::TurnPass { player_id } => {
                    if self.turn_order.is_player_turn(&player_id) {
                        let next_player = self.turn_order.advance_turn();
                        println!("Turn passed to: {}", next_player);

                        if self.turn_order.get_turn_counter() >= 4 {
                            let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
                                connections_id: self
                                    .players_id_to_connection_id
                                    .values()
                                    .cloned()
                                    .into_iter()
                                    .collect(),
                                message: serialize_response(ServerResponse::GameEnded {
                                    winner_id: player_id,
                                }),
                            });
                        }

                        let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
                            connections_id: self
                                .players_id_to_connection_id
                                .values()
                                .cloned()
                                .into_iter()
                                .collect(),
                            message: serialize_response(ServerResponse::TurnChange {
                                next_player_id: next_player,
                            }),
                        });
                    } else {
                        //TODO: ERROR HANDLING FOR ALL GAMEERRORS
                        let player_connection_id = self
                            .players_id_to_connection_id
                            .get(&player_id)
                            .expect("NEED HANDLING")
                            .clone();
                        let _ = cmd_sender.send(ConnectionCommand::SendToPlayer {
                            connection_id: player_connection_id,
                            message: serialize_response(ServerResponse::from_app_error(
                                &crate::AppError::NotPlayerTurn,
                            )),
                        });
                    }
                }
                GameEvent::GameOver { winner_id } => {
                    let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
                        connections_id: self
                            .players_id_to_connection_id
                            .values()
                            .cloned()
                            .into_iter()
                            .collect(),
                        message: serialize_response(ServerResponse::GameEnded { winner_id }),
                    });
                }
            }
        }
    }
}
