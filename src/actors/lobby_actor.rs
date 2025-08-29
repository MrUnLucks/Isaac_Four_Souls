// ENHANCEMENT: Move RoomManager functionality directly into LobbyActor
// This eliminates the last piece of shared state

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::actors::actor_registry::ActorRegistry;
use crate::network::messages::{serialize_response, ServerResponse};
use crate::{AppError, ConnectionCommand, Room};

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

#[derive(Debug, Clone)]
struct PlayerRoomInfo {
    room_id: String,
    room_player_id: String,
    player_name: String,
}

pub struct LobbyActor {
    // MOVED: All RoomManager state directly into LobbyActor
    rooms: HashMap<String, Room>,
    connection_to_room_info: HashMap<String, PlayerRoomInfo>,
    rooms_connections_map: HashMap<String, HashSet<String>>,

    actor_registry: Arc<ActorRegistry>,
    cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
}

impl LobbyActor {
    pub fn new(
        actor_registry: Arc<ActorRegistry>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Self {
        Self {
            rooms: HashMap::new(),
            connection_to_room_info: HashMap::new(),
            rooms_connections_map: HashMap::new(),
            actor_registry,
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
                    .get_player_room_from_connection_id(&connection_id)
                    .ok_or(AppError::ConnectionNotInRoom)?;

                let player_name = self
                    .get_player_name_from_connection_id(&connection_id)
                    .ok_or(AppError::ConnectionNotInRoom)?;

                let connections_id = self.get_connections_id_from_room_id(&room_id)?;

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
                let (room_id, new_player_id) =
                    self.create_room(room_name, connection_id.clone(), first_player_name)?;

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
                let destroyed_room_id = self.destroy_room(&room_id, &connection_id)?;

                self.actor_registry.cleanup_game_actor(&destroyed_room_id);

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
                let player_id =
                    self.join_room(&room_id, connection_id.clone(), player_name.clone())?;

                self.cmd_sender.send(ConnectionCommand::SendToPlayer {
                    connection_id: connection_id.clone(),
                    message: serialize_response(ServerResponse::SelfJoined {
                        player_name: player_name.clone(),
                        player_id: player_id.clone(),
                    }),
                })?;

                let connections_id = self.get_connections_id_from_room_id(&room_id)?;

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
                    .get_player_room_from_connection_id(&connection_id)
                    .ok_or(AppError::ConnectionNotInRoom)?;

                let player_name = self.leave_room(&connection_id)?;
                let connections_id = self.get_connections_id_from_room_id(&room_id)?;

                self.cmd_sender.send(ConnectionCommand::SendToPlayers {
                    connections_id,
                    message: serialize_response(ServerResponse::PlayerLeft { player_name }),
                })?;
            }

            LobbyMessage::PlayerReady { connection_id } => {
                let room_id = self
                    .get_player_room_from_connection_id(&connection_id)
                    .ok_or(AppError::ConnectionNotInRoom)?;

                let player_id = self.get_player_id_from_connection_id(&connection_id)?;
                let ready_result = self.ready_player(&player_id)?;

                if true {
                    // Game start condition
                    let players_mapping = self.get_players_mapping(&room_id)?;

                    // DEBUG: Log the players mapping
                    println!(
                        "🏛️ Starting game for room {} with players: {:?}",
                        room_id, players_mapping
                    );

                    let turn_order = self.actor_registry.start_game_actor(
                        room_id.clone(),
                        players_mapping.clone(), // Clone for notification use
                        self.cmd_sender.clone(),
                    )?;

                    for (player_id, connection_id) in &players_mapping {
                        println!(
                            "🏛️ Notifying connection {} that they are player {} in game {}",
                            connection_id, player_id, room_id
                        );

                        if let Err(e) = self.actor_registry.notify_connection_game_start(
                            connection_id,
                            room_id.clone(),
                            player_id.clone(),
                        ) {
                            eprintln!(
                                "Failed to notify connection {} of game start: {:?}",
                                connection_id, e
                            );
                        }
                    }

                    let connections_id = self.get_connections_id_from_room_id(&room_id)?;

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

                    if let Some(room) = self.rooms.get_mut(&room_id) {
                        room.set_state_in_game();
                    }
                } else {
                    self.cmd_sender.send(ConnectionCommand::SendToAll {
                        message: serialize_response(ServerResponse::PlayersReady {
                            players_ready: ready_result,
                        }),
                    })?;
                }
            }
        }
        Ok(())
    }

    // MOVED: All RoomManager methods directly into LobbyActor
    fn create_room(
        &mut self,
        room_name: String,
        first_player_connection_id: String,
        first_player_name: String,
    ) -> Result<(String, String), AppError> {
        if room_name.trim().is_empty() {
            return Err(AppError::RoomNameEmpty);
        }
        if self
            .connection_to_room_info
            .contains_key(&first_player_connection_id)
        {
            return Err(AppError::PlayerAlreadyInRoom {
                player_name: first_player_name,
            });
        }

        let mut room = Room::new(room_name);
        let new_player_id = room.add_player(first_player_name.clone())?;
        let room_id = room.get_id();

        self.connection_to_room_info.insert(
            first_player_connection_id.clone(),
            PlayerRoomInfo {
                room_id: room_id.clone(),
                room_player_id: new_player_id.clone(),
                player_name: first_player_name,
            },
        );
        self.rooms_connections_map
            .insert(room_id.clone(), HashSet::from([first_player_connection_id]));
        self.rooms.insert(room_id.clone(), room);

        Ok((room_id, new_player_id))
    }

    fn get_player_room_from_connection_id(&self, connection_id: &str) -> Option<String> {
        self.connection_to_room_info
            .get(connection_id)
            .map(|info| info.room_id.clone())
    }

    fn get_player_name_from_connection_id(&self, connection_id: &str) -> Option<String> {
        self.connection_to_room_info
            .get(connection_id)
            .map(|info| info.player_name.clone())
    }

    fn get_player_id_from_connection_id(&self, connection_id: &str) -> Result<String, AppError> {
        self.connection_to_room_info
            .get(connection_id)
            .ok_or(AppError::ConnectionNotInRoom)
            .map(|info| info.room_player_id.clone())
    }

    fn get_connections_id_from_room_id(&self, room_id: &str) -> Result<Vec<String>, AppError> {
        self.rooms_connections_map
            .get(room_id)
            .ok_or(AppError::RoomNotFound {
                room_id: room_id.to_string(),
            })
            .map(|connections| connections.iter().cloned().collect())
    }

    fn join_room(
        &mut self,
        room_id: &str,
        connection_id: String,
        player_name: String,
    ) -> Result<String, AppError> {
        if self.connection_to_room_info.contains_key(&connection_id) {
            return Err(AppError::PlayerAlreadyInRoom { player_name });
        }

        let room = self.rooms.get_mut(room_id).ok_or(AppError::RoomNotFound {
            room_id: room_id.to_string(),
        })?;
        let new_player_id = room.add_player(player_name.clone())?;

        self.connection_to_room_info.insert(
            connection_id.clone(),
            PlayerRoomInfo {
                room_id: room_id.to_string(),
                room_player_id: new_player_id.clone(),
                player_name,
            },
        );
        self.rooms_connections_map
            .entry(room_id.to_string())
            .or_insert_with(HashSet::new)
            .insert(connection_id);

        Ok(new_player_id)
    }

    fn leave_room(&mut self, connection_id: &str) -> Result<String, AppError> {
        let PlayerRoomInfo {
            room_id,
            room_player_id,
            player_name: _,
        } = self
            .connection_to_room_info
            .remove(connection_id)
            .ok_or(AppError::ConnectionNotInRoom)?;

        let room = self.rooms.get_mut(&room_id).ok_or(AppError::RoomNotFound {
            room_id: room_id.clone(),
        })?;

        let connection_set =
            self.rooms_connections_map
                .get_mut(&room_id)
                .ok_or(AppError::RoomNotFound {
                    room_id: room_id.clone(),
                })?;
        connection_set.remove(connection_id);

        let removed_player_name = room.remove_player(&room_player_id)?;

        if room.player_count() == 0 {
            self.rooms.remove(&room_id);
        }

        Ok(removed_player_name)
    }

    fn destroy_room(&mut self, room_id: &str, connection_id: &str) -> Result<String, AppError> {
        self.connection_to_room_info
            .remove(connection_id)
            .ok_or(AppError::ConnectionNotInRoom)?;

        let connection_set =
            self.rooms_connections_map
                .get_mut(room_id)
                .ok_or(AppError::RoomNotFound {
                    room_id: room_id.to_string(),
                })?;
        connection_set.remove(connection_id);

        self.rooms.remove(room_id).ok_or(AppError::RoomNotFound {
            room_id: room_id.to_string(),
        })?;

        Ok(room_id.to_string())
    }

    fn ready_player(&mut self, player_id: &str) -> Result<HashSet<String>, AppError> {
        let room_id = self.get_player_room_from_player_id(player_id)?;

        let room = self.rooms.get_mut(&room_id).ok_or(AppError::RoomNotFound {
            room_id: room_id.clone(),
        })?;

        room.add_player_ready(player_id)
    }

    fn get_player_room_from_player_id(&self, player_id: &str) -> Result<String, AppError> {
        self.connection_to_room_info
            .values()
            .find(|info| info.room_player_id == player_id)
            .map(|info| info.room_id.clone())
            .ok_or(AppError::ConnectionNotInRoom)
    }

    fn get_players_mapping(&self, room_id: &str) -> Result<HashMap<String, String>, AppError> {
        let mut players_mapping = HashMap::new();

        for (connection_id, player_info) in &self.connection_to_room_info {
            if player_info.room_id == *room_id {
                players_mapping.insert(player_info.room_player_id.clone(), connection_id.clone());
            }
        }

        if players_mapping.is_empty() {
            Err(AppError::RoomNotFound {
                room_id: room_id.to_string(),
            })
        } else {
            Ok(players_mapping)
        }
    }
}

impl From<mpsc::error::SendError<ConnectionCommand>> for AppError {
    fn from(_: mpsc::error::SendError<ConnectionCommand>) -> Self {
        AppError::Internal {
            message: "Failed to send connection command".to_string(),
        }
    }
}
