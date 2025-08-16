use crate::game::outbound_handler::spawn_outbound_handler;
use std::sync::Arc;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::{mpsc, Mutex};

use crate::game::game_loop::GameEvent;
use crate::network::messages::{
    deserialize_message, serialize_response, ClientMessage, ServerResponse,
};
use crate::{AppError, ConnectionCommand, LobbyState};

#[derive(Debug)]
pub enum MessageRouterError {
    App(AppError),
    Send(SendError<ConnectionCommand>),
}

impl From<AppError> for MessageRouterError {
    fn from(err: AppError) -> Self {
        MessageRouterError::App(err)
    }
}

impl From<SendError<ConnectionCommand>> for MessageRouterError {
    fn from(err: SendError<ConnectionCommand>) -> Self {
        MessageRouterError::Send(err)
    }
}

fn send_error(
    cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    connection_id: &str,
    error: &AppError,
) -> Result<(), SendError<ConnectionCommand>> {
    cmd_sender.send(ConnectionCommand::SendToPlayer {
        connection_id: connection_id.to_string(),
        message: serialize_response(ServerResponse::from_app_error(error)),
    })
}

pub async fn handle_text_message(
    text: String,
    connection_id: &str,
    lobby_state: &Arc<Mutex<LobbyState>>,
    cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
) {
    let game_message = match deserialize_message(&text) {
        Ok(msg) => msg,
        Err(_) => {
            let _ = send_error(
                cmd_sender,
                connection_id,
                &AppError::SerializationError {
                    message: "Failed to deserialize message".to_string(),
                },
            );
            return;
        }
    };

    let mut state = lobby_state.lock().await;
    let room_id = state
        .room_manager
        .get_player_room_from_connection_id(connection_id);
    let connection_id = connection_id.to_string();

    match handle_message(
        game_message,
        room_id,
        connection_id.clone(),
        cmd_sender,
        &mut state,
    ) {
        Ok(()) => {}
        Err(MessageRouterError::App(app_error)) => {
            // Send the app error to the client
            let _ = send_error(cmd_sender, &connection_id, &app_error);
        }
        Err(MessageRouterError::Send(_)) => {
            // Critical send error - connection probably broken
            eprintln!(
                "Critical send error for connection {}, connection likely broken",
                connection_id
            );
        }
    }
}

pub fn handle_message(
    game_message: ClientMessage,
    room_id: Option<String>,
    connection_id: String,
    cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    state: &mut LobbyState,
) -> Result<(), MessageRouterError> {
    match game_message {
        ClientMessage::Ping => {
            cmd_sender.send(ConnectionCommand::SendToPlayer {
                connection_id,
                message: serialize_response(ServerResponse::Pong),
            })?;
        }

        ClientMessage::Chat { message } => {
            let room_id = room_id.ok_or(AppError::ConnectionNotInRoom)?;
            let player_name = state
                .room_manager
                .get_player_name_from_connection_id(&connection_id)
                .ok_or(AppError::ConnectionNotInRoom)?;

            cmd_sender.send(ConnectionCommand::SendToRoom {
                room_id,
                message: serialize_response(ServerResponse::ChatMessage {
                    player_name,
                    message,
                }),
            })?;
        }

        ClientMessage::CreateRoom {
            room_name,
            first_player_name,
        } => {
            let (room_id, new_player_id) = state.room_manager.create_room(
                room_name,
                connection_id.clone(),
                first_player_name,
            )?;

            cmd_sender.send(ConnectionCommand::SendToPlayer {
                connection_id,
                message: serialize_response(ServerResponse::RoomCreated {
                    room_id: room_id.clone(),
                    player_id: new_player_id,
                }),
            })?;

            cmd_sender.send(ConnectionCommand::SendToAll {
                message: serialize_response(ServerResponse::RoomCreatedBroadcast { room_id }),
            })?;
        }

        ClientMessage::DestroyRoom { room_id } => {
            let destroyed_room_id = state.room_manager.destroy_room(&room_id, &connection_id)?;

            state
                .game_loop_registry
                .cleanup_game_loop(&destroyed_room_id);
            cmd_sender.send(ConnectionCommand::SendToAll {
                message: serialize_response(ServerResponse::RoomDestroyed {
                    room_id: destroyed_room_id,
                }),
            })?;
        }

        ClientMessage::JoinRoom {
            player_name,
            room_id,
        } => {
            let player_id = state.room_manager.join_room(
                &room_id,
                connection_id.clone(),
                player_name.clone(),
            )?;

            cmd_sender.send(ConnectionCommand::SendToPlayer {
                connection_id: connection_id.clone(),
                message: serialize_response(ServerResponse::SelfJoined {
                    player_name: player_name.clone(),
                    player_id: player_id.clone(),
                }),
            })?;

            cmd_sender.send(ConnectionCommand::SendToRoomExceptPlayer {
                connection_id,
                room_id,
                message: serialize_response(ServerResponse::PlayerJoined {
                    player_name,
                    player_id,
                }),
            })?;
        }

        ClientMessage::LeaveRoom => {
            let room_id = room_id.ok_or(AppError::ConnectionNotInRoom)?;
            let player_name = state.room_manager.leave_room(&connection_id)?;

            state.game_loop_registry.cleanup_game_loop(&room_id);
            cmd_sender.send(ConnectionCommand::SendToRoom {
                room_id,
                message: serialize_response(ServerResponse::PlayerLeft { player_name }),
            })?;
        }

        ClientMessage::PlayerReady => {
            let room_id = room_id.ok_or(AppError::ConnectionNotInRoom)?;
            let player_id = state
                .room_manager
                .get_player_id_from_connection_id(&connection_id)?;

            let ready_result = state.room_manager.ready_player(&player_id)?;

            if ready_result.game_started {
                let players_id =
                    state
                        .room_manager
                        .get_player_list(&room_id)
                        .ok_or(AppError::RoomNotFound {
                            room_id: room_id.clone(),
                        })?;

                let (turn_order, outbound_receiver) = state
                    .game_loop_registry
                    .start_game_loop(&room_id, players_id)?;

                cmd_sender.send(ConnectionCommand::SendToRoom {
                    room_id: room_id.clone(),
                    message: serialize_response(ServerResponse::RoomGameStart {
                        turn_order: turn_order.order,
                    }),
                })?;

                cmd_sender.send(ConnectionCommand::SendToAll {
                    message: serialize_response(ServerResponse::LobbyStartedGame {
                        room_id: room_id.clone(),
                    }),
                })?;

                spawn_outbound_handler(room_id, outbound_receiver, cmd_sender.clone());
            } else {
                cmd_sender.send(ConnectionCommand::SendToAll {
                    message: serialize_response(ServerResponse::PlayersReady {
                        players_ready: ready_result.players_ready,
                    }),
                })?;
            }
        }

        ClientMessage::TurnPass => {
            let room_id = room_id.ok_or(AppError::ConnectionNotInRoom)?;
            let player_id = state
                .room_manager
                .get_player_id_from_connection_id(&connection_id)?;

            let _ = state
                .game_loop_registry
                .send_game_event(&room_id, GameEvent::TurnPass { player_id });
        }
    }
    Ok(())
}
