use rand::rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::game::card_loader::create_loot_deck;
use crate::game::cards_types::LootCard;
use crate::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub hand: Vec<LootCard>,
    // pub items:
    pub max_health: u32,
    pub current_health: u32,
    pub loot_play_turn: bool,
    pub loot_play_char: bool,
}

impl Player {
    pub fn new(
        hand: Vec<LootCard>,
        max_health: u32,
        current_health: u32,
        loot_play_turn: bool,
        loot_play_char: bool,
    ) -> Self {
        Self {
            current_health,
            hand,
            loot_play_char,
            loot_play_turn,
            max_health,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Board {
    pub loot_deck: Vec<LootCard>,
    pub loot_discard: Vec<LootCard>,
    pub players: HashMap<String, Player>,
}

impl Board {
    pub fn new(player_ids: Vec<String>) -> Self {
        let mut loot_deck = create_loot_deck();
        let mut rng = rng();
        loot_deck.shuffle(&mut rng);

        let mut players: HashMap<String, Player> = HashMap::new();
        for player_id in player_ids {
            let mut card_drawn: Vec<LootCard> = Vec::new();
            for _ in 1..=3 {
                let card = loot_deck
                    .pop()
                    .expect("Full deck not enough for all players"); // Unreachable error on full deck
                card_drawn.push(card);
            }
            // Characters with different healths defined here
            let player: Player = Player::new(card_drawn, 2, 2, true, true);
            players.insert(player_id, player);
        }

        Self {
            loot_deck,
            loot_discard: Vec::new(),
            players,
        }
    }

    /// Draw one card from the loot deck for a specific player
    pub fn draw_loot_for_player(&mut self, player_id: &str) -> AppResult<LootCard> {
        // Check if player exists
        if !self.players.contains_key(player_id) {
            return Err(AppError::PlayerNotFound);
        }

        // Check if deck is empty, reshuffle discard if needed
        if self.loot_deck.is_empty() {
            self.reshuffle_loot_deck()?;
        }

        // Draw card and add to player's hand
        let drawn_card = self.loot_deck.pop().ok_or(AppError::EmptyLootDeck)?;

        self.players
            .get_mut(player_id)
            .ok_or(AppError::PlayerNotFound)?
            .hand
            .push(drawn_card.clone());

        println!("🃏 Player {} drew: {}", player_id, drawn_card.name);
        Ok(drawn_card)
    }

    /// Get a player's hand (read-only)
    pub fn get_player_hand(&self, player_id: &str) -> AppResult<Vec<LootCard>> {
        let player_hand = self
            .players
            .get(player_id)
            .ok_or(AppError::PlayerNotFound)?
            .hand
            .clone();
        Ok(player_hand)
    }

    /// Get hand size for a player
    pub fn get_hand_size(&self, player_id: &str) -> AppResult<usize> {
        Ok(self.get_player_hand(player_id)?.len())
    }

    /// Remove a card from a player's hand (for playing cards)
    pub fn remove_card_from_hand(&mut self, player_id: &str, card_id: &str) -> AppResult<LootCard> {
        let mut hand = self
            .players
            .get(player_id)
            .ok_or(AppError::PlayerNotFound)?
            .hand
            .clone();

        if let Some(pos) = hand.iter().position(|card| card.template_id == card_id) {
            Ok(hand.remove(pos))
        } else {
            Err(AppError::CardNotInHand)
        }
    }

    /// Add a card to the loot discard pile
    pub fn discard_loot_card(&mut self, card: LootCard) {
        println!("🗑️ Discarding loot card: {}", card.name);
        self.loot_discard.push(card);
    }

    /// Reshuffle the discard pile back into the deck
    fn reshuffle_loot_deck(&mut self) -> AppResult<()> {
        if self.loot_discard.is_empty() && self.loot_deck.is_empty() {
            return Err(AppError::EmptyLootDeck);
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
