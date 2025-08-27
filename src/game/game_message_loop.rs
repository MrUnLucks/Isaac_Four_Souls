use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;

use crate::game::board::Board;
use crate::network::messages::{serialize_response, ServerResponse};
use crate::{ConnectionCommand, TurnOrder};

#[derive(Debug, Clone, Serialize)]
//Priority pass between EACH phase (Except Shop and TurnEnd)
pub enum TurnPhases {
    UntapStartStep, // Start of turn abilities and Untap step -
    //they are together cause priority DOES NOT pass on untap step
    LootStep,
    ActionStep, // Loot play - Attack - Shop
    EndStep,    // End of turn abilities
    TurnEnd,    // Cleanup step is here
}

#[derive(Debug, Clone)]
pub struct GameMessageLoop {
    turn_order: TurnOrder,
    players_id_to_connection_id: HashMap<String, String>,
    room_connections_id: Vec<String>,
    current_phase: TurnPhases,
    game_running: bool,
    waiting_for_priority: bool,
    players_passed_priority: HashSet<String>,
    current_priority_player: String,
    board: Board,
}

#[derive(Debug, Clone)]
pub enum GameEvent {
    TurnPass { player_id: String },
    PriorityPass { player_id: String },
}

#[derive(Debug, Clone)]
pub enum GameError {
    GameEndedUnexpectedly,
    NotPlayerTurn,
    InvalidPhaseAction,
}

impl GameMessageLoop {
    pub fn new(
        players_id_to_connection_id: HashMap<String, String>,
        turn_order: TurnOrder,
    ) -> Self {
        let room_connections_id = players_id_to_connection_id
            .values()
            .cloned()
            .into_iter()
            .collect();

        let board = Board::new(players_id_to_connection_id.keys().cloned().collect());
        Self {
            turn_order: turn_order.clone(),
            players_id_to_connection_id,
            room_connections_id,
            current_phase: TurnPhases::UntapStartStep,
            game_running: true,
            waiting_for_priority: false,
            players_passed_priority: HashSet::new(),
            current_priority_player: turn_order.active_player_id.clone(),
            board,
        }
    }

    pub async fn run(
        &mut self,
        mut event_receiver: mpsc::Receiver<GameEvent>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        let mut player_hands: HashMap<String, usize> = self
            .board
            .player_hands
            .iter()
            .map(|(player_id, hand)| (player_id.clone(), hand.len()))
            .collect();
        let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
            connections_id: self.room_connections_id.clone(),
            message: serialize_response(ServerResponse::PublicBoardState {
                hand_sizes: player_hands,
                loot_deck_size: self.board.loot_deck.len(),
                loot_discard: self.board.loot_discard.clone(),
                current_phase: self.current_phase.clone(),
                active_player: self.turn_order.active_player_id.clone(),
            }),
        });
        for (player_id, conn_id) in self.players_id_to_connection_id.clone() {
            let _ = cmd_sender.send(ConnectionCommand::SendToPlayer {
                connection_id: conn_id,
                message: serialize_response(ServerResponse::PrivateBoardState {
                    player_id: player_id.clone(),
                    hand: self.board.player_hands.get(&player_id).cloned().unwrap(),
                }),
            });
        }
        self.transition_to_phase(TurnPhases::UntapStartStep, &cmd_sender)
            .await;

        while self.game_running {
            if let Some(event) = event_receiver.recv().await {
                self.handle_game_event(event, &cmd_sender).await;
            } else {
                // Channel closed, end game loop
                break;
            }
        }

        println!("🎮 Game loop ended");
    }

    async fn handle_game_event(
        &mut self,
        event: GameEvent,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        match event {
            GameEvent::TurnPass { player_id } => {
                if self.turn_order.is_player_turn(&player_id) {
                    // Force advance to TurnEnd phase, which will advance the turn
                    self.transition_to_phase(TurnPhases::TurnEnd, cmd_sender)
                        .await;
                } else {
                    self.send_error_to_player(
                        &player_id,
                        crate::AppError::NotPlayerTurn,
                        cmd_sender,
                    )
                    .await;
                }
            }
            GameEvent::PriorityPass { player_id } => {
                self.handle_priority_cycle(player_id, cmd_sender).await;
            }
        }
    }

    async fn transition_to_phase(
        &mut self,
        new_phase: TurnPhases,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        self.current_phase = new_phase;

        if matches!(self.current_phase, TurnPhases::UntapStartStep) {
            self.board
                .draw_loot_for_player(&self.turn_order.active_player_id);
        }

        if matches!(self.current_phase, TurnPhases::TurnEnd) {
            let next_player = self.turn_order.advance_turn();
            println!("🔄 Turn advanced to: {}", next_player);

            let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
                connections_id: self.room_connections_id.clone(),
                message: serialize_response(ServerResponse::TurnChange {
                    next_player_id: next_player.clone(),
                }),
            });

            // Reset to first phase of new turn
            self.current_phase = TurnPhases::UntapStartStep;
            self.current_priority_player = next_player;
        }

        // Start priority passing for phases that use it
        if !matches!(self.current_phase, TurnPhases::TurnEnd) {
            self.start_priority_cycle(cmd_sender).await;
        }
    }

    async fn start_priority_cycle(
        &mut self,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        self.waiting_for_priority = true;
        self.players_passed_priority.clear(); // Reset for new cycle
        self.current_priority_player = self.turn_order.active_player_id.clone();

        let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
            connections_id: self.room_connections_id.clone(),
            message: serialize_response(ServerResponse::PhaseStart {
                phase: self.current_phase.clone(),
                priority_player: self.current_priority_player.clone(),
            }),
        });
    }

    async fn handle_priority_cycle(
        &mut self,
        player_id: String,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        if !self.waiting_for_priority || self.current_priority_player != player_id {
            return; // Not in priority cycle or not this player's turn
        }

        // Add the player who just passed
        self.players_passed_priority.insert(player_id);

        // Check if all players have passed
        if self.players_passed_priority.len() == self.turn_order.order.len() {
            self.waiting_for_priority = false;
            let next_phase = self.get_next_phase();
            Box::pin(self.transition_to_phase(next_phase, cmd_sender)).await;
            return;
        }

        // Find next player who hasn't passed
        let current_idx = self
            .turn_order
            .order
            .iter()
            .position(|id| id == &self.current_priority_player)
            .unwrap_or(0);

        for i in 1..=self.turn_order.order.len() {
            let next_idx = (current_idx + i) % self.turn_order.order.len();
            let next_player = &self.turn_order.order[next_idx];

            if !self.players_passed_priority.contains(next_player) {
                self.current_priority_player = next_player.clone();

                let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
                    connections_id: self.room_connections_id.clone(),
                    message: serialize_response(ServerResponse::PriorityChange {
                        player_id: self.current_priority_player.clone(),
                    }),
                });
                break;
            }
        }
    }

    fn get_next_phase(&self) -> TurnPhases {
        match self.current_phase {
            TurnPhases::UntapStartStep => TurnPhases::LootStep,
            TurnPhases::LootStep => TurnPhases::ActionStep,
            TurnPhases::ActionStep => TurnPhases::EndStep,
            TurnPhases::EndStep => TurnPhases::TurnEnd,
            TurnPhases::TurnEnd => TurnPhases::UntapStartStep, // This shouldn't happen
        }
    }

    fn check_win_condition(&self) -> bool {
        // TODO: Implement actual win condition logic
        // This is only called at phase boundaries, not continuously
        self.turn_order.get_turn_counter() >= 100 // Example condition
    }

    fn get_winner(&self) -> Option<String> {
        // TODO: Implement actual winner determination logic
        self.turn_order.order.first().cloned()
    }

    async fn send_error_to_player(
        &self,
        player_id: &str,
        error: crate::AppError,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        if let Some(connection_id) = self.players_id_to_connection_id.get(player_id) {
            let _ = cmd_sender.send(ConnectionCommand::SendToPlayer {
                connection_id: connection_id.clone(),
                message: serialize_response(ServerResponse::from_app_error(&error)),
            });
        }
    }

    async fn end_game(
        &mut self,
        winner_id: String,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        self.game_running = false;
        println!("🏆 Game ended! Winner: {}", winner_id);

        let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
            connections_id: self.room_connections_id.clone(),
            message: serialize_response(ServerResponse::GameEnded { winner_id }),
        });
    }
}
