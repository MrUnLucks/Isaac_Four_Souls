mod async_utils;
mod messages;
mod player;
mod player_manager; // Add this
mod traits

use async_utils::{handle_multiple_requests, simulate_network_delay};
use player::Player;
use player_manager::PlayerManager;

#[tokio::main]
async fn main() {
    let player1 = Player::new("Gino");
    let player2 = Player::new("Fabrizio");

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

    manager.player_count();
    manager.connected_count();
    let result = simulate_network_delay().await;
    println!("{}", result);

    handle_multiple_requests().await;
}
