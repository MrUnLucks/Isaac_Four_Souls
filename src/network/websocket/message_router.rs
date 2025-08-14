use std::sync::Arc;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::{mpsc, Mutex};

use crate::network::lobby::LobbyState;
use crate::network::messages::{
    deserialize_message, serialize_response, ClientMessage, ServerResponse,
};
use crate::network::websocket::connection_commands::ConnectionCommand;
use crate::AppError;

pub async fn handle_text_message(
    text: String,
    connection_id: &str,
    lobby_state: &Arc<Mutex<LobbyState>>,
    cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
) {
    let game_message = match deserialize_message(&text) {
        Ok(msg) => msg,
        Err(_) => {
            let error_response = ServerResponse::from_app_error(&AppError::SerializationError {
                message: "Failed to deserialize message".to_string(),
            });

            cmd_sender
                .send(ConnectionCommand::SendToPlayer {
                    connection_id: connection_id.to_string(),
                    message: serialize_response(error_response),
                })
                .unwrap();

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

    // Ok(())
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
                cmd_sender.send(ConnectionCommand::SendToRoom { room_id, message })?;
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
                        Ok(ready_player_result) => {
                            if ready_player_result.game_started {
                                cmd_sender.send(ConnectionCommand::SendToRoom {
                                    room_id: room_id.clone(), //Check on readyplayer, safe to call
                                    message: serialize_response(ServerResponse::GameStarted {
                                        room_id: room_id.clone(), //Check on readyplayer, safe to call
                                        turn_order: ready_player_result.turn_order.unwrap(), //Check on readyplayer, safe to call
                                    }),
                                })?;
                            } else {
                                cmd_sender.send(ConnectionCommand::SendToPlayer {
                                    connection_id,
                                    message: serialize_response(ServerResponse::PlayersReady {
                                        players_ready: ready_player_result.players_ready,
                                    }),
                                })?
                            };
                        }
                    };
                }
            }
        }
        ClientMessage::TurnPass => {
            let pass_turn_result = state.room_manager.pass_turn(&connection_id);
            match pass_turn_result {
                Err(app_error) => {
                    cmd_sender.send(ConnectionCommand::SendToPlayer {
                        connection_id,
                        message: serialize_response(ServerResponse::from_app_error(&app_error)),
                    })?;
                }
                Ok(next_player_id) => {
                    cmd_sender.send(ConnectionCommand::SendToRoom {
                        room_id: room_id.unwrap(), //Check on turn_pass, safe to call
                        message: serialize_response(ServerResponse::TurnChange { next_player_id }),
                    })?;
                }
            }
        }
    };
    Ok(())
}
