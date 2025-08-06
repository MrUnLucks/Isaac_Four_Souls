use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use uuid::Uuid;

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
            id: Uuid::new_v4().to_string(),
            name,
            players: HashMap::new(), // Add-first-player handled in room_manager!
            players_ready: HashSet::new(),
            state: RoomState::Lobby,
            max_players: Self::DEFAULT_MAX_PLAYERS,
            min_players: Self::DEFAULT_MIN_PLAYERS,
        }
    }

    pub fn add_player(&mut self, player_name: String) -> Result<String, RoomError> {
        if self.players.len() >= self.max_players {
            return Err(RoomError::RoomFull);
        }
        if self.state != RoomState::Lobby {
            return Err(RoomError::RoomInGame);
        }

        let new_player_id = Uuid::new_v4().to_string();
        self.players.insert(new_player_id.clone(), player_name);

        Ok(new_player_id)
    }

    pub fn remove_player(&mut self, player_id: &str) -> Result<String, RoomError> {
        if self.state != RoomState::Lobby {
            return Err(RoomError::RoomInGame);
        }
        let player_name = self
            .players
            .remove(player_id)
            .ok_or(RoomError::PlayerNotInRoom)?;
        self.players_ready.remove(player_id); // Always safe to call

        Ok(player_name)
    }

    pub fn can_start_game(&self) -> bool {
        self.players_ready.len() == self.player_count() && self.state == RoomState::Lobby
    }

    pub fn start_game(&mut self) -> Result<(), RoomError> {
        if self.can_start_game() {
            self.state = RoomState::InGame;
            self.players_ready = HashSet::new();
            Ok(())
        } else {
            Err(RoomError::PlayersNotReady)
        }
    }

    pub fn add_player_ready(&mut self, player_id: &str) -> Result<HashSet<String>, RoomError> {
        if !self.players.contains_key(player_id) {
            Err(RoomError::PlayerNotInRoom)
        } else if self.players_ready.contains(player_id) {
            Ok(self.players_ready.clone())
        } else {
            self.players_ready.insert(player_id.to_string());
            Ok(self.players_ready.clone())
        }
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
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
    pub fn get_id(&self) -> String {
        self.id.clone()
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

#[derive(Debug, PartialEq, Serialize)]
pub enum RoomError {
    RoomFull,
    RoomInGame,
    PlayerNotInRoom,
    PlayersNotReady,
    PlayerAlreadyInRoom,
    RoomNotFound,
}
impl fmt::Display for RoomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RoomError::RoomFull => write!(f, "Room is full"),
            RoomError::RoomInGame => write!(f, "Room is currently in game"),
            RoomError::PlayerNotInRoom => write!(f, "Player not found"),
            RoomError::PlayersNotReady => write!(f, "Players are not ready"),
            RoomError::PlayerAlreadyInRoom => write!(f, "Player already in room"),
            RoomError::RoomNotFound => write!(f, "Room not found"),
        }
    }
}
