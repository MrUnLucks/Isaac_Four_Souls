use rand::rng;
use rand::seq::SliceRandom;

#[derive(Debug, Clone)]
pub struct TurnOrder {
    pub order: Vec<String>,
    active_player_id: String,
    turn_counter: u32,
}

pub enum TurnPhases {
    UntapStep,
    StartStep, // Start of turn abilities
    LootStep,
    ActionStep, // Loot play - Attack - Shop
    EndStep,    // End of turn abilities
    TurnEnd,
}

impl TurnOrder {
    pub fn new(player_ids: &Vec<String>) -> Self {
        let mut random_generator = rng();
        let mut order = player_ids.clone();
        order.shuffle(&mut random_generator);

        let active_player_id = order[0].clone();

        Self {
            order,
            active_player_id,
            turn_counter: 0,
        }
    }

    pub fn is_player_turn(&self, player_id: &str) -> bool {
        self.active_player_id == player_id
    }

    pub fn advance_turn(&mut self) {
        if let Some(current_index) = self
            .order
            .iter()
            .position(|id| id == &self.active_player_id)
        {
            let next_index = (current_index + 1) % self.order.len();
            self.active_player_id = self.order[next_index].clone();
            self.turn_counter += 1;
        }
    }
}
