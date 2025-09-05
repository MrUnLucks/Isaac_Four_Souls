use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct Room {
    id: String,
    name: String,
    players: HashMap<String, String>, // player_id -> player_name
    state: RoomState,
    max_players: usize,
    min_players: usize,
    players_ready: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RoomState {
    Lobby, // Waiting for players
    InGame,
}

impl Room {
    const DEFAULT_MAX_PLAYERS: usize = 4;
    const DEFAULT_MIN_PLAYERS: usize = 2;

    pub fn new(name: String) -> Self {
        Self {
            id: "5edf4e4d-354e-4a84-a2b1-1a1a1f197b9f".to_string(), // TEMPORARY FOR TESTING
            name,
            players: HashMap::new(), // Add-first-player handled in room_manager!
            players_ready: HashSet::new(),
            state: RoomState::Lobby,
            max_players: Self::DEFAULT_MAX_PLAYERS,
            min_players: Self::DEFAULT_MIN_PLAYERS,
        }
    }

    pub fn add_player(&mut self, player_name: String) -> AppResult<String> {
        if self.players.len() >= self.max_players {
            return Err(AppError::RoomFull {
                room_id: self.get_id(),
                max_players: self.max_players,
            });
        }
        if self.state != RoomState::Lobby {
            return Err(AppError::RoomInGame {
                room_id: self.get_id(),
            });
        }

        let new_player_id = Uuid::new_v4().to_string();
        self.players.insert(new_player_id.clone(), player_name);

        Ok(new_player_id)
    }

    pub fn remove_player(&mut self, player_id: &str) -> AppResult<String> {
        if self.state != RoomState::Lobby {
            return Err(AppError::RoomInGame {
                room_id: self.get_id(),
            });
        }
        let player_name = self
            .players
            .remove(player_id)
            .ok_or(AppError::ConnectionNotInRoom)?;
        self.players_ready.remove(player_id); // Always safe to call

        Ok(player_name)
    }

    pub fn add_player_ready(&mut self, player_id: &str) -> AppResult<HashSet<String>> {
        if !self.players.contains_key(player_id) {
            Err(AppError::ConnectionNotInRoom)
        } else if self.players_ready.contains(player_id) {
            Ok(self.players_ready.clone())
        } else {
            self.players_ready.insert(player_id.to_string());
            Ok(self.players_ready.clone())
        }
    }

    pub fn can_start_game(&self) -> bool {
        self.players_ready.len() == self.player_count() && self.state == RoomState::Lobby
    }

    pub fn get_room_info(&self) -> Self {
        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            players: self.players.clone(),
            min_players: self.min_players,
            max_players: self.max_players,
            state: self.state.clone(),
            players_ready: self.players_ready.clone(),
        }
    }
    pub fn set_state_in_game(&mut self) {
        self.state = RoomState::InGame;
    }
    pub fn player_count(&self) -> usize {
        self.players.len()
    }
    pub fn player_ready_count(&self) -> usize {
        self.players_ready.len()
    }
    pub fn get_id(&self) -> String {
        self.id.clone()
    }
    pub fn get_players_id(&self) -> Vec<String> {
        self.players.keys().cloned().collect()
    }
}
