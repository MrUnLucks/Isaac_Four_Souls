use serde::Serialize;

use crate::{game::room::RoomError, Room};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct PlayerRoomInfo {
    pub room_id: String,
    pub room_player_id: String,
    pub player_name: String,
}

pub struct RoomManager {
    rooms: HashMap<String, Room>,
    pub connection_to_room_info: HashMap<String, PlayerRoomInfo>, // connection_id -> room info
}

#[derive(Debug)]
pub struct ReadyPlayerResult {
    pub players_ready: HashSet<String>,
    pub game_started: bool,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            connection_to_room_info: HashMap::new(),
        }
    }

    pub fn create_room(
        &mut self,
        room_name: String,
        first_player_connection_id: String,
        first_player_name: String,
    ) -> Result<(String, String), RoomManagerError> {
        if room_name.trim().is_empty() {
            return Err(RoomManagerError::RoomNameInvalid); // Frontend form handling preferably
        }
        if self
            .connection_to_room_info
            .contains_key(&first_player_connection_id)
        {
            return Err(RoomManagerError::PlayerInDifferentRoom);
        }

        let mut room = Room::new(room_name);
        let new_player_id = room.add_player(first_player_name.clone())?;
        let room_id = room.get_id();

        self.connection_to_room_info.insert(
            first_player_connection_id,
            PlayerRoomInfo {
                room_id: room_id.clone(),
                room_player_id: new_player_id.clone(),
                player_name: first_player_name.clone(),
            },
        );
        self.rooms.insert(room_id.clone(), room);

        Ok((room_id, new_player_id))
    }

    pub fn join_room(
        &mut self,
        room_id: &str,
        connection_id: String,
        player_name: String,
    ) -> Result<String, RoomManagerError> {
        if self.connection_to_room_info.contains_key(&connection_id) {
            return Err(RoomManagerError::PlayerInDifferentRoom);
        }

        let room = self
            .rooms
            .get_mut(room_id)
            .ok_or(RoomManagerError::RoomError(RoomError::RoomNotFound))?;
        let new_player_id = room.add_player(player_name.clone())?;
        self.connection_to_room_info.insert(
            connection_id,
            PlayerRoomInfo {
                room_id: room_id.to_string(),
                room_player_id: new_player_id.clone(),
                player_name: player_name.clone(),
            },
        );
        Ok(new_player_id)
    }

    // Return player name to broadcast it
    pub fn leave_room(&mut self, connection_id: &str) -> Result<String, RoomManagerError> {
        let PlayerRoomInfo {
            room_id,
            room_player_id,
            player_name: _,
        } = self
            .connection_to_room_info
            .remove(connection_id)
            .ok_or_else(|| RoomManagerError::RoomError(RoomError::PlayerNotInRoom))?;

        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or_else(|| RoomManagerError::RoomError(RoomError::RoomNotFound))?;

        let removed_player_name = room.remove_player(&room_player_id)?;

        if room.player_count() == 0 {
            self.rooms.remove(&room_id);
        }

        Ok(removed_player_name)
    }

    pub fn get_room_mut(&mut self, room_id: &str) -> Option<&mut Room> {
        self.rooms.get_mut(room_id)
    }

    pub fn destroy_room(
        &mut self,
        room_id: &str,
        connection_id: &str,
    ) -> Result<(), RoomManagerError> {
        self.connection_to_room_info
            .remove(connection_id)
            .ok_or_else(|| RoomManagerError::RoomError(RoomError::PlayerNotInRoom))?;

        self.rooms
            .remove(room_id)
            .map(|_| ())
            .ok_or_else(|| RoomManagerError::RoomError(RoomError::RoomNotFound))
    }

    pub fn ready_player(&mut self, player_id: &str) -> Result<ReadyPlayerResult, RoomManagerError> {
        let room_id = Self::get_player_room_from_player_id(self, player_id)
            .ok_or_else(|| RoomManagerError::RoomError(RoomError::PlayerNotInRoom))?;

        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or(RoomManagerError::RoomError(RoomError::RoomNotFound))?;

        let players_ready = room.add_player_ready(player_id)?;

        let game_started = if room.can_start_game() {
            room.start_game()?;
            true
        } else {
            false
        };

        Ok(ReadyPlayerResult {
            players_ready,
            game_started,
        })
    }

    pub fn get_player_room_from_player_id(&self, player_id: &str) -> Option<String> {
        self.connection_to_room_info
            .values()
            .find(|info| info.room_player_id == player_id)
            .map(|info| info.room_id.clone())
    }

    pub fn get_player_room_from_connection_id(&self, connection_id: &str) -> Option<String> {
        self.connection_to_room_info
            .get(connection_id)
            .map(|player| player.room_id.clone())
    }

    pub fn get_player_list(&self, room_id: &str) -> Option<Vec<String>> {
        self.rooms.get(room_id).map(|x| x.get_players_id())
    }
}

#[derive(Debug, Serialize)]
pub enum RoomManagerError {
    RoomNameInvalid,
    PlayerInDifferentRoom,
    RoomError(RoomError),
}
impl From<RoomError> for RoomManagerError {
    fn from(err: RoomError) -> Self {
        RoomManagerError::RoomError(err)
    }
}
