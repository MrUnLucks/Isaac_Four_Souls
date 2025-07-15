use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerResources {
    pub health: u8,
    pub coins: u8,
    pub souls: u8,
    pub max_health: u8,
}

const DEFAULT_COINS: u8 = 3;
const MAX_COINS: u8 = 99;
const VICTORY_SOULS: u8 = 4;

impl Default for PlayerResources {
    fn default() -> Self {
        Self::new(2) // Isaac's default health
    }
}

impl PlayerResources {
    pub fn new(max_health: u8) -> Self {
        Self {
            coins: DEFAULT_COINS,
            max_health,
            health: max_health,
            souls: 0,
        }
    }

    pub fn take_damage(&mut self, amount: u8) -> bool {
        if amount >= self.health {
            self.health = 0;
            true // Player dies
        } else {
            self.health -= amount;
            false // Player survives
        }
    }

    pub fn heal(&mut self, amount: u8) {
        self.health = (self.health + amount).min(self.max_health);
    }

    pub fn gain_coins(&mut self, amount: u8) {
        // Should cap at 99 coins (Isaac rule)
        self.coins = (self.coins + amount).min(MAX_COINS);
    }

    pub fn spend_coins(&mut self, amount: u8) -> bool {
        if self.coins >= amount {
            self.coins -= amount;
            true
        } else {
            false
        }
    }

    pub fn gain_souls(&mut self, amount: u8) -> bool {
        self.souls += amount;
        self.souls >= VICTORY_SOULS
    }
    pub fn is_alive(&self) -> bool {
        self.health > 0
    }

    pub fn total_souls(&self) -> u8 {
        // Later will include treasure card souls
        self.souls
    }

    pub fn can_afford(&self, cost: u8) -> bool {
        self.coins >= cost
    }
}
