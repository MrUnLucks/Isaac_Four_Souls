use crate::Player;
use std::collections::HashMap;
pub struct PlayerManager {
    players: HashMap<String, Player>,
}
impl PlayerManager {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
        }
    }
    pub fn add_player(&mut self, player: Player) -> Result<(), String> {
        let id = player.id.clone(); // Clone once
        if self.players.contains_key(&id) {
            Err("ID already exists".to_string())
        } else {
            self.players.insert(id, player);
            Ok(())
        }
    }
    pub fn get_player(&self, id: &str) -> Option<&Player> {
        self.players.get(id)
    }
    pub fn remove_player(&mut self, id: &str) -> Option<Player> {
        self.players.remove(id)
    }
    pub fn list_connected_players(&self) -> Vec<&Player> {
        self.players
            .values()
            .filter(|value| value.is_connected)
            .collect()
    }
    pub fn disconnect_player(&mut self, id: &str) -> Result<(), String> {
        match self.players.get_mut(id) {
            Some(player) => {
                player.disconnect();
                Ok(())
            }
            None => Err("Player not found".to_string()),
        }
    }
    pub fn player_count(&self) -> usize {
        self.players.len()
    }
    pub fn connected_count(&self) -> usize {
        self.players.values().filter(|p| p.is_connected).count()
    }
}
