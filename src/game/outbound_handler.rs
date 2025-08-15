use crate::network::connection_commands::ConnectionCommand;
use crate::network::messages::{serialize_response, ServerResponse};
use crate::AppError;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum GameOutboundEvent {
    TurnChanged {
        next_player_id: String,
    },
    GameEnded {
        winner_id: String,
    },
    GameError {
        error: crate::game::game_loop::GameError,
    },
}

pub struct OutboundEventHandler {
    room_id: String,
    receiver: mpsc::Receiver<GameOutboundEvent>,
    cmd_sender: mpsc::UnboundedSender<ConnectionCommand>, //TODO: need refactor, no need to put network responsibility on the outbound_handler
}

impl OutboundEventHandler {
    pub fn new(
        room_id: String,
        receiver: mpsc::Receiver<GameOutboundEvent>,
        cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
    ) -> Self {
        Self {
            room_id,
            receiver,
            cmd_sender,
        }
    }

    pub fn spawn(mut self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.run().await;
        })
    }

    async fn run(&mut self) {
        println!(
            "🎯 Starting outbound event handler for room {}",
            self.room_id
        );

        while let Some(event) = self.receiver.recv().await {
            if let Err(e) = self.handle_event(event).await {
                eprintln!(
                    "❌ Error handling outbound event for room {}: {}",
                    self.room_id, e
                );
            }
        }

        println!(
            "🏁 Outbound event handler for room {} finished",
            self.room_id
        );
    }

    async fn handle_event(&self, event: GameOutboundEvent) -> Result<(), String> {
        let message = self.event_to_server_response(event);

        self.cmd_sender
            .send(ConnectionCommand::SendToRoom {
                room_id: self.room_id.clone(),
                message: serialize_response(message),
            })
            .map_err(|e| format!("Failed to send command: {}", e))
    }

    fn event_to_server_response(&self, event: GameOutboundEvent) -> ServerResponse {
        match event {
            GameOutboundEvent::TurnChanged { next_player_id } => {
                ServerResponse::TurnChange { next_player_id }
            }
            GameOutboundEvent::GameError { error } => {
                ServerResponse::from_app_error(&AppError::Internal {
                    message: format!("Game error: {:?}", error),
                })
            }
            GameOutboundEvent::GameEnded { winner_id } => ServerResponse::GameEnded { winner_id },
        }
    }
}

pub fn spawn_outbound_handler(
    room_id: String,
    receiver: mpsc::Receiver<GameOutboundEvent>,
    cmd_sender: mpsc::UnboundedSender<ConnectionCommand>,
) -> tokio::task::JoinHandle<()> {
    let handler = OutboundEventHandler::new(room_id, receiver, cmd_sender);
    handler.spawn()
}
