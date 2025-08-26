use std::collections::HashMap;

use tokio::sync::mpsc;

use crate::game::card_loader::create_loot_deck;
use crate::game::components::*;
use crate::game::entity::EntityId;
use crate::game::systems::Board;
use crate::network::messages::{serialize_response, ServerResponse};
use crate::{ConnectionCommand, TurnOrder};

pub struct GameLoop {
    board: Board,
    turn_order: TurnOrder,
    players_id_to_connection_id: HashMap<String, String>,
    room_connections_id: Vec<String>,

    loot_deck_entity: EntityId,
    loot_discard_entity: EntityId,
    player_entities: HashMap<String, EntityId>, // player_id -> entity
}

pub enum GameEvent {
    TurnPass { player_id: String },
    GameOver { winner_id: String },
}

#[derive(Debug, Clone)]
pub enum GameError {
    GameEndedUnexpectedly,
    NotPlayerTurn,
}

impl GameLoop {
    pub fn new(
        players_id_to_connection_id: HashMap<String, String>,
        turn_order: TurnOrder,
    ) -> Self {
        let mut board = Board::new();

        // Create deck and discard pile entities
        let loot_deck_entity = board.entity_manager.create_entity();
        let loot_discard_entity = board.entity_manager.create_entity();

        board.components.decks.insert(
            loot_deck_entity,
            DeckComponent {
                deck_type: DeckType::Loot,
            },
        );
        board.components.discard_piles.insert(
            loot_discard_entity,
            DiscardPileComponent {
                pile_type: DeckType::Loot,
            },
        );

        println!("🎮 Created loot deck entity: {}", loot_deck_entity);
        println!("🗑️ Created discard pile entity: {}", loot_discard_entity);

        // Create all the loot cards as entities
        let loot_cards = create_loot_deck();
        println!("🃏 Creating {} loot card entities", loot_cards.len());

        for card in loot_cards {
            let card_entity = board.entity_manager.create_entity();
            board.add_card(
                card_entity,
                CardComponent {
                    card_data: card.clone(),
                },
            );
            board.add_in_deck(card_entity);
        }

        let mut player_entities = HashMap::new();
        for (player_id, connection_id) in &players_id_to_connection_id {
            let player_entity = board.entity_manager.create_entity();

            board.components.players.insert(
                player_entity.clone(),
                PlayerComponent {
                    player_id: player_id.clone(),
                    name: format!("Player {}", player_id), // Might use names
                    connection_id: connection_id.clone(),
                },
            );

            board
                .components
                .resources
                .insert(player_entity, ResourcesComponent::new(2));

            player_entities.insert(player_id.clone(), player_entity);
            println!(
                "👤 Created player entity for {}: {}",
                player_id, player_entity
            );
        }

        let (deck_count, discard_count) = board.get_pile_counts();
        println!(
            "🚀 Game setup complete - Deck: {}, Discard: {}, Total entities: {}",
            deck_count,
            discard_count,
            board.entity_manager.entity_count()
        );

        let room_connections_id = players_id_to_connection_id
            .values()
            .cloned()
            .into_iter()
            .collect();

        Self {
            board,
            turn_order,
            loot_deck_entity,
            loot_discard_entity,
            player_entities,
            players_id_to_connection_id,
            room_connections_id,
        }
    }

    // Draw cards for a player
    pub fn draw_loot_cards(
        &mut self,
        player_id: &str,
        count: usize,
    ) -> Vec<(EntityId, CardComponent)> {
        let mut drawn_cards = Vec::new();

        for _ in 0..count {
            if let Some((entity, card)) = self.board.draw_card_from_deck(player_id.to_string()) {
                drawn_cards.push((entity, card));
            } else {
                break; // No more cards available
            }
        }

        drawn_cards
    }

    // Discard a card from player's hand
    pub fn discard_card(&mut self, card_entity: EntityId) -> bool {
        self.board.move_card_hand_to_discard(card_entity)
    }

    // Debug method to print all card locations
    pub fn debug_print_card_locations(&self) {
        println!("🔍 Card Locations:");

        let deck_cards = self.board.get_cards_in_deck();
        println!("  📦 Deck ({} cards):", deck_cards.len());
        for (entity, card) in deck_cards {
            println!("    - {} ({})", card.card_data.name, entity);
        }

        let discard_cards = self.board.get_cards_in_discard();
        println!("  🗑️ Discard ({} cards):", discard_cards.len());
        for (entity, card) in discard_cards {
            println!("    - {} ({})", card.card_data.name, entity);
        }

        for (player_id, _) in &self.player_entities {
            let hand_cards = self.board.get_player_hand(player_id);
            println!(
                "  ✋ Player {} Hand ({} cards):",
                player_id,
                hand_cards.len()
            );
            for (entity, card) in hand_cards {
                println!("    - {} ({})", card.card_data.name, entity);
            }
        }
    }

    pub async fn run(
        &mut self,
        mut event_receiver: mpsc::Receiver<GameEvent>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) {
        while let Some(event) = event_receiver.recv().await {
            match event {
                GameEvent::TurnPass { player_id } => {
                    if self.turn_order.is_player_turn(&player_id) {
                        let drawn_cards = self.draw_loot_cards(&player_id, 2);
                        println!("Player {} drew {} cards", player_id, drawn_cards.len());

                        for (entity, card) in &drawn_cards {
                            println!("  🎯 Drew: {} ({})", card.card_data.name, entity);
                        }

                        // For demo: immediately discard the first card
                        if let Some((entity, card)) = drawn_cards.first() {
                            if self.discard_card(*entity) {
                                println!("  🗑️ Discarded: {}", card.card_data.name);
                            }
                        }

                        let next_player = self.turn_order.advance_turn();
                        println!("Turn passed to: {}", next_player);

                        if self.turn_order.get_turn_counter() >= 4 {
                            let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
                                connections_id: self.room_connections_id.clone(),
                                message: serialize_response(ServerResponse::GameEnded {
                                    winner_id: player_id,
                                }),
                            });
                        }

                        let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
                            connections_id: self.room_connections_id.clone(),
                            message: serialize_response(ServerResponse::TurnChange {
                                next_player_id: next_player,
                            }),
                        });
                    } else {
                        //TODO: ERROR HANDLING FOR ALL GAMEERRORS
                        let player_connection_id = self
                            .players_id_to_connection_id
                            .get(&player_id)
                            .expect("NEED HANDLING")
                            .clone();
                        let _ = cmd_sender.send(ConnectionCommand::SendToPlayer {
                            connection_id: player_connection_id,
                            message: serialize_response(ServerResponse::from_app_error(
                                &crate::AppError::NotPlayerTurn,
                            )),
                        });
                    }
                }
                GameEvent::GameOver { winner_id } => {
                    let _ = cmd_sender.send(ConnectionCommand::SendToPlayers {
                        connections_id: self.room_connections_id.clone(),
                        message: serialize_response(ServerResponse::GameEnded { winner_id }),
                    });
                }
            }
        }
    }
}
