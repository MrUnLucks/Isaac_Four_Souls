use crate::game::outbound_handler::spawn_outbound_handler;
use std::sync::Arc;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::{mpsc, Mutex};

use crate::game::game_loop::GameEvent;
use crate::network::messages::{
    deserialize_message, serialize_response, ClientMessage, ServerResponse,
};
use crate::{AppError, ConnectionCommand, LobbyState};

pub async fn handle_text_message(
    text: String,
    connection_id: &str,
    lobby_state: &Arc<Mutex<LobbyState>>,
    cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
) {
    let game_message = match deserialize_message(&text) {
        Ok(msg) => msg,
        Err(_) => {
            cmd_sender
                .send(ConnectionCommand::SendToPlayer {
                    connection_id: connection_id.to_string(),
                    message: serialize_response(ServerResponse::from_app_error(
                        &AppError::SerializationError {
                            message: "Failed to deserialize message".to_string(),
                        },
                    )),
                })
                .expect("Send error on deserialization message, panicking...");

            return;
        }
    };

    let mut state = lobby_state.lock().await;
    let room_id = state
        .room_manager
        .get_player_room_from_connection_id(connection_id);
    let connection_id = connection_id.to_string();

    handle_message(game_message, room_id, connection_id, cmd_sender, &mut state)
        .expect("Critical send error, panicking tokio thread...");
}

pub fn handle_message(
    game_message: ClientMessage,
    room_id: Option<String>,
    connection_id: String,
    cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
    state: &mut LobbyState,
) -> Result<(), SendError<ConnectionCommand>> {
    match game_message {
        ClientMessage::Ping => {
            cmd_sender.send(ConnectionCommand::SendToPlayer {
                connection_id,
                message: serialize_response(ServerResponse::Pong),
            })?;
        }
        ClientMessage::Chat { message } => match room_id {
            None => {
                cmd_sender.send(ConnectionCommand::SendToPlayer {
                    connection_id,
                    message: serialize_response(ServerResponse::from_app_error(
                        &AppError::RoomNotFound {
                            room_id: "".to_string(),
                        },
                    )),
                })?;
            }
            Some(room_id) => {
                let player_name = state
                    .room_manager
                    .get_player_name_from_connection_id(&connection_id);
                match player_name {
                    Some(player_name) => {
                        cmd_sender.send(ConnectionCommand::SendToRoom {
                            room_id,
                            message: serialize_response(ServerResponse::ChatMessage {
                                player_name,
                                message,
                            }),
                        })?;
                    }
                    None => {
                        cmd_sender.send(ConnectionCommand::SendToPlayer {
                            connection_id,
                            message: serialize_response(ServerResponse::from_app_error(
                                &AppError::ConnectionNotInRoom,
                            )),
                        })?;
                    }
                }
            }
        },
        ClientMessage::CreateRoom {
            room_name,
            first_player_name,
        } => {
            match state.room_manager.create_room(
                room_name,
                connection_id.to_string(),
                first_player_name,
            ) {
                Err(app_error) => {
                    cmd_sender.send(ConnectionCommand::SendToPlayer {
                        connection_id,
                        message: serialize_response(ServerResponse::from_app_error(&app_error)),
                    })?;
                }
                Ok((room_id, new_player_id)) => {
                    cmd_sender.send(ConnectionCommand::SendToPlayer {
                        connection_id,
                        message: serialize_response(ServerResponse::RoomCreated {
                            room_id: room_id.clone(),
                            player_id: new_player_id,
                        }),
                    })?;
                    cmd_sender.send(ConnectionCommand::SendToAll {
                        message: serialize_response(ServerResponse::RoomCreatedBroadcast {
                            room_id,
                        }),
                    })?;
                }
            }
        }
        ClientMessage::DestroyRoom { room_id } => {
            match state.room_manager.destroy_room(&room_id, &connection_id) {
                Err(app_error) => {
                    cmd_sender.send(ConnectionCommand::SendToPlayer {
                        connection_id,
                        message: serialize_response(ServerResponse::from_app_error(&app_error)),
                    })?;
                }
                Ok(room_id) => {
                    state.game_loop_registry.cleanup_game_loop(&room_id);
                    cmd_sender.send(ConnectionCommand::SendToAll {
                        message: serialize_response(ServerResponse::RoomDestroyed { room_id }),
                    })?;
                }
            }
        }
        ClientMessage::JoinRoom {
            player_name,
            room_id,
        } => {
            let join_room_result =
                state
                    .room_manager
                    .join_room(&room_id, connection_id.clone(), player_name.clone());
            match join_room_result {
                Err(app_error) => {
                    cmd_sender.send(ConnectionCommand::SendToPlayer {
                        connection_id: connection_id.clone(),
                        message: serialize_response(ServerResponse::from_app_error(&app_error)),
                    })?;
                }
                Ok(player_id) => {
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
            };
        }
        ClientMessage::LeaveRoom => match room_id {
            None => {
                cmd_sender.send(ConnectionCommand::SendToPlayer {
                    connection_id,
                    message: serialize_response(ServerResponse::from_app_error(
                        &AppError::RoomNotFound {
                            room_id: "".to_string(),
                        },
                    )),
                })?;
            }
            Some(room_id) => {
                let leave_room_result = state.room_manager.leave_room(&connection_id);
                match leave_room_result {
                    Err(app_error) => {
                        cmd_sender.send(ConnectionCommand::SendToPlayer {
                            connection_id,
                            message: serialize_response(ServerResponse::from_app_error(&app_error)),
                        })?;
                    }
                    Ok(player_name) => {
                        state.game_loop_registry.cleanup_game_loop(&room_id);
                        cmd_sender.send(ConnectionCommand::SendToRoom {
                            room_id,
                            message: serialize_response(ServerResponse::PlayerLeft { player_name }),
                        })?;
                    }
                }
            }
        },
        ClientMessage::PlayerReady => {
            let player_id = state
                .room_manager
                .get_player_id_from_connection_id(&connection_id);
            let Some(room_id) = room_id.as_ref() else {
                cmd_sender.send(ConnectionCommand::SendToPlayer {
                    connection_id,
                    message: serialize_response(ServerResponse::from_app_error(
                        &AppError::ConnectionNotInRoom,
                    )),
                })?;
                return Ok(());
            };
            match player_id {
                Err(app_error) => {
                    cmd_sender.send(ConnectionCommand::SendToPlayer {
                        connection_id,
                        message: serialize_response(ServerResponse::from_app_error(&app_error)),
                    })?;
                }
                Ok(player_id) => {
                    let ready_result = state.room_manager.ready_player(&player_id);
                    match ready_result {
                        Err(app_error) => {
                            cmd_sender.send(ConnectionCommand::SendToPlayer {
                                connection_id,
                                message: serialize_response(ServerResponse::from_app_error(
                                    &app_error,
                                )),
                            })?;
                        }
                        Ok(ready_result) => {
                            if ready_result.game_started {
                                let players_id = state.room_manager.get_player_list(room_id);
                                match players_id {
                                    None => {
                                        cmd_sender.send(ConnectionCommand::SendToPlayer {
                                            connection_id,
                                            message: serialize_response(
                                                ServerResponse::from_app_error(
                                                    &AppError::RoomNotFound {
                                                        room_id: room_id.to_string(),
                                                    },
                                                ),
                                            ),
                                        })?;
                                    }
                                    Some(players_id) => {
                                        let game_start_result = state
                                            .game_loop_registry
                                            .start_game_loop(&room_id, players_id);
                                        match game_start_result {
                                            Err(app_error) => {
                                                cmd_sender.send(
                                                    ConnectionCommand::SendToPlayer {
                                                        connection_id,
                                                        message: serialize_response(
                                                            ServerResponse::from_app_error(
                                                                &app_error,
                                                            ),
                                                        ),
                                                    },
                                                )?;
                                            }
                                            Ok((turn_order, outbound_receiver)) => {
                                                cmd_sender.send(ConnectionCommand::SendToRoom {
                                                    room_id: room_id.to_string(),
                                                    message: serialize_response(
                                                        ServerResponse::RoomGameStart {
                                                            turn_order: turn_order.order,
                                                        },
                                                    ),
                                                })?;
                                                cmd_sender.send(ConnectionCommand::SendToAll {
                                                    message: serialize_response(
                                                        ServerResponse::LobbyStartedGame {
                                                            room_id: room_id.to_string(),
                                                        },
                                                    ),
                                                })?;
                                                spawn_outbound_handler(
                                                    room_id.to_string(),
                                                    outbound_receiver,
                                                    cmd_sender.clone(),
                                                );
                                            }
                                        }
                                    }
                                }
                            } else {
                                cmd_sender.send(ConnectionCommand::SendToAll {
                                    message: serialize_response(ServerResponse::PlayersReady {
                                        players_ready: ready_result.players_ready,
                                    }),
                                })?;
                            }
                        }
                    }
                }
            }
        }
        ClientMessage::TurnPass => {
            let player_id = state
                .room_manager
                .get_player_id_from_connection_id(&connection_id);
            let Some(room_id) = room_id.as_ref() else {
                cmd_sender.send(ConnectionCommand::SendToPlayer {
                    connection_id,
                    message: serialize_response(ServerResponse::from_app_error(
                        &AppError::ConnectionNotInRoom,
                    )),
                })?;
                return Ok(());
            };
            match player_id {
                Err(app_error) => {
                    cmd_sender.send(ConnectionCommand::SendToPlayer {
                        connection_id,
                        message: serialize_response(ServerResponse::from_app_error(&app_error)),
                    })?;
                }
                Ok(player_id) => {
                    let _ = state
                        .game_loop_registry
                        .send_game_event(room_id, GameEvent::TurnPass { player_id });
                }
            }
        }
    };
    Ok(())
}
