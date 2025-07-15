use isaac_four_souls::game::resources::PlayerResources;
use isaac_four_souls::websocket_server::MultiClientWebSocketServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎮 Starting Isaac Four Souls TCP Server...");

    let server = MultiClientWebSocketServer::new("127.0.0.1:8080");
    // server.run().await?;
    let mut resources = PlayerResources::new(2);
    assert_eq!(resources.health, 2);
    assert_eq!(resources.coins, 3);
    assert!(resources.spend_coins(2)); // Should work
    assert!(!resources.spend_coins(5)); // Should fail
    Ok(())
}
