use std::sync::Arc;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::{mpsc, Mutex};

use crate::game::event_handler::GameEvent;
use crate::network::messages::{serialize_response, ClientMessage, ServerResponse};
use crate::{AppError, ConnectionCommand, GameMessageLoopRegistry};

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
                GameEvent::TurnPass { player_id },
            )?
        }
        ClientMessage::PriorityPass => {
            let (_, player_id) = game_registry.get_player_info_from_connection_id(connection_id)?;

            game_registry.send_game_event_to_room_by_connection_id(
                connection_id,
                GameEvent::PriorityPass { player_id },
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
