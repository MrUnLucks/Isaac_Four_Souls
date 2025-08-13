use once_cell::sync::Lazy;
use std::{collections::HashMap, error::Error, fs};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootCard {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub card_type: String,
    pub subtype: String,
    pub description: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    pub loot_cards: HashMap<String, LootCard>,
}

impl Database {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        println!("🃏 Loading card databases...");
        let database_path = fs::read_to_string("src/data/cards/loot.json")?;
        let data: Vec<LootCard> = serde_json::from_str(&database_path)?;
        let mut loot_cards = HashMap::new();

        for database_card in data {
            loot_cards.insert(database_card.id.clone(), database_card);
        }
        Ok(Self { loot_cards })
    }

    pub fn create_loot_deck(&self) -> Vec<LootCard> {
        let mut deck = Vec::new();
        for card in self.loot_cards.values() {
            for _ in 0..card.count {
                deck.push(card.clone());
            }
        }
        deck
    }
}
static CARD_DATABASE: Lazy<Database> =
    Lazy::new(|| Database::load().expect("Failed to load card database"));
pub fn get_database() -> &'static Database {
    &CARD_DATABASE
}

pub fn create_loot_deck() -> Vec<LootCard> {
    CARD_DATABASE.create_loot_deck()
}

pub fn initialize_database() {
    let _ = &*CARD_DATABASE;
    println!("🎮 Global card database initialized");
}
