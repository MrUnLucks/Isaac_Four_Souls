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
        let is_player_present = self.players.iter().any(|(key, _player)| *key == player.id);
        self.players.insert(player.id.clone(), player);
        if is_player_present {
            Ok(())
        } else {
            Err("ID already exists".to_string())
        }
    }
    fn get_player(&self, id: String) -> Option<&Player> {
        self.players
            .iter()
            .find_map(|(key, player)| if *key == id { Some(player) } else { None })
    }
    fn remove_player(&mut self, id: String) -> Option<Player> {
        self.players.remove(&id)
    }
    fn list_connected_players(&self) -> Vec<&Player> {
        self.players
            .iter()
            .filter(|(key, value)| value.is_connected)
            .map(|(key, value)| value)
            .collect()
    }
}
fn main() {
    let mut player1 = Player::new("Gino");
    let mut player2 = Player::new("Fabrizio");
    player1.disconnect();
    player2.disconnect();

    let mut manager = PlayerManager::new();
    let player = manager.get_player(player1.id.clone());
    match player {
        Some(pl) => println!("{}", pl),
        None => println!("Error: cannot get player"),
    }
    let added_player = manager.add_player(player1);
    match added_player {
        Err(err) => println!("{}", err),
        Ok(..) => println!("Player added!"),
    }
    let connected_players = list_connected_players()
}
