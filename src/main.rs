use isaac_four_souls::websocket_server::MultiClientWebSocketServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎮 Starting Isaac Four Souls TCP Server...");

    let server = MultiClientWebSocketServer::new("127.0.0.1:8080");
    server.run().await?;

    Ok(())
}
