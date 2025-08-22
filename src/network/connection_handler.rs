use futures_util::StreamExt;
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use crate::network::message_router::handle_game_message;
use crate::network::messages::{
    deserialize_message, serialize_response, ClientMessageCategory, ServerResponse,
};
use crate::{handle_lobby_message, ConnectionCommand, GameLoopRegistry, RoomManager};

pub struct ConnectionHandler;

impl ConnectionHandler {
    pub async fn handle_connection(
        stream: TcpStream,
        connection_id: String,
        room_manager: Arc<Mutex<RoomManager>>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
        game_registry: Arc<GameLoopRegistry>,
    ) -> Result<(), Box<dyn Error>> {
        let ws_stream = accept_async(stream).await?;
        println!("✅ WebSocket connection {} established", connection_id);

        let (ws_sender, mut ws_receiver) = ws_stream.split();

        cmd_sender.send(ConnectionCommand::AddConnection {
            id: connection_id.clone(),
            sender: ws_sender,
        })?;

        // TEMPORARY FOR DEBUGGING: Send connection ID to client
        let connection_id_message = serialize_response(ServerResponse::ConnectionId {
            connection_id: connection_id.clone(),
        });
        cmd_sender.send(ConnectionCommand::SendToPlayer {
            connection_id: connection_id.clone(),
            message: connection_id_message,
        })?;

        while let Some(msg) = ws_receiver.next().await {
            match msg? {
                Message::Text(text) => {
                    // Filter the 2 different messages: Game and Lobby
                    let client_message = match deserialize_message(&text) {
                        Ok(msg) => msg,
                        Err(_) => {
                            cmd_sender.send(ConnectionCommand::SendToPlayer {
                                connection_id: connection_id.to_string(),
                                message: serialize_response(ServerResponse::from_app_error(
                                    &crate::AppError::SerializationError {
                                        message: "Deserialization error".to_string(),
                                    },
                                )),
                            })?;
                            continue;
                        }
                    };

                    match client_message.category() {
                        ClientMessageCategory::LobbyMessage => {
                            handle_lobby_message(
                                client_message,
                                &connection_id,
                                &room_manager,
                                &cmd_sender,
                                &game_registry,
                            )
                            .await
                        }

                        ClientMessageCategory::GameMessage => handle_game_message(
                            client_message,
                            &connection_id,
                            &game_registry,
                            &cmd_sender,
                        ),
                    }
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

        game_registry.remove_player(&connection_id);

        println!("📴 Connection {} closed", connection_id);
        Ok(())
    }
}
