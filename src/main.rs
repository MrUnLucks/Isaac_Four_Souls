use isaac_four_souls::game::card_loader;
use isaac_four_souls::network::websocket::WebsocketServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    card_loader::initialize_database();
    println!("🎮 Starting Isaac Four Souls TCP Server...");
    let server = WebsocketServer::new("127.0.0.1:8080");
    server.run().await?;
    Ok(())
}
