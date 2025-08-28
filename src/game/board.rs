use rand::rng;
use rand::seq::SliceRandom;
use std::collections::HashMap;

use crate::game::card_loader::create_loot_deck;
use crate::game::cards_types::LootCard;

#[derive(Debug, Clone)]
pub struct Board {
    pub loot_deck: Vec<LootCard>,
    pub loot_discard: Vec<LootCard>,
    pub player_hands: HashMap<String, Vec<LootCard>>, // player_id -> hand
}

#[derive(Debug, Clone)]
pub enum BoardError {
    PlayerNotFound,
    EmptyLootDeck,
    CardNotInHand,
}

impl Board {
    pub fn new(player_ids: Vec<String>) -> Self {
        let mut loot_deck = create_loot_deck();
        let mut rng = rng();
        loot_deck.shuffle(&mut rng);

        // Initialize empty hands for all players
        let mut player_hands = HashMap::new();
        for player_id in player_ids {
            let mut card_drawn: Vec<LootCard> = Vec::new();
            for _ in 1..=3 {
                let card = loot_deck
                    .pop()
                    .expect("Full deck not enough for all players"); // Unreachable error on full deck
                card_drawn.push(card);
            }
            player_hands.insert(player_id, card_drawn);
        }

        Self {
            loot_deck,
            loot_discard: Vec::new(),
            player_hands,
        }
    }

    /// Draw one card from the loot deck for a specific player
    pub fn draw_loot_for_player(&mut self, player_id: &str) -> Result<LootCard, BoardError> {
        // Check if player exists
        if !self.player_hands.contains_key(player_id) {
            return Err(BoardError::PlayerNotFound);
        }

        // Check if deck is empty, reshuffle discard if needed
        if self.loot_deck.is_empty() {
            self.reshuffle_loot_deck()?;
        }

        // Draw card and add to player's hand
        let drawn_card = self.loot_deck.pop().ok_or(BoardError::EmptyLootDeck)?;

        self.player_hands
            .get_mut(player_id)
            .ok_or(BoardError::PlayerNotFound)?
            .push(drawn_card.clone());

        println!("🃏 Player {} drew: {}", player_id, drawn_card.name);
        Ok(drawn_card)
    }

    /// Get a player's hand (read-only)
    pub fn get_player_hand(&self, player_id: &str) -> Result<&Vec<LootCard>, BoardError> {
        self.player_hands
            .get(player_id)
            .ok_or(BoardError::PlayerNotFound)
    }

    /// Get hand size for a player
    pub fn get_hand_size(&self, player_id: &str) -> Result<usize, BoardError> {
        Ok(self.get_player_hand(player_id)?.len())
    }

    /// Remove a card from a player's hand (for playing cards)
    pub fn remove_card_from_hand(
        &mut self,
        player_id: &str,
        card_id: &str,
    ) -> Result<LootCard, BoardError> {
        let hand = self
            .player_hands
            .get_mut(player_id)
            .ok_or(BoardError::PlayerNotFound)?;

        if let Some(pos) = hand.iter().position(|card| card.template_id == card_id) {
            Ok(hand.remove(pos))
        } else {
            Err(BoardError::CardNotInHand)
        }
    }

    /// Add a card to the loot discard pile
    pub fn discard_loot_card(&mut self, card: LootCard) {
        println!("🗑️ Discarding loot card: {}", card.name);
        self.loot_discard.push(card);
    }

    /// Reshuffle the discard pile back into the deck
    fn reshuffle_loot_deck(&mut self) -> Result<(), BoardError> {
        if self.loot_discard.is_empty() && self.loot_deck.is_empty() {
            return Err(BoardError::EmptyLootDeck);
        }

        if !self.loot_discard.is_empty() {
            println!("🔄 Reshuffling loot discard pile into deck");
            self.loot_deck.append(&mut self.loot_discard);

            let mut rng = rng();
            self.loot_deck.shuffle(&mut rng);
        }

        Ok(())
    }
}
