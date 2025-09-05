use crate::game::game_state::GameState;
use crate::network::messages::{serialize_response, ServerResponse};
use crate::ConnectionCommand;
use std::collections::HashMap;
use tokio::sync::mpsc;

pub struct StateBroadcaster {
    players_id_to_connection_id: HashMap<String, String>,
    room_connections_id: Vec<String>,
    cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
}

impl StateBroadcaster {
    pub fn new(
        players_id_to_connection_id: HashMap<String, String>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Self {
        let room_connections_id = players_id_to_connection_id.values().cloned().collect();

        Self {
            players_id_to_connection_id,
            room_connections_id,
            cmd_sender,
        }
    }

    pub async fn broadcast_full_state(&self, state: &GameState) {
        self.broadcast_public_state(state).await;
        self.broadcast_private_states(state).await;
    }

    async fn broadcast_public_state(&self, state: &GameState) {
        let _ = self.cmd_sender.send(ConnectionCommand::SendToPlayers {
            connections_id: self.room_connections_id.clone(),
            message: serialize_response(ServerResponse::PublicBoardState {
                loot_deck_size: state.board.loot_deck.len(),
                loot_discard: state.board.loot_discard.clone(),
                current_phase: state.current_phase.clone(),
                active_player: state.turn_order.active_player_id.clone(),
                players: state.board.players.clone(),
            }),
        });
    }

    async fn broadcast_private_states(&self, state: &GameState) {
        for (player_id, conn_id) in &self.players_id_to_connection_id {
            let player_hand = state.board.players_hands.get(player_id).cloned();
            match player_hand {
                None => {
                    let _ = self.cmd_sender.send(ConnectionCommand::SendToPlayer {
                        connection_id: conn_id.clone(),
                        message: serialize_response(ServerResponse::from_app_error(
                            &crate::AppError::PlayerNotFound,
                        )),
                    });
                }
                Some(player_hand) => {
                    let _ = self.cmd_sender.send(ConnectionCommand::SendToPlayer {
                        connection_id: conn_id.clone(),
                        message: serialize_response(ServerResponse::PrivateBoardState {
                            hand: player_hand,
                        }),
                    });
                }
            }
        }
    }

    pub async fn broadcast_phase_start(&self, state: &GameState) {
        let _ = self.cmd_sender.send(ConnectionCommand::SendToPlayers {
            connections_id: self.room_connections_id.clone(),
            message: serialize_response(ServerResponse::TurnPhaseChange {
                player_id: state.current_priority_player.clone(),
                phase: state.current_phase.clone(),
            }),
        });
    }

    pub async fn broadcast_game_ended(&self, winner_id: String) {
        let _ = self.cmd_sender.send(ConnectionCommand::SendToPlayers {
            connections_id: self.room_connections_id.clone(),
            message: serialize_response(ServerResponse::GameEnded { winner_id }),
        });
    }
}
