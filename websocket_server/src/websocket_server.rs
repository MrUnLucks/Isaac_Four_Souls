use futures_util::{SinkExt, StreamExt};
use std::error::Error;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};

pub struct WebSocketServer {
    address: String,
}

impl WebSocketServer {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(&self.address).await?;
        println!("🌐 WebSocket Server listening on {}", self.address);

        while let Ok((stream, addr)) = listener.accept().await {
            println!("🔗 New connection from: {}", addr);

            // Spawn a task to handle each connection
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream).await {
                    eprintln!("❌ Error handling WebSocket connection: {}", e);
                }
            });
        }

        Ok(())
    }

    async fn handle_connection(stream: TcpStream) -> Result<(), Box<dyn Error>> {
        // Step 1: Upgrade to WebSocket
        let ws_stream = accept_async(stream).await?;
        println!("✅ WebSocket connection established");

        // Step 2: Split into sender and receiver
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Step 3: Handle messages in a loop
        while let Some(msg) = ws_receiver.next().await {
            match msg? {
                Message::Text(text) => {
                    println!("📨 Received text: {}", text);
                    // Echo back the message
                    let response = format!("Echo: {}", text);
                    ws_sender.send(Message::Text(response)).await?;
                }
                Message::Binary(data) => {
                    println!("📨 Received {} bytes of binary data", data.len());
                    // Echo back binary data
                    ws_sender.send(Message::Binary(data)).await?;
                }
                Message::Close(_) => {
                    println!("👋 Client requested close");
                    break;
                }
                Message::Ping(data) => {
                    println!("🏓 Received ping");
                    ws_sender.send(Message::Pong(data)).await?;
                }
                Message::Pong(_) => {
                    println!("🏓 Received pong");
                }
                Message::Frame(_) => {
                    println!("🔧 Received raw frame (ignoring)");
                }
            }
        }

        println!("📴 WebSocket connection closed");
        Ok(())
    }
}
