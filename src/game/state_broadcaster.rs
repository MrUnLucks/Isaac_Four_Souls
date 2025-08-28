use crate::game::game_state::GameState;
use crate::network::messages::{serialize_response, ServerResponse};
use crate::ConnectionCommand;
use std::collections::HashMap;
use tokio::sync::mpsc;

pub struct StateBroadcaster {
    players_id_to_connection_id: HashMap<String, String>,
    room_connections_id: Vec<String>,
}

impl StateBroadcaster {
    pub fn new(players_id_to_connection_id: HashMap<String, String>) -> Self {
        let room_connections_id = players_id_to_connection_id.values().cloned().collect();

        Self {
            players_id_to_connection_id,
            room_connections_id,
        }
    }

    pub async fn broadcast_full_state(
        &self,
        state: &GameState,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        self.broadcast_public_state(state, cmd_sender).await;
        self.broadcast_private_states(state, cmd_sender).await;
    }

    async fn broadcast_public_state(
        &self,
        state: &GameState,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        let hand_sizes: HashMap<String, usize> = state
            .board
            .player_hands
            .iter()
            .map(|(player_id, hand)| (player_id.clone(), hand.len()))
            .collect();

        let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
            connections_id: self.room_connections_id.clone(),
            message: serialize_response(ServerResponse::PublicBoardState {
                hand_sizes,
                loot_deck_size: state.board.loot_deck.len(),
                loot_discard: state.board.loot_discard.clone(),
                current_phase: state.current_phase.clone(),
                active_player: state.turn_order.active_player_id.clone(),
            }),
        });
    }

    async fn broadcast_private_states(
        &self,
        state: &GameState,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        for (player_id, conn_id) in &self.players_id_to_connection_id {
            let hand = state
                .board
                .player_hands
                .get(player_id)
                .cloned()
                .unwrap_or_default();

            let _ = cmd_sender.send(ConnectionCommand::SendToPlayer {
                connection_id: conn_id.clone(),
                message: serialize_response(ServerResponse::PrivateBoardState {
                    player_id: player_id.clone(),
                    hand,
                }),
            });
        }
    }

    pub async fn broadcast_phase_start(
        &self,
        state: &GameState,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
            connections_id: self.room_connections_id.clone(),
            message: serialize_response(ServerResponse::PhaseStart {
                phase: state.current_phase.clone(),
                priority_player: state.current_priority_player.clone(),
            }),
        });
    }

    pub async fn broadcast_game_ended(
        &self,
        winner_id: String,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
            connections_id: self.room_connections_id.clone(),
            message: serialize_response(ServerResponse::GameEnded { winner_id }),
        });
    }
}
