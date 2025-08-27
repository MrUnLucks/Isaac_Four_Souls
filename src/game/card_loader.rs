use once_cell::sync::Lazy;
use rand::rng;
use rand::seq::SliceRandom;
use std::{collections::HashMap, error::Error, fs};
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::game::cards_types::{Card, CardTemplate, CardType, LootCard, Zone};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    pub loot_templates: HashMap<String, CardTemplate>,
}

impl Database {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        println!("🃏 Loading card databases...");
        let database_path = fs::read_to_string("src/data/cards/loot.json")?;
        let data: Vec<CardTemplate> = serde_json::from_str(&database_path)?;
        let mut loot_templates = HashMap::new();

        for database_card in data {
            loot_templates.insert(database_card.id.clone(), database_card);
        }
        Ok(Self { loot_templates })
    }

    pub fn create_loot_deck(&self) -> Vec<LootCard> {
        let mut deck = Vec::new();
        for template in self.loot_templates.values() {
            for _ in 0..template.count {
                let card = Card {
                    entity_id: Uuid::new_v4().to_string(),
                    template_id: template.id.clone(),
                    name: template.name.clone(),
                    description: template.description.clone(),
                    zone: Zone::LootDeck,
                    card_type: CardType::Loot,
                    owner_id: String::new(), // Set when drawn
                    subtype: template.subtype.clone(),
                };

                deck.push(LootCard { card });
            }
        }
        let mut random_generator = rng();
        deck.shuffle(&mut random_generator);
        println!("{:?}", deck);
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
