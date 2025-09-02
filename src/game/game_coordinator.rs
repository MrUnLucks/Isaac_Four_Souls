use std::collections::HashMap;

use crate::game::event_handler::{EventHandler, GameEvent};
use crate::game::game_state::{GameState, TurnPhases};
use crate::game::state_broadcaster::StateBroadcaster;
use crate::ConnectionCommand;
use crate::TurnOrder;
use tokio::sync::mpsc;

pub struct GameCoordinator {
    game_state: GameState,
    event_handler: EventHandler,
    state_broadcaster: StateBroadcaster,
}

impl GameCoordinator {
    pub fn new(
        players_id_to_connection_id: HashMap<String, String>,
        turn_order: TurnOrder,
    ) -> Self {
        let player_ids = players_id_to_connection_id.keys().cloned().collect();
        let game_state = GameState::new(player_ids, turn_order);

        let event_handler = EventHandler::new(players_id_to_connection_id.clone());
        let state_broadcaster = StateBroadcaster::new(players_id_to_connection_id);

        Self {
            game_state,
            event_handler,
            state_broadcaster,
        }
    }

    pub async fn initialize_game(&mut self, cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>) {
        // Send initial state to all players
        self.state_broadcaster
            .broadcast_full_state(&self.game_state, cmd_sender)
            .await;

        // Start first phase
        self.transition_to_phase(self.game_state.current_phase.clone(), cmd_sender)
            .await;
    }

    pub async fn handle_event(
        &mut self,
        event: GameEvent,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Result<(), crate::AppError> {
        match self
            .event_handler
            .handle_event(event, &self.game_state, cmd_sender)
            .await
        {
            Ok(new_state) => {
                self.game_state = new_state;

                // Broadcast updated state
                self.state_broadcaster
                    .broadcast_full_state(&self.game_state, cmd_sender)
                    .await;

                // Check win condition
                if self.check_win_condition() {
                    if let Some(winner) = self.get_winner() {
                        self.end_game(winner, cmd_sender).await;
                    }
                }

                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    async fn transition_to_phase(
        &mut self,
        new_phase: TurnPhases,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
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
                .broadcast_phase_start(&self.game_state, cmd_sender)
                .await;
        }
    }

    fn check_win_condition(&self) -> bool {
        self.game_state.turn_order.get_turn_counter() >= 100
    }

    fn get_winner(&self) -> Option<String> {
        self.game_state.turn_order.order.first().cloned()
    }

    async fn end_game(
        &mut self,
        winner_id: String,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        self.game_state.game_running = false;
        self.state_broadcaster
            .broadcast_game_ended(winner_id, cmd_sender)
            .await;
    }

    pub fn is_running(&self) -> bool {
        self.game_state.game_running
    }
}
