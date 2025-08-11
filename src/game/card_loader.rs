use rand::rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, fs};

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
pub struct LootDatabase {
    pub loot_cards: Vec<LootCard>,
}

#[derive(Debug, Clone)]
pub struct LootDeck {
    deck: Vec<LootCard>,
    discard_pile: Vec<LootCard>,
    database: HashMap<String, LootCard>,
}

impl LootDeck {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let database_path = fs::read_to_string("src/data/cards/loot.json")?;
        let data: Vec<LootCard> = serde_json::from_str(&database_path)?;
        let mut database: HashMap<String, LootCard> = HashMap::new();

        for database_card in data {
            database.insert(database_card.id.clone(), database_card);
        }

        let mut deck: Vec<LootCard> = Vec::new();

        for card in database.values() {
            for _ in 0..card.count {
                deck.push(card.clone());
            }
        }

        Ok(Self {
            deck,
            discard_pile: Vec::new(),
            database,
        })
    }

    pub fn shuffle(&mut self) {
        let mut random_generator = rng();
        let mut deck = self.deck.clone();
        deck.shuffle(&mut random_generator);
        self.deck = deck;
    }

    pub fn draw_card(&mut self) -> Option<LootCard> {
        if self.deck.is_empty() {
            self.reshuffle_discard();
        }
        self.deck.pop()
    }

    pub fn draw_cards(&mut self, count: usize) -> Vec<LootCard> {
        let mut drawn = Vec::new();
        for _ in 0..count {
            if let Some(card) = self.draw_card() {
                drawn.push(card);
            } else {
                break;
            }
        }
        drawn
    }

    pub fn discard_card(&mut self, card: LootCard) {
        self.discard_pile.push(card);
    }

    pub fn discard_cards(&mut self, cards: Vec<LootCard>) {
        for card in cards {
            self.discard_card(card);
        }
    }

    fn reshuffle_discard(&mut self) {
        if !self.discard_pile.is_empty() {
            self.deck.append(&mut self.discard_pile);
            self.shuffle();
        }
    }

    pub fn peek_top(&self, count: usize) -> Vec<&LootCard> {
        self.deck.iter().rev().take(count).collect()
    }

    pub fn cards_remaining(&self) -> usize {
        self.deck.len()
    }

    pub fn discard_pile_size(&self) -> usize {
        self.discard_pile.len()
    }

    pub fn get_card_by_id(&self, id: &str) -> Option<&LootCard> {
        self.database.get(id)
    }

    pub fn total_deck_size(&self) -> usize {
        self.deck.len() + self.discard_pile.len()
    }
}
