use std::collections::HashMap;

use crate::game::game_state::{GameState, TurnPhases};
use crate::network::messages::{serialize_response, ServerResponse};
use crate::{AppError, ConnectionCommand};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum GameEvent {
    TurnPass { player_id: String },
    PriorityPass { player_id: String },
}

pub struct EventHandler {
    players_id_to_connection_id: HashMap<String, String>,
    room_connections_id: Vec<String>,
}

impl EventHandler {
    pub fn new(players_id_to_connection_id: HashMap<String, String>) -> Self {
        let room_connections_id = players_id_to_connection_id.values().cloned().collect();

        Self {
            players_id_to_connection_id,
            room_connections_id,
        }
    }

    pub async fn handle_event(
        &self,
        event: GameEvent,
        current_state: &GameState,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Result<GameState, AppError> {
        match event {
            GameEvent::TurnPass { player_id } => {
                if current_state.can_player_pass_turn(&player_id) {
                    let new_state = current_state.with_phase_transition(TurnPhases::TurnEnd);
                    self.broadcast_turn_change(&new_state, cmd_sender).await;
                    Ok(new_state)
                } else {
                    Err(AppError::NotPlayerTurn)
                }
            }
            GameEvent::PriorityPass { player_id } => {
                match current_state.with_priority_pass(player_id) {
                    Ok(new_state) => {
                        self.broadcast_priority_change(&new_state, cmd_sender).await;
                        Ok(new_state)
                    }
                    Err(AppError::InvalidPriorityPass) => Err(AppError::InvalidPriorityPass),
                    _ => Err(AppError::Internal {
                        message: "Unexpected game state error".to_string(),
                    }),
                }
            }
        }
    }

    async fn broadcast_turn_change(
        &self,
        state: &GameState,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
            connections_id: self.room_connections_id.clone(),
            message: serialize_response(ServerResponse::TurnChange {
                next_player_id: state.turn_order.active_player_id.clone(),
            }),
        });
    }

    async fn broadcast_priority_change(
        &self,
        state: &GameState,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
            connections_id: self.room_connections_id.clone(),
            message: serialize_response(ServerResponse::PriorityChange {
                player_id: state.current_priority_player.clone(),
            }),
        });
    }

    pub async fn send_error_to_player(
        &self,
        player_id: &str,
        error: AppError,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        if let Some(connection_id) = self.players_id_to_connection_id.get(player_id) {
            let _ = cmd_sender.send(ConnectionCommand::SendToPlayer {
                connection_id: connection_id.clone(),
                message: serialize_response(ServerResponse::from_app_error(&error)),
            });
        }
    }
}
