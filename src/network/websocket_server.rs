use futures_util::{stream::SplitSink, StreamExt};
use std::{error::Error, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex},
};
use tokio_tungstenite::{accept_async, tungstenite::Message, WebSocketStream};
use uuid::Uuid;

use crate::{
    game::room_manager::RoomManager,
    network::{
        connection_manager::ConnectionManager,
        messages::{
            deserialize_message, handle_message, serialize_response, ServerError, ServerMessage,
            ServerResponse,
        },
    },
};

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
    SendToRoom {
        room_id: String,
        message: String,
    },
    SendToPlayer {
        connection_id: String,
        message: String,
    },
}
struct LobbyState {
    room_manager: RoomManager,
    connection_manager: ConnectionManager,
}

impl LobbyState {
    fn new() -> Self {
        Self {
            room_manager: RoomManager::new(),
            connection_manager: ConnectionManager::new(),
        }
    }
}

pub struct WebsocketServer {
    address: String,
}

impl WebsocketServer {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(&self.address).await?;

        let lobby_state = Arc::new(Mutex::new(LobbyState::new()));

        // Create channel for connection management commands
        let (cmd_sender, mut cmd_receiver) = mpsc::unbounded_channel::<ConnectionCommand>();

        // Spawn connection manager task
        let lobby_state_clone = lobby_state.clone();
        tokio::spawn(async move {
            while let Some(command) = cmd_receiver.recv().await {
                let mut state = lobby_state_clone.lock().await;

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
                    ConnectionCommand::SendToPlayer {
                        connection_id,
                        message,
                    } => {
                        state
                            .connection_manager
                            .send_to_player(&connection_id, &message)
                            .await;
                    }
                    ConnectionCommand::SendToRoom { room_id, message } => {
                        match state.room_manager.get_player_list(&room_id) {
                            None => println!("No room found"),
                            Some(player_id_vec) => {
                                for player_id in player_id_vec {
                                    state
                                        .connection_manager
                                        .send_to_player(&player_id, &message)
                                        .await;
                                }
                            }
                        };
                    }
                }
            }
        });

        // Accept connections
        while let Ok((stream, addr)) = listener.accept().await {
            println!("🔗 New connection from: {}", addr);
            let connection_id = Uuid::new_v4().to_string();

            let lobby_state = lobby_state.clone();
            let cmd_sender = cmd_sender.clone();

            tokio::spawn(async move {
                if let Err(e) =
                    Self::handle_connection(stream, connection_id, lobby_state, cmd_sender).await
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
        lobby_state: Arc<Mutex<LobbyState>>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Result<(), Box<dyn Error>> {
        let ws_stream = accept_async(stream).await?;
        println!("✅ WebSocket connection {} established", connection_id);

        let (ws_sender, mut ws_receiver) = ws_stream.split();

        cmd_sender.send(ConnectionCommand::AddConnection {
            id: connection_id.clone(),
            sender: ws_sender,
        })?;

        // Temporary handling for giving client connection_id on connect
        cmd_sender.send(ConnectionCommand::SendToPlayer {
            connection_id: connection_id.clone(),
            message: format!(r#"{{"Connection_id":"{}"}}"#, connection_id.clone()),
        })?;

        // Handle incoming messages
        while let Some(msg) = ws_receiver.next().await {
            match msg? {
                Message::Text(text) => {
                    println!("📨 Connection id: {} received: {}", connection_id, text);
                    match deserialize_message(&text) {
                        Ok(game_message) => {
                            println!("✅ Parsed message text: {:?}", game_message);

                            // Process the message and determine broadcast behavior
                            let response = {
                                let mut state = lobby_state.lock().await;
                                handle_message(
                                    game_message,
                                    &mut state.room_manager,
                                    &connection_id,
                                )
                            };

                            let parsed_msg = deserialize_message(&text)?;

                            match (&parsed_msg, &response) {
                                (
                                    ServerMessage::JoinRoom { room_id, .. },
                                    ServerResponse::PlayerJoined { .. },
                                ) => {
                                    if let Ok(json) = serialize_response(&response) {
                                        cmd_sender.send(ConnectionCommand::SendToPlayer {
                                            connection_id: connection_id.clone(),
                                            message: json,
                                        })?;
                                    }

                                    if let Ok(json) = serialize_response(&response) {
                                        cmd_sender.send(ConnectionCommand::SendToRoom {
                                            room_id: room_id.clone(),
                                            message: json,
                                        })?;
                                    }
                                }

                                (
                                    ServerMessage::Chat { .. },
                                    ServerResponse::ChatMessage { .. },
                                ) => {
                                    if let Ok(json) = serialize_response(&response) {
                                        cmd_sender
                                            .send(ConnectionCommand::SendToAll { message: json })?;
                                    }
                                }
                                (
                                    ServerMessage::PlayerReady { player_id },
                                    ServerResponse::GameStarted { .. },
                                ) => {
                                    println!("{:?}", response);
                                    if let Ok(json) = serialize_response(&response) {
                                        cmd_sender
                                            .send(ConnectionCommand::SendToAll { message: json })?;
                                    }
                                }
                                // Fallback for handling other messages
                                _ => {
                                    if let Ok(json) = serialize_response(&response) {
                                        cmd_sender
                                            .send(ConnectionCommand::SendToAll { message: json })?;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to parse message: {}", e);
                            let error_response = ServerResponse::Error {
                                message: ServerError::UnknownResponse,
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
                _ => {
                    // Handling other message types (maybe error?)
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
