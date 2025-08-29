use futures_util::StreamExt;
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use crate::actors::actor_registry::ActorRegistry;
use crate::actors::lobby_actor::LobbyMessage;
use crate::network::message_router::handle_game_message;
use crate::network::messages::{
    deserialize_message, serialize_response, ClientMessage, ClientMessageCategory, ServerResponse,
};
use crate::{AppError, ConnectionCommand, GameMessageLoopRegistry};

pub struct ConnectionHandler;

impl ConnectionHandler {
    pub async fn handle_connection(
        stream: TcpStream,
        connection_id: String,
        actor_registry: Arc<ActorRegistry>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
        game_registry: Arc<GameMessageLoopRegistry>,
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
            match msg {
                Ok(Message::Text(text)) => {
                    if let Err(e) = process_message(
                        text,
                        &connection_id,
                        &actor_registry,
                        &cmd_sender,
                        &game_registry,
                    )
                    .await
                    {
                        eprintln!(
                            "⚠️ Message error from {}: {} (continuing...)",
                            connection_id, e
                        );
                        // Send error but keep connection alive
                        let _ = cmd_sender.send(ConnectionCommand::SendToPlayer {
                            connection_id: connection_id.clone(),
                            message: serialize_response(ServerResponse::from_app_error(
                                &AppError::UnknownMessage {
                                    message: "Message failed to process".to_string(),
                                },
                            )),
                        });
                    }
                }
                Ok(Message::Close(_)) => break,
                Ok(_) => continue, // Ignore other message types
                Err(e) => {
                    eprintln!("WebSocket error {}: {}", connection_id, e);
                    break; // Only break on WebSocket errors
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

async fn process_message(
    text: String,
    connection_id: &str,
    actor_registry: &Arc<ActorRegistry>,
    cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    game_registry: &Arc<GameMessageLoopRegistry>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client_message = deserialize_message(&text).map_err(|e| format!("Parse error: {}", e))?;

    match client_message.category() {
        ClientMessageCategory::LobbyMessage => {
            let lobby_message = convert_to_lobby_message(client_message, connection_id)?;
            actor_registry.send_lobby_message(lobby_message)?;
        }
        ClientMessageCategory::GameMessage => {
            handle_game_message(client_message, connection_id, game_registry, cmd_sender);
        }
    }
    Ok(())
}
fn convert_to_lobby_message(
    client_message: ClientMessage,
    connection_id: &str,
) -> Result<LobbyMessage, AppError> {
    let connection_id = connection_id.to_string();

    match client_message {
        ClientMessage::Ping => Ok(LobbyMessage::Ping { connection_id }),
        ClientMessage::Chat { message } => Ok(LobbyMessage::Chat {
            connection_id,
            message,
        }),
        ClientMessage::CreateRoom {
            room_name,
            first_player_name,
        } => Ok(LobbyMessage::CreateRoom {
            connection_id,
            room_name,
            first_player_name,
        }),
        ClientMessage::DestroyRoom { room_id } => Ok(LobbyMessage::DestroyRoom {
            connection_id,
            room_id,
        }),
        ClientMessage::JoinRoom {
            player_name,
            room_id,
        } => Ok(LobbyMessage::JoinRoom {
            connection_id,
            player_name,
            room_id,
        }),
        ClientMessage::LeaveRoom => Ok(LobbyMessage::LeaveRoom { connection_id }),
        ClientMessage::PlayerReady => Ok(LobbyMessage::PlayerReady { connection_id }),
        _ => Err(AppError::Internal {
            message: "Invalid lobby message conversion".to_string(),
        }),
    }
}
