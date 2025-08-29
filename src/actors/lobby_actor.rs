use std::sync::Arc;
use tokio::sync::mpsc;

use crate::network::messages::{serialize_response, ServerResponse};
use crate::{AppError, ConnectionCommand, GameMessageLoopRegistry, RoomManager};

#[derive(Debug)]
pub enum LobbyMessage {
    Ping {
        connection_id: String,
    },
    Chat {
        connection_id: String,
        message: String,
    },
    CreateRoom {
        connection_id: String,
        room_name: String,
        first_player_name: String,
    },
    DestroyRoom {
        connection_id: String,
        room_id: String,
    },
    JoinRoom {
        connection_id: String,
        player_name: String,
        room_id: String,
    },
    LeaveRoom {
        connection_id: String,
    },
    PlayerReady {
        connection_id: String,
    },
}

pub struct LobbyActor {
    room_manager: RoomManager,
    game_registry: Arc<GameMessageLoopRegistry>,
    cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
}

impl LobbyActor {
    pub fn new(
        game_registry: Arc<GameMessageLoopRegistry>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Self {
        Self {
            room_manager: RoomManager::new(),
            game_registry,
            cmd_sender,
        }
    }

    pub async fn run(&mut self, mut receiver: mpsc::UnboundedReceiver<LobbyMessage>) {
        println!("🏛️ Lobby actor started");

        while let Some(message) = receiver.recv().await {
            if let Err(error) = self.handle_message(message).await {
                eprintln!("Lobby actor error: {:?}", error);
            }
        }

        println!("🏛️ Lobby actor stopped");
    }

    async fn handle_message(&mut self, message: LobbyMessage) -> Result<(), AppError> {
        match message {
            LobbyMessage::Ping { connection_id } => {
                self.cmd_sender.send(ConnectionCommand::SendToPlayer {
                    connection_id,
                    message: serialize_response(ServerResponse::Pong),
                })?;
            }

            LobbyMessage::Chat {
                connection_id,
                message,
            } => {
                let room_id = self
                    .room_manager
                    .get_player_room_from_connection_id(&connection_id)
                    .ok_or(AppError::ConnectionNotInRoom)?;

                let player_name = self
                    .room_manager
                    .get_player_name_from_connection_id(&connection_id)
                    .ok_or(AppError::ConnectionNotInRoom)?;

                let connections_id = self
                    .room_manager
                    .get_connections_id_from_room_id(&room_id)?;

                self.cmd_sender.send(ConnectionCommand::SendToPlayers {
                    connections_id,
                    message: serialize_response(ServerResponse::ChatMessage {
                        player_name,
                        message,
                    }),
                })?;
            }

            LobbyMessage::CreateRoom {
                connection_id,
                room_name,
                first_player_name,
            } => {
                let (room_id, new_player_id) = self.room_manager.create_room(
                    room_name,
                    connection_id.clone(),
                    first_player_name,
                )?;

                self.cmd_sender.send(ConnectionCommand::SendToPlayer {
                    connection_id,
                    message: serialize_response(ServerResponse::RoomCreated {
                        room_id: room_id.clone(),
                        player_id: new_player_id,
                    }),
                })?;

                self.cmd_sender.send(ConnectionCommand::SendToAll {
                    message: serialize_response(ServerResponse::RoomCreatedBroadcast { room_id }),
                })?;
            }

            LobbyMessage::DestroyRoom {
                connection_id,
                room_id,
            } => {
                let destroyed_room_id = self.room_manager.destroy_room(&room_id, &connection_id)?;

                self.cmd_sender.send(ConnectionCommand::SendToAll {
                    message: serialize_response(ServerResponse::RoomDestroyed {
                        room_id: destroyed_room_id,
                    }),
                })?;
            }

            LobbyMessage::JoinRoom {
                connection_id,
                player_name,
                room_id,
            } => {
                let player_id = self.room_manager.join_room(
                    &room_id,
                    connection_id.clone(),
                    player_name.clone(),
                )?;

                self.cmd_sender.send(ConnectionCommand::SendToPlayer {
                    connection_id: connection_id.clone(),
                    message: serialize_response(ServerResponse::SelfJoined {
                        player_name: player_name.clone(),
                        player_id: player_id.clone(),
                    }),
                })?;

                let connections_id = self
                    .room_manager
                    .get_connections_id_from_room_id(&room_id)?;

                self.cmd_sender.send(ConnectionCommand::SendToPlayers {
                    connections_id,
                    message: serialize_response(ServerResponse::PlayerJoined {
                        player_name,
                        player_id,
                    }),
                })?;
            }

            LobbyMessage::LeaveRoom { connection_id } => {
                let room_id = self
                    .room_manager
                    .get_player_room_from_connection_id(&connection_id)
                    .ok_or(AppError::ConnectionNotInRoom)?;

                let player_name = self.room_manager.leave_room(&connection_id)?;
                let connections_id = self
                    .room_manager
                    .get_connections_id_from_room_id(&room_id)?;

                self.cmd_sender.send(ConnectionCommand::SendToPlayers {
                    connections_id,
                    message: serialize_response(ServerResponse::PlayerLeft { player_name }),
                })?;
            }

            LobbyMessage::PlayerReady { connection_id } => {
                let room_id = self
                    .room_manager
                    .get_player_room_from_connection_id(&connection_id)
                    .ok_or(AppError::ConnectionNotInRoom)?;

                let player_id = self
                    .room_manager
                    .get_player_id_from_connection_id(&connection_id)?;
                let ready_result = self.room_manager.ready_player(&player_id)?;

                // Short circuit for faster testing
                if true {
                    let players_mapping = self.room_manager.get_players_mapping(&room_id)?;

                    let turn_order = self.game_registry.start_game_message_loop(
                        &room_id,
                        players_mapping,
                        self.cmd_sender.clone(),
                    )?;

                    let connections_id = self
                        .room_manager
                        .get_connections_id_from_room_id(&room_id)?;

                    self.cmd_sender.send(ConnectionCommand::SendToPlayers {
                        connections_id: connections_id.clone(),
                        message: serialize_response(ServerResponse::RoomGameStart {
                            turn_order: turn_order.order,
                        }),
                    })?;

                    self.cmd_sender.send(ConnectionCommand::SendToAll {
                        message: serialize_response(ServerResponse::LobbyStartedGame {
                            room_id: room_id.clone(),
                        }),
                    })?;

                    if let Some(room) = self.room_manager.get_room_mut(&room_id) {
                        room.set_state_in_game();
                    }
                } else {
                    self.cmd_sender.send(ConnectionCommand::SendToAll {
                        message: serialize_response(ServerResponse::PlayersReady {
                            players_ready: ready_result.players_ready,
                        }),
                    })?;
                }
            }
        }
        Ok(())
    }
}

impl From<mpsc::error::SendError<ConnectionCommand>> for AppError {
    fn from(_: mpsc::error::SendError<ConnectionCommand>) -> Self {
        AppError::Internal {
            message: "Failed to send connection command".to_string(),
        }
    }
}
