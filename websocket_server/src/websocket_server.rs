use futures_util::{SinkExt, StreamExt};
use std::{
    error::Error,
    sync::{Arc, Mutex},
};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use crate::PlayerManager;
use crate::messages::{ServerResponse, deserialize_message, handle_message, serialize_response};

pub struct GameWebsocketServer {
    address: String,
    player_manager: Arc<Mutex<PlayerManager>>,
}

impl GameWebsocketServer {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
            player_manager: Arc::new(Mutex::new(PlayerManager::new())),
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(&self.address).await?;
        println!("🌐 WebSocket Server listening on {}", self.address);

        while let Ok((stream, addr)) = listener.accept().await {
            println!("🔗 New connection from: {}", addr);

            // Clone the Arc to share with the spawned task
            let player_manager = self.player_manager.clone();

            // Spawn a task to handle each connection
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, player_manager).await {
                    eprintln!("❌ Error handling WebSocket connection: {}", e);
                }
            });
        }

        Ok(())
    }

    async fn handle_connection(
        stream: TcpStream,
        player_manager: Arc<Mutex<PlayerManager>>, // Added parameter
    ) -> Result<(), Box<dyn Error>> {
        // Step 1: Upgrade to WebSocket
        let ws_stream = accept_async(stream).await?;
        println!("✅ WebSocket connection established");

        // Step 2: Split into sender and receiver
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Step 3: Handle messages in a loop
        while let Some(msg) = ws_receiver.next().await {
            match msg? {
                Message::Text(text) => {
                    println!("📨 Received JSON: {}", text);

                    match deserialize_message(&text) {
                        Ok(game_message) => {
                            println!("✅ Parsed ServerMessage: {:?}", game_message);

                            // Use the shared player manager - IMPORTANT: release lock before await
                            let response = {
                                let mut manager = player_manager.lock().unwrap();
                                handle_message(game_message, &mut manager)
                                // Lock is automatically released here
                            };

                            println!("🎮 Generated response: {:?}", response);

                            match serialize_response(&response) {
                                Ok(json_response) => {
                                    println!("📤 Sending JSON: {}", json_response);
                                    ws_sender.send(Message::Text(json_response)).await?;
                                }
                                Err(err) => {
                                    eprintln!("❌ Failed to serialize response: {}", err);
                                    let error_msg = format!(
                                        "{{\"Error\":{{\"message\":\"Failed to serialize response: {}\"}}}}",
                                        err
                                    );
                                    ws_sender.send(Message::Text(error_msg)).await?;
                                }
                            }
                        }
                        Err(parse_error) => {
                            eprintln!("❌ Failed to parse JSON: {}", parse_error);

                            let error_response = ServerResponse::Error {
                                message: format!("Invalid JSON message: {}", parse_error),
                            };

                            match serialize_response(&error_response) {
                                Ok(error_json) => {
                                    println!("📤 Sending error: {}", error_json);
                                    ws_sender.send(Message::Text(error_json)).await?;
                                }
                                Err(_) => {
                                    // Fallback error if even error serialization fails
                                    let fallback_error = format!(
                                        "{{\"Error\":{{\"message\":\"Invalid JSON: {}\"}}}}",
                                        text.replace('"', "'") // Escape quotes to prevent JSON corruption
                                    );
                                    ws_sender.send(Message::Text(fallback_error)).await?;
                                }
                            }
                        }
                    }

                    // REMOVED: The echo line that was causing double messages
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
