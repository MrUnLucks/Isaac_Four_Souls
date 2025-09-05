use serde::Serialize;
use std::collections::HashSet;

use crate::game::board::Board;
use crate::{AppError, AppResult, TurnOrder};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum TurnPhases {
    UntapStartStep,
    LootStep,
    ActionStep,
    EndStep,
    TurnEnd,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub turn_order: TurnOrder,
    pub current_phase: TurnPhases,
    pub current_priority_player: String,
    pub players_passed_priority: HashSet<String>,
    pub board: Board,
    pub game_running: bool,
    pub waiting_for_priority: bool,
}

impl GameState {
    pub fn new(player_ids: Vec<String>, turn_order: TurnOrder) -> Self {
        let board = Board::new(player_ids);
        Self {
            current_priority_player: turn_order.active_player_id.clone(),
            current_phase: TurnPhases::UntapStartStep,
            turn_order,
            board,
            players_passed_priority: HashSet::new(),
            game_running: true,
            waiting_for_priority: false,
        }
    }

    // Pure state validation - no side effects
    pub fn can_player_pass_turn(&self, player_id: &str) -> bool {
        self.turn_order.is_player_turn(player_id)
    }

    pub fn can_player_pass_priority(&self, player_id: &str) -> bool {
        self.waiting_for_priority && self.current_priority_player == *player_id
    }

    pub fn all_players_passed_priority(&self) -> bool {
        self.players_passed_priority.len() == self.turn_order.order.len()
    }

    pub fn get_next_phase(&self) -> TurnPhases {
        match self.current_phase {
            TurnPhases::UntapStartStep => TurnPhases::LootStep,
            TurnPhases::LootStep => TurnPhases::ActionStep,
            TurnPhases::ActionStep => TurnPhases::EndStep,
            TurnPhases::EndStep => TurnPhases::TurnEnd,
            TurnPhases::TurnEnd => TurnPhases::UntapStartStep,
        }
    }

    pub fn get_next_priority_player(&self) -> Option<String> {
        let current_idx = self
            .turn_order
            .order
            .iter()
            .position(|id| id == &self.current_priority_player)?;

        for i in 1..=self.turn_order.order.len() {
            let next_idx = (current_idx + i) % self.turn_order.order.len();
            let next_player = &self.turn_order.order[next_idx];

            if !self.players_passed_priority.contains(next_player) {
                return Some(next_player.clone());
            }
        }
        None
    }

    // State mutation methods - return new state or error
    pub fn with_priority_pass(&self, player_id: String) -> AppResult<Self> {
        if !self.can_player_pass_priority(&player_id) {
            println!("❌ Player {} cannot pass priority", player_id);
            return Err(AppError::InvalidPriorityPass);
        }

        let mut new_state = self.clone();
        new_state.players_passed_priority.insert(player_id.clone());

        if new_state.all_players_passed_priority() {
            println!("🎯 All players passed priority, advancing phase");
            new_state = new_state.with_phase_transition(new_state.get_next_phase());
        } else if let Some(next_player) = new_state.get_next_priority_player() {
            println!("🎯 Next priority player: {}", next_player);
            new_state.current_priority_player = next_player;
        } else {
            println!("❌ No next priority player found!");
        }

        Ok(new_state)
    }

    pub fn with_phase_transition(&self, new_phase: TurnPhases) -> Self {
        let mut new_state = self.clone();
        new_state.current_phase = new_phase.clone();

        if matches!(new_phase, TurnPhases::TurnEnd) {
            new_state.turn_order.advance_turn();
            new_state.current_phase = TurnPhases::UntapStartStep;
            new_state.current_priority_player = new_state.turn_order.active_player_id.clone();
            new_state.waiting_for_priority = true;
            new_state.players_passed_priority.clear();
            // Temporary since Priority is commented
            new_state
                .board
                .draw_loot_for_player(&new_state.current_priority_player);
        } else {
            new_state.waiting_for_priority = true;
            new_state.players_passed_priority.clear();
            new_state.current_priority_player = new_state.turn_order.active_player_id.clone();
        }

        println!(
            "🔄 New state - Phase: {:?}, Priority player: {}, Waiting: {}, Passed: {:?}",
            new_state.current_phase,
            new_state.current_priority_player,
            new_state.waiting_for_priority,
            new_state.players_passed_priority
        );

        new_state
    }
}
