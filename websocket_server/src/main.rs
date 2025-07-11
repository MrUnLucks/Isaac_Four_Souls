mod async_utils;
mod messages;
mod player;
mod player_manager;
mod tcp_server;
mod traits;

// use tcp_server::TcpServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎮 Starting Isaac Four Souls TCP Server...");

    // let server = TcpServer::new("127.0.0.1:8080").await?;
    // println!("🚀 Server listening on 127.0.0.1:8080");

    // // Run the server
    // server.run().await?;

    Ok(())
}
