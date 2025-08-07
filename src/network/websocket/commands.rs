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
    SendToRoom {
        room_id: String,
        message: String,
    },
    SendToPlayer {
        connection_id: String,
        message: String,
    },
    SendToRoomExceptPlayer {
        connection_id: String,
        room_id: String,
        message: String,
    },
}

pub struct CommandProcessor;

impl CommandProcessor {
    pub async fn process_command(
        command: ConnectionCommand,
        state: &mut crate::network::lobby::LobbyState,
    ) -> Result<(), Box<dyn Error>> {
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
                    .await?;
            }
            ConnectionCommand::SendToRoom { room_id, message } => {
                if let Some(connection_ids) =
                    state.room_manager.get_connections_id_from_room_id(&room_id)
                {
                    for connection_id in connection_ids {
                        state
                            .connection_manager
                            .send_to_player(&connection_id, &message)
                            .await?;
                    }
                }
            }
            ConnectionCommand::SendToRoomExceptPlayer {
                connection_id,
                room_id,
                message,
            } => {
                if let Some(mut connection_ids) =
                    state.room_manager.get_connections_id_from_room_id(&room_id)
                {
                    if connection_ids.remove(&connection_id) {
                        for connection_id in connection_ids {
                            state
                                .connection_manager
                                .send_to_player(&connection_id, &message)
                                .await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
