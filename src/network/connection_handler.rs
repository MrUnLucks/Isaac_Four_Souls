use futures_util::StreamExt;
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, tungstenite::Message};

use crate::actors::actor_registry::ActorRegistry;
use crate::actors::connection_actor::{ConnectionActor, ConnectionMessage};
use crate::network::messages::{deserialize_message, serialize_response, ServerResponse};
use crate::{AppError, ConnectionCommand};

pub struct ConnectionHandler;

impl ConnectionHandler {
    pub async fn handle_connection(
        stream: TcpStream,
        connection_id: String,
        actor_registry: Arc<ActorRegistry>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Result<(), Box<dyn Error>> {
        let ws_stream = accept_async(stream).await?;
        println!("✅ WebSocket connection {} established", connection_id);

        let (ws_sender, mut ws_receiver) = ws_stream.split();

        // Add WebSocket connection to connection manager
        cmd_sender.send(ConnectionCommand::AddConnection {
            id: connection_id.clone(),
            sender: ws_sender,
        })?;

        // Send connection ID to client
        let connection_id_message = serialize_response(ServerResponse::ConnectionId {
            connection_id: connection_id.clone(),
        });
        cmd_sender.send(ConnectionCommand::SendToPlayer {
            connection_id: connection_id.clone(),
            message: connection_id_message,
        })?;

        let (conn_sender, conn_receiver) = mpsc::unbounded_channel::<ConnectionMessage>();
        let mut connection_actor = ConnectionActor::new(
            connection_id.clone(),
            actor_registry.clone(),
            cmd_sender.clone(),
        );

        // Register connection actor in registry
        actor_registry.register_connection_actor(connection_id.clone(), conn_sender.clone());

        // Spawn connection actor task
        tokio::spawn(async move {
            connection_actor.run(conn_receiver).await;
        });

        // Main WebSocket loop just forwards messages to connection actor
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    match deserialize_message(&text) {
                        Ok(client_message) => {
                            let connection_message = ConnectionMessage::ClientMessage {
                                message: client_message,
                            };

                            if let Err(_) = conn_sender.send(connection_message) {
                                // Connection actor is gone, break the loop
                                eprintln!("Connection actor for {} is gone", connection_id);
                                break;
                            }
                        }
                        Err(e) => {
                            eprintln!("Parse error from {}: {}", connection_id, e);
                            // Send error but continue
                            let _ = cmd_sender.send(ConnectionCommand::SendToPlayer {
                                connection_id: connection_id.clone(),
                                message: serialize_response(ServerResponse::from_app_error(
                                    &AppError::UnknownMessage {
                                        message: format!("Parse error: {}", e),
                                    },
                                )),
                            });
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("🔌 WebSocket close for {}", connection_id);
                    break;
                }
                Ok(_) => continue, // Ignore other message types
                Err(e) => {
                    eprintln!("WebSocket error {}: {}", connection_id, e);
                    break;
                }
            }
        }

        // Notify connection actor to disconnect
        let _ = actor_registry.disconnect_connection_actor(&connection_id);

        // Remove WebSocket connection
        cmd_sender.send(ConnectionCommand::RemoveConnection {
            id: connection_id.clone(),
        })?;

        println!("📴 Connection {} closed", connection_id);
        Ok(())
    }
}
