use rand::rng;
use rand::seq::SliceRandom;

#[derive(Debug, Clone)]
pub struct TurnOrder {
    pub order: Vec<String>,
    pub active_player_id: String,
    turn_counter: u32,
}

impl TurnOrder {
    pub fn new(player_ids: Vec<String>) -> Self {
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

    pub fn get_turn_counter(&self) -> u32 {
        self.turn_counter
    }

    pub fn is_player_turn(&self, player_id: &str) -> bool {
        self.active_player_id == player_id
    }

    pub fn advance_turn(&mut self) -> String {
        if let Some(current_index) = self
            .order
            .iter()
            .position(|id| id == &self.active_player_id)
        {
            let next_index = (current_index + 1) % self.order.len();
            self.active_player_id = self.order[next_index].clone();
            self.turn_counter += 1;
        }
        self.active_player_id.clone()
    }
}
