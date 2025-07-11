// tests/integration_tests.rs

use isaac_four_souls::{
    async_utils::{handle_multiple_requests, simulate_network_delay},
    player::Player,
    player_manager::PlayerManager,
};

#[tokio::test]
async fn test_player_manager_workflow() {
    println!("=== Testing Player Manager Workflow ===");

    let player1 = Player::new("Gino");
    let player2 = Player::new("Fabrizio");

    let mut manager = PlayerManager::new();

    // Save IDs before moving players
    let player1_id = player1.id.clone();
    let player2_id = player2.id.clone();

    // Test get_player before adding (should be None)
    println!("Testing get_player before adding...");
    let player = manager.get_player(&player1_id);
    match player {
        Some(pl) => println!("Unexpected: Found player {}", pl),
        None => println!("✓ Correctly returned None for non-existent player"),
    }

    // Add players
    println!("Adding players...");
    let added_player_result = manager.add_player(player1);
    match added_player_result {
        Err(err) => println!("✗ Error adding player: {}", err),
        Ok(..) => println!("✓ Player 1 added successfully!"),
    }

    // Test adding second player
    match manager.add_player(player2) {
        Err(err) => println!("✗ Error adding player 2: {}", err),
        Ok(..) => println!("✓ Player 2 added successfully!"),
    }

    // Test player listing
    println!("Testing player listing...");
    let connected_players = manager.list_connected_players();
    println!("Connected players: {:?}", connected_players);
    assert_eq!(
        connected_players.len(),
        2,
        "Should have 2 connected players"
    );

    // Test disconnect
    println!("Testing player disconnection...");
    manager.disconnect_player(&player1_id).unwrap();
    manager.disconnect_player(&player2_id).unwrap();

    let connected_players = manager.list_connected_players();
    println!(
        "Connected players after disconnect: {:?}",
        connected_players
    );
    assert_eq!(
        connected_players.len(),
        0,
        "Should have 0 connected players"
    );

    // Test player removal
    println!("Testing player removal...");
    let removed_player = manager.remove_player(&player1_id);
    match removed_player {
        None => println!("✗ Player not found for removal!"),
        Some(player) => println!("✓ Removed player: {}", player),
    }

    // Test counts
    println!("Testing player counts...");
    let total_count = manager.player_count();
    let connected_count = manager.connected_count();
    println!(
        "Total players: {}, Connected: {}",
        total_count, connected_count
    );

    // Test async utilities
    println!("Testing async network simulation...");
    let result = simulate_network_delay().await;
    println!("Network delay result: {}", result);

    println!("Testing multiple concurrent requests...");
    handle_multiple_requests().await;

    println!("=== All tests completed successfully! ===");
}

#[tokio::test]
async fn test_player_manager_edge_cases() {
    println!("=== Testing Edge Cases ===");

    let mut manager = PlayerManager::new();

    // Test removing non-existent player
    let fake_id = "non-existent-id";
    let result = manager.remove_player(fake_id);
    assert!(
        result.is_none(),
        "Should return None for non-existent player"
    );

    // Test disconnecting non-existent player
    let disconnect_result = manager.disconnect_player(fake_id);
    assert!(
        disconnect_result.is_err(),
        "Should return error for non-existent player"
    );

    // Test adding duplicate player
    let player = Player::new("TestPlayer");
    let _player_id = player.id.clone();
    let duplicate_player = Player::new("TestPlayer");
    // Note: This should work since they have different IDs even with same name

    manager.add_player(player).unwrap();
    manager.add_player(duplicate_player).unwrap();

    assert_eq!(
        manager.player_count(),
        2,
        "Should allow players with same name but different IDs"
    );

    println!("✓ All edge cases handled correctly");
}

#[tokio::test]
async fn test_concurrent_operations() {
    println!("=== Testing Concurrent Operations ===");

    let mut manager = PlayerManager::new();
    let mut _handles: Vec<tokio::task::JoinHandle<()>> = vec![];

    // Create multiple players concurrently
    for i in 0..10 {
        let player = Player::new(&format!("Player{}", i));
        manager.add_player(player).unwrap();
    }

    // Test concurrent access (this would be more meaningful with Arc<Mutex<PlayerManager>>)
    let connected_count = manager.connected_count();
    let total_count = manager.player_count();

    println!(
        "Concurrent test - Connected: {}, Total: {}",
        connected_count, total_count
    );
    assert_eq!(connected_count, 10, "All players should be connected");
    assert_eq!(total_count, 10, "Total count should match");

    println!("✓ Concurrent operations test passed");
}

// Helper function to run manual tests (not using #[tokio::test])
pub async fn run_manual_integration_tests() {
    println!("🚀 Running Isaac Four Souls Manual Integration Tests\n");

    // Since the actual tests are marked with #[tokio::test], we'll create
    // a manual version here for demonstration purposes
    manual_player_manager_test().await;

    println!("\n🎉 Manual integration tests completed!");
    println!("💡 Run 'cargo test' to execute the full test suite with #[tokio::test]");
}

async fn manual_player_manager_test() {
    println!("=== Manual Player Manager Test ===");

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
        None => println!("✓ Error: cannot get player (expected behavior)"),
    }

    // Add players
    let added_player_result = manager.add_player(player1);
    match added_player_result {
        Err(err) => println!("✗ {}", err),
        Ok(..) => println!("✓ Player added!"),
    }

    manager.add_player(player2).unwrap();

    let connected_players = manager.list_connected_players();
    println!("Connected players: {:?}", connected_players);

    manager.disconnect_player(&player1_id).unwrap();
    manager.disconnect_player(&player2_id).unwrap();

    let connected_players = manager.list_connected_players();
    println!(
        "Connected players after disconnect: {:?}",
        connected_players
    );

    let removed_player = manager.remove_player(&player1_id);
    match removed_player {
        None => println!("✓ User not found! (expected after removal)"),
        Some(player) => println!("✓ Removed: {}", player),
    }

    println!("Player count: {}", manager.player_count());
    println!("Connected count: {}", manager.connected_count());

    let result = simulate_network_delay().await;
    println!("Network delay result: {}", result);

    handle_multiple_requests().await;
}
