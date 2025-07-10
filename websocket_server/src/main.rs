use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

#[derive(Debug)]
struct Player {
    id: String,
    name: String,
    is_connected: bool,
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Name: {} ({}), is_connected: {}",
            self.name, self.id, self.is_connected
        )
    }
}
impl Player {
    fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            is_connected: true,
        }
    }
    fn disconnect(&mut self) {
        self.is_connected = false;
    }
}

struct PlayerManager {
    players: HashMap<String, Player>,
}
impl PlayerManager {
    fn new() -> Self {
        Self {
            players: HashMap::new(),
        }
    }
    fn add_player(&mut self, player: Player) -> Result<(), String> {
        let id = player.id.clone(); // Clone once
        if self.players.contains_key(&id) {
            Err("ID already exists".to_string())
        } else {
            self.players.insert(id, player);
            Ok(())
        }
    }
    fn get_player(&self, id: &str) -> Option<&Player> {
        self.players.get(id)
    }
    fn remove_player(&mut self, id: &str) -> Option<Player> {
        self.players.remove(id)
    }
    fn list_connected_players(&self) -> Vec<&Player> {
        self.players
            .values()
            .filter(|value| value.is_connected)
            .collect()
    }
    fn disconnect_player(&mut self, id: &str) -> Result<(), String> {
        match self.players.get_mut(id) {
            Some(player) => {
                player.disconnect();
                Ok(())
            }
            None => Err("Player not found".to_string()),
        }
    }
    fn player_count(&self) -> usize {
        self.players.len()
    }
    fn connected_count(&self) -> usize {
        self.players.values().filter(|p| p.is_connected).count()
    }
}
fn main() {
    let mut player1 = Player::new("Gino");
    let mut player2 = Player::new("Fabrizio");

    let mut manager = PlayerManager::new();

    // Save IDs before moving players
    let player1_id = player1.id.clone();
    let player2_id = player2.id.clone();

    // Test get_player before adding (should be None)
    let player = manager.get_player(&player1_id);
    match player {
        Some(pl) => println!("{}", pl),
        None => println!("Error: cannot get player"),
    }

    // Add players
    let added_player_result = manager.add_player(player1);
    match added_player_result {
        Err(err) => println!("{}", err),
        Ok(..) => println!("Player added!"),
    }

    manager.add_player(player2).unwrap();

    let connected_players = manager.list_connected_players();
    println!("{:?}", connected_players);
    manager.disconnect_player(&player1_id).unwrap();
    manager.disconnect_player(&player2_id).unwrap();

    let connected_players = manager.list_connected_players();
    println!("{:?}", connected_players);

    let removed_player = manager.remove_player(&player1_id);
    match removed_player {
        None => println!("User not found!"),
        Some(player) => println!("{}", player),
    }
}
