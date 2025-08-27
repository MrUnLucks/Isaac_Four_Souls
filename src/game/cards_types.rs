use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardTemplate {
    pub id: String,
    pub name: String,
    pub card_type: String,
    pub subtype: String,
    pub description: String,
    pub count: u32, // How many copies to create
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Zone {
    Hand,
    LootDeck,
    LootDiscard,
    Playing,
    Item,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CardType {
    Monster,
    Loot,
    Treasure,
    Character,
    BonusSoul,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub entity_id: String,
    pub template_id: String,
    pub name: String,
    pub description: String,
    pub zone: Zone,
    pub card_type: CardType,
    pub owner_id: String,
    pub subtype: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootCard {
    #[serde(flatten)]
    pub card: Card,
}

impl Deref for LootCard {
    type Target = Card;

    fn deref(&self) -> &Self::Target {
        &self.card
    }
}
