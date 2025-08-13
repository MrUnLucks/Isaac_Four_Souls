use crate::game::card_loader::{create_loot_deck, LootCard};
use rand::rng;
use rand::seq::SliceRandom;

#[derive(Debug, Clone)]
pub struct LootDeck {
    deck: Vec<LootCard>,
    discard_pile: Vec<LootCard>,
}
impl LootDeck {
    pub fn new() -> Self {
        let deck = create_loot_deck();

        Self {
            deck,
            discard_pile: Vec::new(),
        }
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

    pub fn total_deck_size(&self) -> usize {
        self.deck.len() + self.discard_pile.len()
    }
}
