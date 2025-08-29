use std::sync::Arc;
use tokio::sync::mpsc;

use crate::actors::actor_registry::ActorRegistry;
use crate::actors::game_actor::GameMessage;
use crate::actors::lobby_actor::LobbyMessage;
use crate::network::messages::{ClientMessage, ClientMessageCategory};
use crate::{AppError, ConnectionCommand};

#[derive(Debug)]
pub enum ConnectionMessage {
    ClientMessage { message: ClientMessage },
    TransitionToGame { game_id: String, player_id: String },
    TransitionToLobby,
    Disconnect,
}

#[derive(Debug, Clone)]
pub enum ConnectionState {
    InLobby,
    InGame { game_id: String, player_id: String },
}

pub struct ConnectionActor {
    connection_id: String,
    state: ConnectionState,
    actor_registry: Arc<ActorRegistry>,
    cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
}

impl ConnectionActor {
    pub fn new(
        connection_id: String,
        actor_registry: Arc<ActorRegistry>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Self {
        Self {
            connection_id,
            state: ConnectionState::InLobby,
            actor_registry,
            cmd_sender,
        }
    }

    pub async fn run(&mut self, mut receiver: mpsc::UnboundedReceiver<ConnectionMessage>) {
        println!("🔌 Connection actor started for {}", self.connection_id);

        while let Some(message) = receiver.recv().await {
            match message {
                ConnectionMessage::ClientMessage { message } => {
                    if let Err(error) = self.handle_client_message(message).await {
                        eprintln!(
                            "Connection actor error for {}: {:?}",
                            self.connection_id, error
                        );
                        self.send_error_to_client(error).await;
                    }
                }
                // NEW: Handle state transitions
                ConnectionMessage::TransitionToGame { game_id, player_id } => {
                    println!(
                        "🔌 Connection {} transitioning to game {} as player {}",
                        self.connection_id, game_id, player_id
                    );
                    self.state = ConnectionState::InGame { game_id, player_id };
                }
                ConnectionMessage::TransitionToLobby => {
                    println!(
                        "🔌 Connection {} transitioning to lobby",
                        self.connection_id
                    );
                    self.state = ConnectionState::InLobby;
                }
                ConnectionMessage::Disconnect => {
                    println!(
                        "🔌 Connection actor {} received disconnect",
                        self.connection_id
                    );
                    break;
                }
            }
        }

        // Cleanup on disconnect
        self.cleanup().await;
        println!("🔌 Connection actor stopped for {}", self.connection_id);
    }

    async fn handle_client_message(&mut self, message: ClientMessage) -> Result<(), AppError> {
        println!(
            "🔌 Connection {} (state: {:?}) handling message: {:?}",
            self.connection_id, self.state, message
        );
        match message.category() {
            ClientMessageCategory::LobbyMessage => self.handle_lobby_message(message).await,
            ClientMessageCategory::GameMessage => self.handle_game_message(message).await,
        }
    }

    async fn handle_lobby_message(&mut self, message: ClientMessage) -> Result<(), AppError> {
        let lobby_message = self.convert_to_lobby_message(message)?;

        // Special handling for state transitions
        match &lobby_message {
            LobbyMessage::LeaveRoom { .. } => {
                self.state = ConnectionState::InLobby;
            }
            _ => {}
        }

        self.actor_registry.send_lobby_message(lobby_message)?;
        Ok(())
    }

    async fn handle_game_message(&mut self, message: ClientMessage) -> Result<(), AppError> {
        // Validate we're in a game
        match &self.state {
            ConnectionState::InGame { game_id, player_id } => {
                println!(
                    "🔌 Connection {} in game {} as player {}, processing game message",
                    self.connection_id, game_id, player_id
                );

                // Use connection-based game messages instead of player-based
                let game_message = self.convert_to_game_message_with_connection(message)?;
                self.actor_registry
                    .send_game_message(&self.connection_id, game_message)?;
                Ok(())
            }
            ConnectionState::InLobby => {
                println!(
                    "🔌 Connection {} is in lobby, cannot process game message",
                    self.connection_id
                );
                Err(AppError::ConnectionNotInRoom)
            }
        }
    }

    fn convert_to_lobby_message(&self, message: ClientMessage) -> Result<LobbyMessage, AppError> {
        let connection_id = self.connection_id.clone();

        match message {
            ClientMessage::Ping => Ok(LobbyMessage::Ping { connection_id }),
            ClientMessage::Chat { message } => Ok(LobbyMessage::Chat {
                connection_id,
                message,
            }),
            ClientMessage::CreateRoom {
                room_name,
                first_player_name,
            } => Ok(LobbyMessage::CreateRoom {
                connection_id,
                room_name,
                first_player_name,
            }),
            ClientMessage::DestroyRoom { room_id } => Ok(LobbyMessage::DestroyRoom {
                connection_id,
                room_id,
            }),
            ClientMessage::JoinRoom {
                player_name,
                room_id,
            } => Ok(LobbyMessage::JoinRoom {
                connection_id,
                player_name,
                room_id,
            }),
            ClientMessage::LeaveRoom => Ok(LobbyMessage::LeaveRoom { connection_id }),
            ClientMessage::PlayerReady => Ok(LobbyMessage::PlayerReady { connection_id }),
            _ => Err(AppError::Internal {
                message: "Invalid lobby message conversion".to_string(),
            }),
        }
    }

    fn convert_to_game_message_with_connection(
        &self,
        message: ClientMessage,
    ) -> Result<GameMessage, AppError> {
        match message {
            ClientMessage::TurnPass => Ok(GameMessage::TurnPassFromConnection {
                connection_id: self.connection_id.clone(),
            }),
            ClientMessage::PriorityPass => Ok(GameMessage::PriorityPassFromConnection {
                connection_id: self.connection_id.clone(),
            }),
            _ => Err(AppError::Internal {
                message: "Invalid game message conversion".to_string(),
            }),
        }
    }

    async fn send_error_to_client(&self, error: AppError) {
        use crate::network::messages::{serialize_response, ServerResponse};

        let _ = self.cmd_sender.send(ConnectionCommand::SendToPlayer {
            connection_id: self.connection_id.clone(),
            message: serialize_response(ServerResponse::from_app_error(&error)),
        });
    }

    async fn cleanup(&mut self) {
        self.actor_registry
            .remove_player_connection(&self.connection_id);

        // If we're in a game, we might want to notify other players
        if let ConnectionState::InGame { .. } = self.state {
            // In a full implementation, we might send a "player disconnected" message
            // to the game actor, but for now the game will handle it via normal cleanup
        }
    }

    // PUBLIC METHODS for state management from other actors

    pub fn transition_to_game(&mut self, game_id: String, player_id: String) {
        println!(
            "🔌 Connection {} transitioning to game {}",
            self.connection_id, game_id
        );
        self.state = ConnectionState::InGame { game_id, player_id };
    }

    pub fn transition_to_lobby(&mut self) {
        println!(
            "🔌 Connection {} transitioning to lobby",
            self.connection_id
        );
        self.state = ConnectionState::InLobby;
    }

    pub fn get_state(&self) -> &ConnectionState {
        &self.state
    }
}
