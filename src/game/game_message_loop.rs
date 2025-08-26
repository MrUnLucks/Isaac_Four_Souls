use std::collections::HashMap;

use tokio::sync::mpsc;

use crate::game::card_loader::create_loot_deck;
use crate::network::messages::{serialize_response, ServerResponse};
use crate::{ConnectionCommand, TurnOrder};

pub struct GameMessageLoop {
    turn_order: TurnOrder,
    players_id_to_connection_id: HashMap<String, String>,
    room_connections_id: Vec<String>,
}

pub enum GameEvent {
    TurnPass { player_id: String },
    GameOver { winner_id: String },
}

#[derive(Debug, Clone)]
pub enum GameError {
    GameEndedUnexpectedly,
    NotPlayerTurn,
}

impl GameMessageLoop {
    pub fn new(
        players_id_to_connection_id: HashMap<String, String>,
        turn_order: TurnOrder,
    ) -> Self {
        // Create all the loot cards as entities
        let loot_cards = create_loot_deck();
        println!("🃏 Creating {} loot card entities", loot_cards.len());

        let room_connections_id = players_id_to_connection_id
            .values()
            .cloned()
            .into_iter()
            .collect();

        Self {
            turn_order,
            players_id_to_connection_id,
            room_connections_id,
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

                        let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
                            connections_id: self.room_connections_id.clone(),
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
                        connections_id: self.room_connections_id.clone(),
                        message: serialize_response(ServerResponse::GameEnded { winner_id }),
                    });
                }
            }
        }
    }
}
