use crate::game::card_loader::LootCard;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct CardComponent {
    pub card_data: LootCard,
}

#[derive(Debug, Clone)]
pub struct InDeckComponent;

#[derive(Debug, Clone)]
pub struct InDiscardPileComponent;

#[derive(Debug, Clone)]
pub struct InHandComponent {
    pub player_id: String,
}

#[derive(Debug, Clone)]
pub struct DeckComponent {
    pub deck_type: DeckType,
}

#[derive(Debug, Clone)]
pub enum DeckType {
    Loot,
    Treasure,
    Monster,
}

#[derive(Debug, Clone)]
pub struct DiscardPileComponent {
    pub pile_type: DeckType,
}

#[derive(Debug, Clone)]
pub struct PlayerComponent {
    pub player_id: String,
    pub name: String,
    pub connection_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesComponent {
    pub health: u8,
    pub coins: u8,
    pub souls: u8,
    pub max_health: u8,
}

impl ResourcesComponent {
    pub fn new(max_health: u8) -> Self {
        Self {
            coins: 3,
            max_health,
            health: max_health,
            souls: 0,
        }
    }
}
