use std::collections::HashMap;

use dashmap::DashMap;
use tokio::sync::mpsc::{self, UnboundedSender};

use crate::game::game_loop::{GameEvent, GameLoop};
use crate::{AppError, AppResult, ConnectionCommand, TurnOrder};

pub struct GameLoopRegistry {
    // DashMap is lock-free - no Arc<Mutex<>> needed!
    game_loops: DashMap<String, mpsc::Sender<GameEvent>>, // room_id -> game event sender
    connection_ids_to_room_info: DashMap<String, (String, String)>, // conn_id -> (room_id, player_id)
}

impl GameLoopRegistry {
    pub fn new() -> Self {
        Self {
            game_loops: DashMap::new(),
            connection_ids_to_room_info: DashMap::new(),
        }
    }

    pub fn start_game_loop(
        &self,
        room_id: &str,
        players_id_to_connection_id: HashMap<String, String>,
        cmd_sender: UnboundedSender<ConnectionCommand>,
    ) -> AppResult<TurnOrder> {
        let (inbound_sender, inbound_receiver) = mpsc::channel(32);

        self.game_loops.insert(room_id.to_string(), inbound_sender);
        for (player_id, conn_id) in players_id_to_connection_id.clone() {
            self.connection_ids_to_room_info
                .insert(conn_id, (room_id.to_string(), player_id));
        }
        let turn_order = TurnOrder::new(players_id_to_connection_id.keys().cloned().collect());

        let mut game_loop = GameLoop::new(players_id_to_connection_id, turn_order.clone());

        tokio::spawn(async move {
            game_loop.run(inbound_receiver, cmd_sender).await;
        });

        Ok(turn_order)
    }

    pub fn send_game_event_to_room(&self, room_id: &str, event: GameEvent) -> AppResult<()> {
        if let Some(sender) = self.game_loops.get(room_id) {
            sender
                .try_send(event)
                .map_err(|err| AppError::GameEventSendFailed {
                    reason: err.to_string(),
                })?;
            Ok(())
        } else {
            Err(AppError::GameLoopNotFound {
                room_id: room_id.to_string(),
            })
        }
    }

    pub fn send_game_event_to_room_by_connection_id(
        &self,
        connection_id: &str,
        event: GameEvent,
    ) -> AppResult<()> {
        let (room_id, _) = self
            .connection_ids_to_room_info
            .get(connection_id)
            .map(|entry| entry.value().clone()) // Clone the value from the DashMap entry
            .ok_or(AppError::ConnectionNotInRoom)?;

        if let Some(sender) = self.game_loops.get(&room_id) {
            sender
                .try_send(event)
                .map_err(|err| AppError::GameEventSendFailed {
                    reason: err.to_string(),
                })?;
            Ok(())
        } else {
            Err(AppError::GameLoopNotFound { room_id })
        }
    }

    pub fn get_player_info_from_connection_id(
        &self,
        connection_id: &str,
    ) -> AppResult<(String, String)> {
        self.connection_ids_to_room_info
            .get(connection_id)
            .map(|entry| entry.value().clone()) // Clone the tuple from DashMap entry
            .ok_or(AppError::ConnectionNotInRoom)
    }

    pub fn cleanup_game_loop(&self, room_id: &str) {
        self.game_loops.remove(room_id);

        self.connection_ids_to_room_info
            .retain(|_, (game_room_id, _)| game_room_id != room_id);
    }

    pub fn has_game_loop(&self, room_id: &str) -> bool {
        self.game_loops.contains_key(room_id)
    }

    pub fn remove_player(&self, connection_id: &str) -> Option<(String, String)> {
        self.connection_ids_to_room_info
            .remove(connection_id)
            .map(|(_, value)| value)
    }

    pub fn get_game_players(&self, room_id: &str) -> Vec<String> {
        self.connection_ids_to_room_info
            .iter()
            .filter_map(|entry| {
                let (conn_id, (game_room_id, _)) = entry.pair();
                if game_room_id == room_id {
                    Some(conn_id.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}
