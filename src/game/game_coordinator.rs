use std::collections::HashMap;

use crate::game::game_state::{GameState, TurnPhases};
use crate::game::state_broadcaster::StateBroadcaster;
use crate::{AppError, ConnectionCommand};
use crate::{AppResult, TurnOrder};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum GameEvent {
    TurnPass { player_id: String },
    // PriorityPass { player_id: String },
}

pub struct GameCoordinator {
    game_state: GameState,
    state_broadcaster: StateBroadcaster,
}

impl GameCoordinator {
    pub fn new(
        players_id_to_connection_id: HashMap<String, String>,
        turn_order: TurnOrder,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Self {
        let player_ids = players_id_to_connection_id.keys().cloned().collect();
        let game_state = GameState::new(player_ids, turn_order);

        let state_broadcaster = StateBroadcaster::new(players_id_to_connection_id, cmd_sender);

        Self {
            game_state,
            state_broadcaster,
        }
    }

    pub async fn initialize_game(&mut self) {
        // Temporary for shortcircuiting priority
        let _ = self
            .game_state
            .board
            .draw_loot_for_player(&self.game_state.turn_order.active_player_id);

        // Send initial state to all players
        self.state_broadcaster
            .broadcast_full_state(&self.game_state)
            .await;

        // Start first phase
        self.transition_to_phase(self.game_state.current_phase.clone())
            .await;
    }

    pub async fn handle_event(&mut self, event: GameEvent) -> Result<(), AppError> {
        match self.handle_game_event(event, &self.game_state).await {
            Ok(new_state) => {
                self.game_state = new_state;

                self.state_broadcaster
                    .broadcast_full_state(&self.game_state)
                    .await;

                // Check win condition
                if self.check_win_condition() {
                    if let Some(winner) = self.get_winner() {
                        self.end_game(winner).await;
                    }
                }

                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    pub async fn handle_game_event(
        &self,
        event: GameEvent,
        current_state: &GameState,
    ) -> AppResult<GameState> {
        match event {
            GameEvent::TurnPass { player_id } => {
                if current_state.can_player_pass_turn(&player_id) {
                    let new_state = current_state.with_phase_transition(TurnPhases::TurnEnd);
                    let _ = self.state_broadcaster.broadcast_phase_start(&new_state);
                    Ok(new_state)
                } else {
                    Err(AppError::NotPlayerTurn)
                }
            } // GameEvent::PriorityPass { player_id } => {
              //     match current_state.with_priority_pass(player_id) {
              //         Ok(new_state) => {
              //             let _ = self.state_broadcaster.broadcast_phase_start(&new_state);
              //             Ok(new_state)
              //         }
              //         Err(AppError::InvalidPriorityPass) => Err(AppError::InvalidPriorityPass),
              //         _ => Err(AppError::Internal {
              //             message: "Unexpected game state error".to_string(),
              //         }),
              //     }
              // }
        }
    }

    async fn transition_to_phase(&mut self, new_phase: TurnPhases) {
        self.game_state = self.game_state.with_phase_transition(new_phase);

        // Handle phase-specific logic
        if matches!(self.game_state.current_phase, TurnPhases::LootStep) {
            // Draw loot for active player
            let _ = self
                .game_state
                .board
                .draw_loot_for_player(&self.game_state.turn_order.active_player_id);
        }

        // Start priority if not TurnEnd
        if !matches!(self.game_state.current_phase, TurnPhases::TurnEnd) {
            self.state_broadcaster
                .broadcast_phase_start(&self.game_state)
                .await;
        }
    }

    fn check_win_condition(&self) -> bool {
        self.game_state.turn_order.get_turn_counter() >= 100
    }

    fn get_winner(&self) -> Option<String> {
        self.game_state.turn_order.order.first().cloned()
    }

    async fn end_game(&mut self, winner_id: String) {
        self.game_state.game_running = false;
        self.state_broadcaster.broadcast_game_ended(winner_id).await;
    }

    pub fn is_running(&self) -> bool {
        self.game_state.game_running
    }
}
