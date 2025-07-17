use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub players: HashMap<String, String>, // player_id -> player_name
    pub state: RoomState,
    pub max_players: usize,
    pub min_players: usize,
    pub players_ready: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoomState {
    Lobby, // Waiting for players
    Starting,
    InGame,
    Finished,
}

impl Room {
    const DEFAULT_MAX_PLAYERS: usize = 4;
    const DEFAULT_MIN_PLAYERS: usize = 2;

    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            players: HashMap::new(), // Add first handled in room_manager!
            players_ready: HashSet::new(),
            state: RoomState::Lobby,
            max_players: Self::DEFAULT_MAX_PLAYERS,
            min_players: Self::DEFAULT_MIN_PLAYERS,
        }
    }

    pub fn add_player(&mut self, player_name: String) -> Result<String, String> {
        if self.players.len() >= self.max_players {
            return Err("Room is full".to_string());
        }

        let new_player_id = Uuid::new_v4().to_string();
        self.players.insert(new_player_id.clone(), player_name);

        Ok(new_player_id)
    }

    pub fn remove_player(&mut self, player_id: &str) -> Result<String, String> {
        let player_name = self
            .players
            .remove(player_id)
            .ok_or("Cannot find player to remove".to_string())?;
        self.players_ready.remove(player_id); // Always safe to call

        Ok(player_name)
    }

    pub fn can_start_game(&self) -> bool {
        self.players_ready.len() == self.player_count()
    }

    pub fn start_game(&mut self) -> Result<(), String> {
        if self.can_start_game() {
            self.state = RoomState::InGame;
            self.players_ready = HashSet::new();
            Ok(())
        } else {
            Err("Cannot start game".to_string())
        }
    }

    pub fn add_player_ready(&mut self, player_id: &str) -> Result<HashSet<String>, String> {
        if !self.players.contains_key(player_id) {
            Err("Player not in the room".to_string())
        } else if self.players_ready.contains(player_id) {
            Err("Player already ready".to_string())
        } else {
            self.players_ready.insert(player_id.to_string());
            Ok(self.players_ready.clone())
        }
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn get_room_info(&self) -> RoomInfo {
        RoomInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            player_count: self.players.len(),
            max_players: self.max_players,
            state: self.state.clone(),
        }
    }
    pub fn get_player(&self, player_id: &str) -> Option<&String> {
        self.players.get(player_id)
    }
    pub fn get_players_id(&self) -> Vec<String> {
        self.players.keys().cloned().collect()
    }
    pub fn handle_action(player_id: String, action: String) {
        println!("TODO! player:{}, action:{}", player_id, action)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomInfo {
    pub id: String,
    pub name: String,
    pub player_count: usize,
    pub max_players: usize,
    pub state: RoomState,
}
