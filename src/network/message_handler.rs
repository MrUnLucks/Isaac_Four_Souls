use crate::{
    network::messages::{ClientMessage, ServerResponse},
    AppError, AppResult, RoomManager,
};

pub fn handle_message(
    msg: ClientMessage,
    room_manager: &mut RoomManager,
    connection_id: &str,
) -> AppResult<ServerResponse> {
    match msg {
        ClientMessage::Ping => Ok(ServerResponse::Pong),

        // This may need to be moved inside room_manager
        ClientMessage::Chat { message } => {
            match room_manager.connection_to_room_info.get(connection_id) {
                None => Err(AppError::ConnectionNotFound {
                    connection_id: connection_id.to_string(),
                }),
                Some(room_info) => Ok(ServerResponse::ChatMessage {
                    player_name: room_info.clone().player_name,
                    message: message,
                }),
            }
        }
        ClientMessage::CreateRoom {
            room_name,
            first_player_name,
        } => {
            let (room_id, player_id) = room_manager.create_room(
                room_name,
                connection_id.to_string(),
                first_player_name,
            )?;
            Ok(ServerResponse::FirstPlayerRoomCreated { room_id, player_id })
        }

        ClientMessage::DestroyRoom { room_id } => {
            room_manager.destroy_room(&room_id, &connection_id)?;
            Ok(ServerResponse::RoomDestroyed)
        }
        ClientMessage::JoinRoom {
            player_name,
            room_id,
        } => {
            let player_id =
                room_manager.join_room(&room_id, connection_id.to_string(), player_name.clone())?;
            Ok(ServerResponse::PlayerJoined {
                player_id,
                player_name,
            })
        }
        ClientMessage::LeaveRoom => {
            let player_name = room_manager.leave_room(&connection_id)?;
            Ok(ServerResponse::PlayerLeft { player_name })
        }
        ClientMessage::PlayerReady => {
            let player_id = room_manager.get_player_id_from_connection_id(connection_id)?;
            let ready_result = room_manager.ready_player(&player_id)?;
            let room_id = room_manager.get_player_room_from_player_id(&player_id)?;
            Ok(if ready_result.game_started {
                ServerResponse::GameStarted {
                    room_id: room_id.to_string(),
                    turn_order: ready_result.turn_order.unwrap(),
                }
            } else {
                ServerResponse::PlayersReady {
                    players_ready: ready_result.players_ready,
                }
            })
        }
        ClientMessage::TurnPass => {
            let next_player_id = room_manager.pass_turn(connection_id)?;
            Ok(ServerResponse::TurnChange {
                next_player_id: next_player_id,
            })
        }
    }
}
