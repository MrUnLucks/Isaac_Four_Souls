use futures_util::StreamExt;
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use crate::network::lobby::LobbyState;
use crate::network::messages::{serialize_response, ServerResponse};
use crate::network::websocket::commands::ConnectionCommand;
use crate::network::websocket::handler::MessageHandler;

pub struct ConnectionHandler;

impl ConnectionHandler {
    pub async fn handle_connection(
        stream: TcpStream,
        connection_id: String,
        lobby_state: Arc<Mutex<LobbyState>>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Result<(), Box<dyn Error>> {
        let ws_stream = accept_async(stream).await?;
        println!("✅ WebSocket connection {} established", connection_id);

        let (ws_sender, mut ws_receiver) = ws_stream.split();

        // Add connection to manager
        cmd_sender.send(ConnectionCommand::AddConnection {
            id: connection_id.clone(),
            sender: ws_sender,
        })?;

        // Send connection ID to client
        let connection_id_message = serialize_response(&ServerResponse::ConnectionId {
            connection_id: connection_id.clone(),
        })?;
        cmd_sender.send(ConnectionCommand::SendToPlayer {
            connection_id: connection_id.clone(),
            message: connection_id_message,
        })?;

        // Handle incoming messages
        while let Some(msg) = ws_receiver.next().await {
            match msg? {
                Message::Text(text) => {
                    if let Err(e) = MessageHandler::handle_text_message(
                        text,
                        &connection_id,
                        &lobby_state,
                        &cmd_sender,
                    )
                    .await
                    {
                        eprintln!("❌ Error handling message: {}", e);
                    }
                }
                Message::Close(_) => {
                    println!("👋 Connection {} requested close", connection_id);
                    break;
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
