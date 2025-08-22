use std::error::Error;

use futures_util::stream::SplitSink;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

#[derive(Debug)]
pub enum ConnectionCommand {
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
        connection_id: String,
        message: String,
    },
    SendToPlayers {
        connections_id: Vec<String>,
        message: String,
    },
}

pub struct CommandProcessor;

impl CommandProcessor {
    pub async fn process_command(
        command: ConnectionCommand,
        connection_manager: &mut crate::ConnectionManager,
    ) -> Result<(), Box<dyn Error>> {
        match command {
            ConnectionCommand::AddConnection { id, sender } => {
                connection_manager.add_connection(id, sender);
            }
            ConnectionCommand::RemoveConnection { id } => {
                connection_manager.remove_connection(&id);
            }
            ConnectionCommand::SendToAll { message } => {
                connection_manager.send_to_all(&message).await;
            }
            ConnectionCommand::SendToPlayer {
                connection_id,
                message,
            } => {
                connection_manager
                    .send_to_player(&connection_id, &message)
                    .await?;
            }
            ConnectionCommand::SendToPlayers {
                connections_id,
                message,
            } => {
                for connection_id in connections_id {
                    connection_manager
                        .send_to_player(&*connection_id, &message)
                        .await?;
                }
            }
        }
        Ok(())
    }
}
