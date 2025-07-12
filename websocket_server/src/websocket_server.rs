// src/multi_client_websocket.rs
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use std::{collections::HashMap, error::Error, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Mutex, mpsc},
};
use tokio_tungstenite::{WebSocketStream, accept_async, tungstenite::Message};
use uuid::Uuid;

use crate::{
    PlayerManager,
    messages::{
        ServerMessage, ServerResponse, deserialize_message, handle_message, serialize_response,
    },
};

// Represents a single WebSocket connection
#[derive(Debug)]
struct WebSocketConnection {
    id: String,
    player_id: Option<String>, // None until they join the game
    sender: SplitSink<WebSocketStream<TcpStream>, Message>,
}

// Commands sent to the connection manager
#[derive(Debug)]
enum ConnectionCommand {
    AddConnection {
        id: String,
        sender: SplitSink<WebSocketStream<TcpStream>, Message>,
    },
    RemoveConnection {
        id: String,
    },
    SendToAll {
        message: String,
    },
    SendToPlayer {
        player_id: String,
        message: String,
    },
    AssociatePlayer {
        connection_id: String,
        player_id: String,
    },
}

// Manages all active WebSocket connections
struct ConnectionManager {
    connections: HashMap<String, WebSocketConnection>,
}

impl ConnectionManager {
    fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    fn add_connection(
        &mut self,
        id: String,
        sender: SplitSink<WebSocketStream<TcpStream>, Message>,
    ) {
        let connection = WebSocketConnection {
            id: id.clone(),
            player_id: None,
            sender,
        };
        self.connections.insert(id.clone(), connection);
        println!(
            "📝 Added connection: {} (Total: {})",
            id,
            self.connections.len()
        );
    }

    fn remove_connection(&mut self, id: &str) {
        if let Some(connection) = self.connections.remove(id) {
            println!(
                "🗑️ Removed connection: {} (Total: {})",
                id,
                self.connections.len()
            );
            if let Some(player_id) = connection.player_id {
                println!("👋 Player {} disconnected", player_id);
            }
        }
    }

    fn associate_player(&mut self, connection_id: &str, player_id: String) {
        if let Some(connection) = self.connections.get_mut(connection_id) {
            connection.player_id = Some(player_id.clone());
            println!(
                "🔗 Associated connection {} with player {}",
                connection_id, player_id
            );
        }
    }

    async fn send_to_all(&mut self, message: &str) {
        println!(
            "📢 Broadcasting to {} connections: {}",
            self.connections.len(),
            message
        );

        let mut failed_connections = Vec::new();

        for (id, connection) in &mut self.connections {
            if let Err(e) = connection
                .sender
                .send(Message::Text(message.to_string()))
                .await
            {
                eprintln!("❌ Failed to send to connection {}: {}", id, e);
                failed_connections.push(id.clone());
            }
        }

        // Remove failed connections
        for failed_id in failed_connections {
            self.remove_connection(&failed_id);
        }
    }

    async fn send_to_player(&mut self, player_id: &str, message: &str) {
        let mut connection_to_send = None;

        // Find the connection for this player
        for (conn_id, connection) in &self.connections {
            if let Some(ref conn_player_id) = connection.player_id {
                if conn_player_id == player_id {
                    connection_to_send = Some(conn_id.clone());
                    break;
                }
            }
        }

        if let Some(conn_id) = connection_to_send {
            if let Some(connection) = self.connections.get_mut(&conn_id) {
                if let Err(e) = connection
                    .sender
                    .send(Message::Text(message.to_string()))
                    .await
                {
                    eprintln!("❌ Failed to send to player {}: {}", player_id, e);
                    self.remove_connection(&conn_id);
                }
            }
        }
    }
}

// Combined game state
struct GameState {
    player_manager: PlayerManager,
    connection_manager: ConnectionManager,
}

impl GameState {
    fn new() -> Self {
        Self {
            player_manager: PlayerManager::new(),
            connection_manager: ConnectionManager::new(),
        }
    }
}

pub struct MultiClientWebSocketServer {
    address: String,
}

impl MultiClientWebSocketServer {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(&self.address).await?;
        println!(
            "🌐 Multi-Client WebSocket Server listening on {}",
            self.address
        );

        // Create shared game state
        let game_state = Arc::new(Mutex::new(GameState::new()));

        // Create channel for connection management commands
        let (cmd_sender, mut cmd_receiver) = mpsc::unbounded_channel::<ConnectionCommand>();

        // Spawn connection manager task
        let game_state_clone = game_state.clone();
        tokio::spawn(async move {
            while let Some(command) = cmd_receiver.recv().await {
                let mut state = game_state_clone.lock().await;

                match command {
                    ConnectionCommand::AddConnection { id, sender } => {
                        state.connection_manager.add_connection(id, sender);
                    }
                    ConnectionCommand::RemoveConnection { id } => {
                        state.connection_manager.remove_connection(&id);
                    }
                    ConnectionCommand::SendToAll { message } => {
                        state.connection_manager.send_to_all(&message).await;
                    }
                    ConnectionCommand::SendToPlayer { player_id, message } => {
                        state
                            .connection_manager
                            .send_to_player(&player_id, &message)
                            .await;
                    }
                    ConnectionCommand::AssociatePlayer {
                        connection_id,
                        player_id,
                    } => {
                        state
                            .connection_manager
                            .associate_player(&connection_id, player_id);
                    }
                }
            }
        });

        // Accept connections
        while let Ok((stream, addr)) = listener.accept().await {
            println!("🔗 New connection from: {}", addr);
            let connection_id = Uuid::new_v4().to_string();

            let game_state = game_state.clone();
            let cmd_sender = cmd_sender.clone();

            tokio::spawn(async move {
                if let Err(e) =
                    Self::handle_connection(stream, connection_id, game_state, cmd_sender).await
                {
                    eprintln!("❌ Error handling connection: {}", e);
                }
            });
        }

        Ok(())
    }

    async fn handle_connection(
        stream: TcpStream,
        connection_id: String,
        game_state: Arc<Mutex<GameState>>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Result<(), Box<dyn Error>> {
        // Upgrade to WebSocket
        let ws_stream = accept_async(stream).await?;
        println!("✅ WebSocket connection {} established", connection_id);

        // Split the stream
        let (ws_sender, mut ws_receiver) = ws_stream.split();

        // Add connection to manager
        cmd_sender.send(ConnectionCommand::AddConnection {
            id: connection_id.clone(),
            sender: ws_sender,
        })?;

        // Handle incoming messages
        while let Some(msg) = ws_receiver.next().await {
            match msg? {
                Message::Text(text) => {
                    println!("📨 Connection {} received: {}", connection_id, text);

                    match deserialize_message(&text) {
                        Ok(game_message) => {
                            println!("✅ Parsed message: {:?}", game_message);

                            // Process the message and determine broadcast behavior
                            let response = {
                                let mut state = game_state.lock().await;
                                handle_message(game_message, &mut state.player_manager)
                            };

                            // Re-parse the original message for pattern matching
                            // (since we moved game_message above)
                            let parsed_msg = deserialize_message(&text)?;

                            // Handle special cases for broadcasting
                            match (&parsed_msg, &response) {
                                // When someone joins, associate their connection with player ID
                                (
                                    ServerMessage::Join { .. },
                                    ServerResponse::Welcome { player_id },
                                ) => {
                                    cmd_sender.send(ConnectionCommand::AssociatePlayer {
                                        connection_id: connection_id.clone(),
                                        player_id: player_id.clone(),
                                    })?;

                                    // Send welcome to the joining player
                                    if let Ok(json) = serialize_response(&response) {
                                        cmd_sender.send(ConnectionCommand::SendToPlayer {
                                            player_id: player_id.clone(),
                                            message: json,
                                        })?;
                                    }

                                    // Broadcast join notification to everyone else
                                    if let ServerMessage::Join { player_name } = parsed_msg {
                                        let join_notification = ServerResponse::PlayerJoined {
                                            player_name: player_name.clone(),
                                        };
                                        if let Ok(json) = serialize_response(&join_notification) {
                                            cmd_sender.send(ConnectionCommand::SendToAll {
                                                message: json,
                                            })?;
                                        }
                                    }
                                }

                                // Broadcast chat messages to everyone
                                (
                                    ServerMessage::Chat { .. },
                                    ServerResponse::ChatMessage { .. },
                                ) => {
                                    if let Ok(json) = serialize_response(&response) {
                                        cmd_sender
                                            .send(ConnectionCommand::SendToAll { message: json })?;
                                    }
                                }

                                // Handle other messages normally
                                _ => {
                                    if let Ok(json) = serialize_response(&response) {
                                        // For now, just send back to the sender
                                        // You can enhance this later for specific broadcasting logic
                                        cmd_sender
                                            .send(ConnectionCommand::SendToAll { message: json })?;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to parse message: {}", e);
                            let error_response = ServerResponse::Error {
                                message: format!("Invalid message: {}", e),
                            };
                            if let Ok(json) = serialize_response(&error_response) {
                                cmd_sender.send(ConnectionCommand::SendToAll { message: json })?;
                            }
                        }
                    }
                }
                Message::Close(_) => {
                    println!("👋 Connection {} requested close", connection_id);
                    break;
                }
                Message::Ping(data) => {
                    // Handle ping/pong at connection level if needed
                    println!("🏓 Ping from connection {}", connection_id);
                }
                _ => {
                    // Handle other message types
                }
            }
        }

        // Clean up when connection closes
        cmd_sender.send(ConnectionCommand::RemoveConnection {
            id: connection_id.clone(),
        })?;

        println!("📴 Connection {} closed", connection_id);
        Ok(())
    }
}
