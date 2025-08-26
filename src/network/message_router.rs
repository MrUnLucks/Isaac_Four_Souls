use std::sync::Arc;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::{mpsc, Mutex};

use crate::network::messages::{serialize_response, ClientMessage, ServerResponse};
use crate::{AppError, ConnectionCommand, GameMessageLoopRegistry, RoomManager};

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

pub async fn handle_lobby_message(
    client_message: ClientMessage,
    connection_id: &str,
    room_manager: &Arc<Mutex<RoomManager>>,
    cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    game_registry: &Arc<GameMessageLoopRegistry>,
) {
    let mut room_manager_lock = room_manager.lock().await;
    let room_id = room_manager_lock.get_player_room_from_connection_id(connection_id);
    let connection_id = connection_id.to_string();

    match route_lobby_message(
        client_message,
        room_id,
        connection_id.clone(),
        cmd_sender,
        &mut room_manager_lock,
        game_registry,
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

pub fn route_lobby_message(
    client_message: ClientMessage,
    room_id: Option<String>,
    connection_id: String,
    cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    room_manager: &mut RoomManager,
    game_registry: &Arc<GameMessageLoopRegistry>,
) -> Result<(), MessageRouterError> {
    match client_message {
        ClientMessage::Ping => {
            cmd_sender.send(ConnectionCommand::SendToPlayer {
                connection_id,
                message: serialize_response(ServerResponse::Pong),
            })?;
        }

        ClientMessage::Chat { message } => {
            let room_id = room_id.ok_or(AppError::ConnectionNotInRoom)?;
            let player_name = room_manager
                .get_player_name_from_connection_id(&connection_id)
                .ok_or(AppError::ConnectionNotInRoom)?;

            let connections_id = room_manager.get_connections_id_from_room_id(&room_id)?;

            cmd_sender.send(ConnectionCommand::SendToPlayers {
                connections_id,
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
            let (room_id, new_player_id) =
                room_manager.create_room(room_name, connection_id.clone(), first_player_name)?;

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
            let destroyed_room_id = room_manager.destroy_room(&room_id, &connection_id)?;

            // state
            //     .game_message_loop_registry
            //     .cleanup_game_message_loop(&destroyed_room_id);
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
            let player_id =
                room_manager.join_room(&room_id, connection_id.clone(), player_name.clone())?;

            cmd_sender.send(ConnectionCommand::SendToPlayer {
                connection_id: connection_id.clone(),
                message: serialize_response(ServerResponse::SelfJoined {
                    player_name: player_name.clone(),
                    player_id: player_id.clone(),
                }),
            })?;

            let connections_id = room_manager.get_connections_id_from_room_id(&room_id)?;

            cmd_sender.send(ConnectionCommand::SendToPlayers {
                connections_id,
                message: serialize_response(ServerResponse::PlayerJoined {
                    player_name,
                    player_id,
                }),
            })?;
        }

        ClientMessage::LeaveRoom => {
            let room_id = room_id.ok_or(AppError::ConnectionNotInRoom)?;
            let player_name = room_manager.leave_room(&connection_id)?;
            let connections_id = room_manager.get_connections_id_from_room_id(&room_id)?;

            // state.game_message_loop_registry.cleanup_game_message_loop(&room_id);
            cmd_sender.send(ConnectionCommand::SendToPlayers {
                connections_id,
                message: serialize_response(ServerResponse::PlayerLeft { player_name }),
            })?;
        }

        ClientMessage::PlayerReady => {
            let room_id = room_id.ok_or(AppError::ConnectionNotInRoom)?;
            let player_id = room_manager.get_player_id_from_connection_id(&connection_id)?;

            let ready_result = room_manager.ready_player(&player_id)?;

            if ready_result.game_started {
                let players_mapping = room_manager.get_players_mapping(&room_id)?;

                let turn_order = game_registry.start_game_message_loop(
                    &room_id,
                    players_mapping,
                    cmd_sender.clone(),
                )?;

                let connections_id = room_manager.get_connections_id_from_room_id(&room_id)?;

                cmd_sender.send(ConnectionCommand::SendToPlayers {
                    connections_id: connections_id.clone(),
                    message: serialize_response(ServerResponse::RoomGameStart {
                        turn_order: turn_order.order,
                    }),
                })?;

                cmd_sender.send(ConnectionCommand::SendToAll {
                    message: serialize_response(ServerResponse::LobbyStartedGame {
                        room_id: room_id.clone(),
                    }),
                })?;

                if let Some(room) = room_manager.get_room_mut(&room_id) {
                    room.set_state_in_game();
                }
            } else {
                // Not all players ready yet
                cmd_sender.send(ConnectionCommand::SendToAll {
                    message: serialize_response(ServerResponse::PlayersReady {
                        players_ready: ready_result.players_ready,
                    }),
                })?;
            }
        }

        _ => {
            return Err(MessageRouterError::App(AppError::Internal {
                message: "Route not defined".to_string(),
            }))
        }
    }
    Ok(())
}

pub fn handle_game_message(
    client_message: ClientMessage,
    connection_id: &str,
    game_registry: &Arc<GameMessageLoopRegistry>,
    cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
) {
    match route_game_message(client_message, connection_id, game_registry) {
        Ok(()) => {}
        Err(MessageRouterError::App(app_error)) => {
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

pub fn route_game_message(
    client_message: ClientMessage,
    connection_id: &str,
    game_registry: &GameMessageLoopRegistry,
) -> Result<(), MessageRouterError> {
    match client_message {
        ClientMessage::TurnPass => {
            let (_, player_id) = game_registry.get_player_info_from_connection_id(connection_id)?;

            game_registry.send_game_event_to_room_by_connection_id(
                connection_id,
                crate::game::game_message_loop::GameEvent::TurnPass { player_id },
            )?
        }
        _ => {
            return Err(MessageRouterError::App(AppError::Internal {
                message: "Route not defined".to_string(),
            }))
        }
    }
    Ok(())
}
