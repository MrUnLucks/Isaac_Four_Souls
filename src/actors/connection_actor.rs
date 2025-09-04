use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::actors::actor_registry::ActorRegistry;
use crate::actors::game_actor::GameMessage;
use crate::actors::lobby_actor::LobbyMessage;
use crate::network::messages::{ClientMessage, ClientMessageCategory, ServerResponse};
use crate::network::reliable_messaging::{
    create_reliable_message, MessageAck, MessageReceiver, PendingMessage, ReliableMessage,
};
use crate::{AppError, AppResult, ConnectionCommand};

#[derive(Debug)]
pub enum ConnectionMessage {
    ClientMessage { message: ClientMessage },
    TransitionToGame { game_id: String, player_id: String },
    TransitionToLobby,
    Disconnect,
    ReliableMessage { message: ReliableMessage },
    MessageAck { ack: MessageAck },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ReliableServerResponse {
    Reliable(ReliableMessage),
    Ack(MessageAck),
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

    message_receiver: MessageReceiver,
    pending_messages: HashMap<String, PendingMessage>,
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
            message_receiver: MessageReceiver::new(),
            pending_messages: HashMap::new(),
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
                ConnectionMessage::ReliableMessage { message } => {
                    if let Err(error) = self.handle_reliable_message(message).await {
                        eprintln!(
                            "Reliable message error for {}: {:?}",
                            self.connection_id, error
                        );
                    }
                }
                ConnectionMessage::MessageAck { ack } => {
                    self.handle_ack(ack);
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

    async fn handle_client_message(&mut self, message: ClientMessage) -> AppResult<()> {
        println!(
            "🔌 Connection {} (state: {:?}) handling message: {:?}",
            self.connection_id, self.state, message
        );
        match message.category() {
            ClientMessageCategory::LobbyMessage => self.handle_lobby_message(message).await,
            ClientMessageCategory::GameMessage => self.handle_game_message(message).await,
        }
    }

    async fn send_message_now(&self, message: ReliableMessage) {
        let wrapper = ReliableServerResponse::Reliable(message);
        let serialized = serde_json::to_string(&wrapper).unwrap();

        let _ = self.cmd_sender.send(ConnectionCommand::SendToPlayer {
            connection_id: self.connection_id.clone(),
            message: serialized,
        });
    }

    pub async fn send_reliable(&mut self, payload: String) {
        let message = create_reliable_message(payload);

        // Try to send, retry up to 3 times immediately
        for _ in 1..=3 {
            self.send_message_now(message.clone()).await;

            // Wait a bit and see if we get an ack
            tokio::time::sleep(Duration::from_millis(500)).await;

            // If message was acked, we're done
            if !self.pending_messages.contains_key(&message.id) {
                return;
            }
        }
    }

    pub async fn handle_reliable_message(&mut self, message: ReliableMessage) -> AppResult<()> {
        let (ack, ordered_messages) = self.message_receiver.receive_message(message);

        // Send ack
        self.send_ack(ack).await;

        // Process ordered messages
        for msg in ordered_messages {
            if let Ok(client_message) = serde_json::from_str(&msg.payload) {
                self.handle_client_message(client_message).await?;
            }
        }

        Ok(())
    }

    async fn send_ack(&self, ack: MessageAck) {
        let wrapper = ReliableServerResponse::Ack(ack);
        let serialized = serde_json::to_string(&wrapper).unwrap();

        let _ = self.cmd_sender.send(ConnectionCommand::SendToPlayer {
            connection_id: self.connection_id.clone(),
            message: serialized,
        });
    }

    pub fn handle_ack(&mut self, ack: MessageAck) {
        if self.pending_messages.remove(&ack.message_id).is_some() {
            println!("✅ Message {} acknowledged", ack.message_id);
        }
    }

    async fn handle_lobby_message(&mut self, message: ClientMessage) -> AppResult<()> {
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

    async fn handle_game_message(&mut self, message: ClientMessage) -> AppResult<()> {
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

    fn convert_to_lobby_message(&self, message: ClientMessage) -> AppResult<LobbyMessage> {
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
    ) -> AppResult<GameMessage> {
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

    pub async fn send_reliable_game_state(
        connection_actor: &mut ConnectionActor,
        state: &ServerResponse,
    ) {
        let payload = serde_json::to_string(state).unwrap();
        connection_actor.send_reliable(payload).await;
    }

    pub async fn send_unreliable_chat(
        cmd_sender: &mpsc::UnboundedSender<ConnectionCommand>,
        connection_id: &str,
        message: &ServerResponse,
    ) {
        let serialized = serde_json::to_string(message).unwrap();
        let _ = cmd_sender.send(ConnectionCommand::SendToPlayer {
            connection_id: connection_id.to_string(),
            message: serialized,
        });
    }
}
