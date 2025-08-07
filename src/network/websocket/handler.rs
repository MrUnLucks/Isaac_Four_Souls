use std::error::Error;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::network::lobby::LobbyState;
use crate::network::messages::{
    deserialize_message, handle_message, serialize_response, ClientMessage, ServerError,
    ServerResponse,
};
use crate::network::websocket::commands::ConnectionCommand;

pub struct MessageHandler;

impl MessageHandler {
    pub async fn handle_text_message(
        text: String,
        connection_id: &str,
        lobby_state: &Arc<Mutex<LobbyState>>,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Result<(), Box<dyn Error>> {
        match deserialize_message(&text) {
            Ok(game_message) => {
                println!("✅ Parsed message text: {:?}", game_message);
                Self::process_game_message(
                    game_message,
                    connection_id,
                    lobby_state,
                    cmd_sender,
                    &text,
                )
                .await?;
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
        Ok(())
    }

    async fn process_game_message(
        game_message: ClientMessage,
        connection_id: &str,
        lobby_state: &Arc<Mutex<LobbyState>>,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
        original_text: &str,
    ) -> Result<(), Box<dyn Error>> {
        // Process the message and determine broadcast behavior
        let response = {
            let mut state = lobby_state.lock().await;
            let result =
                handle_message(game_message.clone(), &mut state.room_manager, connection_id);

            let current_room_id = state
                .room_manager
                .get_player_room_from_connection_id(connection_id);

            match result {
                Ok(server_response) => (server_response, current_room_id),
                Err(err) => (ServerResponse::Error { message: err }, current_room_id),
            }
        };
        let (response, current_room_id) = response;

        let parsed_msg = deserialize_message(original_text)?;

        // Route the response based on message type
        Self::route_response(
            &parsed_msg,
            &response,
            connection_id,
            current_room_id,
            cmd_sender,
        )?;

        Ok(())
    }

    fn route_response(
        parsed_msg: &ClientMessage,
        response: &ServerResponse,
        connection_id: &str,
        current_room_id: Option<String>,
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Result<(), Box<dyn Error>> {
        match (parsed_msg, response) {
            (
                ClientMessage::JoinRoom { room_id, .. },
                ServerResponse::PlayerJoined {
                    player_id,
                    player_name,
                },
            ) => {
                let player_name = player_name.to_string();
                let player_id = player_id.to_string();

                let joiner_response = ServerResponse::SelfJoined {
                    player_name: player_name.clone(),
                    player_id: player_id.clone(),
                };

                let joiner_json = serialize_response(&joiner_response)?;
                cmd_sender.send(ConnectionCommand::SendToPlayer {
                    connection_id: connection_id.to_string(),
                    message: joiner_json,
                })?;

                let others_response = ServerResponse::PlayerJoined {
                    player_name,
                    player_id,
                };

                let others_json = serialize_response(&others_response)?;
                cmd_sender.send(ConnectionCommand::SendToRoomExceptPlayer {
                    connection_id: connection_id.to_string(),
                    room_id: room_id.to_string(),
                    message: others_json,
                })?;
            }
            (ClientMessage::Chat { .. }, ServerResponse::ChatMessage { .. }) => {
                if let Ok(json) = serialize_response(response) {
                    if let Some(room_id) = current_room_id {
                        cmd_sender.send(ConnectionCommand::SendToRoom {
                            room_id,
                            message: json,
                        })?;
                    }
                }
            }
            (ClientMessage::PlayerReady { .. }, ServerResponse::GameStarted { .. }) => {
                if let Ok(json) = serialize_response(response) {
                    cmd_sender.send(ConnectionCommand::SendToAll { message: json })?;
                }
            }
            (
                ClientMessage::CreateRoom { .. },
                ServerResponse::FirstPlayerRoomCreated { room_id, player_id },
            ) => {
                let first_player_response = ServerResponse::FirstPlayerRoomCreated {
                    room_id: room_id.to_string(),
                    player_id: player_id.to_string(),
                };

                let first_player_response = serialize_response(&first_player_response)?;
                cmd_sender.send(ConnectionCommand::SendToPlayer {
                    connection_id: connection_id.to_string(),
                    message: first_player_response,
                })?;

                let others_response = ServerResponse::RoomCreated {
                    room_id: room_id.to_string(),
                };

                let others_json = serialize_response(&others_response)?;
                cmd_sender.send(ConnectionCommand::SendToAll {
                    message: others_json,
                })?;
            }
            _ => {
                if let Ok(json) = serialize_response(response) {
                    cmd_sender.send(ConnectionCommand::SendToAll { message: json })?;
                }
            }
        }
        Ok(())
    }
}
