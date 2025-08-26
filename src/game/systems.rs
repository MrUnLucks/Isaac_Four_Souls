use crate::game::components::*;
use crate::game::entity::{EntityId, EntityManager};
use rand::rng;
use rand::seq::SliceRandom;
use std::collections::HashMap;

pub struct Board {
    pub entity_manager: EntityManager,
    pub components: ComponentStorage,
}

pub struct ComponentStorage {
    pub cards: HashMap<EntityId, CardComponent>,
    pub in_deck: HashMap<EntityId, InDeckComponent>,
    pub in_discard: HashMap<EntityId, InDiscardPileComponent>,
    pub in_hand: HashMap<EntityId, InHandComponent>,
    pub decks: HashMap<EntityId, DeckComponent>,
    pub discard_piles: HashMap<EntityId, DiscardPileComponent>,
    pub players: HashMap<EntityId, PlayerComponent>,
    pub resources: HashMap<EntityId, ResourcesComponent>,
}

impl ComponentStorage {
    pub fn new() -> Self {
        Self {
            cards: HashMap::new(),
            in_deck: HashMap::new(),
            in_discard: HashMap::new(),
            in_hand: HashMap::new(),
            decks: HashMap::new(),
            discard_piles: HashMap::new(),
            players: HashMap::new(),
            resources: HashMap::new(),
        }
    }
}

impl Board {
    pub fn new() -> Self {
        Self {
            entity_manager: EntityManager::new(),
            components: ComponentStorage::new(),
        }
    }

    pub fn add_card(&mut self, entity: EntityId, card: CardComponent) {
        self.components.cards.insert(entity, card);
    }

    pub fn add_in_deck(&mut self, entity: EntityId) {
        self.components.in_deck.insert(entity, InDeckComponent);
    }

    pub fn add_in_discard(&mut self, entity: EntityId) {
        self.components
            .in_discard
            .insert(entity, InDiscardPileComponent);
    }

    pub fn add_in_hand(&mut self, entity: EntityId, player_id: String) {
        self.components
            .in_hand
            .insert(entity, InHandComponent { player_id });
    }

    pub fn get_cards_in_deck(&self) -> Vec<(EntityId, &CardComponent)> {
        self.components
            .in_deck
            .keys()
            .filter_map(|&entity| {
                self.components
                    .cards
                    .get(&entity)
                    .map(|card| (entity, card))
            })
            .collect()
    }

    pub fn get_cards_in_discard(&self) -> Vec<(EntityId, &CardComponent)> {
        self.components
            .in_discard
            .keys()
            .filter_map(|&entity| {
                self.components
                    .cards
                    .get(&entity)
                    .map(|card| (entity, card))
            })
            .collect()
    }

    pub fn get_player_hand(&self, player_id: &str) -> Vec<(EntityId, &CardComponent)> {
        self.components
            .in_hand
            .iter()
            .filter(|(_, hand_comp)| hand_comp.player_id == player_id)
            .filter_map(|(&entity, _)| {
                self.components
                    .cards
                    .get(&entity)
                    .map(|card| (entity, card))
            })
            .collect()
    }

    pub fn move_card_deck_to_hand(&mut self, card_entity: EntityId, player_id: String) -> bool {
        if self.components.in_deck.remove(&card_entity).is_some() {
            self.add_in_hand(card_entity, player_id.clone());
            println!(
                "🎯 Moved card {} from deck to player {}'s hand",
                card_entity, player_id
            );
            true
        } else {
            false
        }
    }

    pub fn move_card_hand_to_discard(&mut self, card_entity: EntityId) -> bool {
        if self.components.in_hand.remove(&card_entity).is_some() {
            self.add_in_discard(card_entity);
            println!("🗑️ Moved card {} from hand to discard pile", card_entity);
            true
        } else {
            false
        }
    }

    pub fn reshuffle_discard_to_deck(&mut self) {
        let discard_entities: Vec<EntityId> = self.components.in_discard.keys().copied().collect();

        if !discard_entities.is_empty() {
            println!(
                "🔄 Reshuffling {} cards from discard to deck",
                discard_entities.len()
            );

            for entity in discard_entities {
                self.components.in_discard.remove(&entity);
                self.add_in_deck(entity);
            }

            println!("✅ Reshuffle complete");
        }
    }

    fn get_random_deck_card_entity(&mut self) -> Option<EntityId> {
        let mut deck_entity_ids: Vec<EntityId> = self.components.in_deck.keys().copied().collect();

        if deck_entity_ids.is_empty() {
            self.reshuffle_discard_to_deck();
            deck_entity_ids = self.components.in_deck.keys().copied().collect();
        }

        if !deck_entity_ids.is_empty() {
            let mut random_generator = rng();
            deck_entity_ids.shuffle(&mut random_generator);
            Some(deck_entity_ids[0])
        } else {
            None
        }
    }

    pub fn draw_card_from_deck(&mut self, player_id: String) -> Option<(EntityId, CardComponent)> {
        // Get a random card entity
        let card_entity = self.get_random_deck_card_entity()?;

        // Get the card component data
        let card_component = self.components.cards.get(&card_entity)?.clone();

        // Move the card to player's hand
        if self.move_card_deck_to_hand(card_entity, player_id) {
            Some((card_entity, card_component))
        } else {
            None
        }
    }

    pub fn get_pile_counts(&self) -> (usize, usize) {
        (
            self.components.in_deck.len(),
            self.components.in_discard.len(),
        )
    }
}
