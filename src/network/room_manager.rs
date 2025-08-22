use std::collections::{HashMap, HashSet};

use crate::{AppError, AppResult, Room};

#[derive(Debug, Clone)]
pub struct PlayerRoomInfo {
    pub room_id: String,
    pub room_player_id: String,
    pub player_name: String,
}

pub struct RoomManager {
    pub rooms: HashMap<String, Room>,
    pub connection_to_room_info: HashMap<String, PlayerRoomInfo>, // connection_id -> room info
    pub rooms_connections_map: HashMap<String, HashSet<String>>, // room_id -> HashSet<connection_id>
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
            rooms_connections_map: HashMap::new(),
        }
    }

    pub fn create_room(
        &mut self,
        room_name: String,
        first_player_connection_id: String,
        first_player_name: String,
    ) -> AppResult<(String, String)> {
        if room_name.trim().is_empty() {
            return Err(AppError::RoomNameEmpty);
        }
        if self
            .connection_to_room_info
            .contains_key(&first_player_connection_id)
        {
            return Err(AppError::PlayerAlreadyInRoom {
                player_name: first_player_name,
            });
        }

        let mut room = Room::new(room_name);
        let new_player_id = room.add_player(first_player_name.clone())?;
        let room_id = room.get_id();

        self.connection_to_room_info.insert(
            first_player_connection_id.clone(),
            PlayerRoomInfo {
                room_id: room_id.clone(),
                room_player_id: new_player_id.clone(),
                player_name: first_player_name.clone(),
            },
        );
        self.rooms_connections_map
            .insert(room_id.clone(), HashSet::from([first_player_connection_id]));
        self.rooms.insert(room_id.clone(), room);

        Ok((room_id, new_player_id))
    }

    pub fn join_room(
        &mut self,
        room_id: &str,
        connection_id: String,
        player_name: String,
    ) -> AppResult<String> {
        if self.connection_to_room_info.contains_key(&connection_id) {
            return Err(AppError::PlayerAlreadyInRoom { player_name });
        }

        let room = self.rooms.get_mut(room_id).ok_or(AppError::RoomNotFound {
            room_id: room_id.to_string(),
        })?;
        let new_player_id = room.add_player(player_name.clone())?;
        self.connection_to_room_info.insert(
            connection_id.clone(),
            PlayerRoomInfo {
                room_id: room_id.to_string(),
                room_player_id: new_player_id.clone(),
                player_name: player_name.clone(),
            },
        );
        self.rooms_connections_map
            .entry(room_id.to_string())
            .or_insert_with(HashSet::new)
            .insert(connection_id);
        Ok(new_player_id)
    }

    // Return player name to broadcast it
    pub fn leave_room(&mut self, connection_id: &str) -> AppResult<String> {
        let PlayerRoomInfo {
            room_id,
            room_player_id,
            player_name: _,
        } = self
            .connection_to_room_info
            .remove(connection_id)
            .ok_or_else(|| AppError::ConnectionNotInRoom)?;

        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or_else(|| AppError::RoomNotFound {
                room_id: room_id.clone(),
            })?;

        let connection_set = self
            .rooms_connections_map
            .get_mut(&room_id.to_string())
            .ok_or_else(|| AppError::RoomNotFound {
                room_id: room_id.clone(),
            })?;
        connection_set.remove(connection_id); // Safe to call
        let removed_player_name = room.remove_player(&room_player_id)?;

        if room.player_count() == 0 {
            self.rooms.remove(&room_id);
        }

        Ok(removed_player_name)
    }

    pub fn destroy_room(&mut self, room_id: &str, connection_id: &str) -> AppResult<String> {
        self.connection_to_room_info
            .remove(connection_id)
            .ok_or_else(|| AppError::ConnectionNotInRoom)?;

        let connection_set = self
            .rooms_connections_map
            .get_mut(&room_id.to_string())
            .ok_or_else(|| AppError::RoomNotFound {
                room_id: room_id.to_string(),
            })?;
        connection_set.remove(connection_id);

        self.rooms
            .remove(room_id)
            .ok_or_else(|| AppError::RoomNotFound {
                room_id: room_id.to_string(),
            })?;

        Ok(room_id.to_string())
    }

    pub fn ready_player(&mut self, player_id: &str) -> AppResult<ReadyPlayerResult> {
        let room_id = Self::get_player_room_from_player_id(self, player_id)?;

        let room = self.rooms.get_mut(&room_id).ok_or(AppError::RoomNotFound {
            room_id: room_id.clone(),
        })?;

        let players_ready = room.add_player_ready(player_id)?;

        Ok(ReadyPlayerResult {
            players_ready,
            game_started: room.can_start_game(),
        })
    }

    pub fn get_players_mapping(&self, room_id: &str) -> AppResult<HashMap<String, String>> {
        let mut players_mapping = HashMap::new();

        for (connection_id, player_info) in &self.connection_to_room_info {
            if player_info.room_id == *room_id {
                players_mapping.insert(player_info.room_player_id.clone(), connection_id.clone());
            }
        }

        if players_mapping.is_empty() {
            Err(AppError::RoomNotFound {
                room_id: room_id.to_string(),
            })
        } else {
            Ok(players_mapping)
        }
    }

    pub fn get_room_mut(&mut self, room_id: &str) -> Option<&mut Room> {
        self.rooms.get_mut(room_id)
    }

    pub fn get_player_id_from_connection_id(&self, connection_id: &str) -> AppResult<String> {
        self.connection_to_room_info
            .get(connection_id)
            .ok_or_else(|| AppError::ConnectionNotInRoom)
            .map(|player| player.room_player_id.clone())
    }

    pub fn get_player_room_from_player_id(&self, player_id: &str) -> AppResult<String> {
        self.connection_to_room_info
            .values()
            .find(|info| info.room_player_id == player_id)
            .map(|info| info.room_id.clone())
            .ok_or_else(|| AppError::ConnectionNotInRoom)
    }

    pub fn get_player_room_from_connection_id(&self, connection_id: &str) -> Option<String> {
        self.connection_to_room_info
            .get(connection_id)
            .map(|player| player.room_id.clone())
    }

    pub fn get_player_name_from_connection_id(&self, connection_id: &str) -> Option<String> {
        self.connection_to_room_info
            .get(connection_id)
            .map(|player| player.player_name.clone())
    }

    pub fn get_connections_id_from_room_id(&self, room_id: &str) -> AppResult<Vec<String>> {
        self.rooms_connections_map
            .get(room_id)
            .ok_or_else(|| AppError::RoomNotFound {
                room_id: room_id.to_string(),
            })
            .map(|connections| connections.iter().cloned().collect())
    }

    pub fn get_connections_id_from_room_id_except_player(
        &self,
        room_id: &str,
        connection_id: &str,
    ) -> AppResult<Vec<String>> {
        self.rooms_connections_map
            .get(room_id)
            .ok_or_else(|| AppError::ConnectionNotInRoom)
            .map(|connections| {
                connections
                    .iter()
                    .filter(|&conn| conn != connection_id)
                    .cloned()
                    .collect()
            })
    }

    pub fn get_player_list(&self, room_id: &str) -> Option<Vec<String>> {
        self.rooms.get(room_id).map(|x| x.get_players_id())
    }
}
