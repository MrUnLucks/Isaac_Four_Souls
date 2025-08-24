use std::collections::HashMap;

use dashmap::DashMap;
use tokio::sync::mpsc::{self, UnboundedSender};

use crate::game::game_loop::{GameEvent, GameLoop};
use crate::{AppError, AppResult, ConnectionCommand, TurnOrder};

pub struct GameLoopRegistry {
    // DashMap is lock-free - no Arc<Mutex<>> needed!
    game_loops: DashMap<String, (mpsc::Sender<GameEvent>, tokio::task::JoinHandle<()>)>, // room_id -> game event sender
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

        for (player_id, conn_id) in players_id_to_connection_id.clone() {
            self.connection_ids_to_room_info
                .insert(conn_id, (room_id.to_string(), player_id));
        }
        let turn_order = TurnOrder::new(players_id_to_connection_id.keys().cloned().collect());

        let mut game_loop = GameLoop::new(players_id_to_connection_id, turn_order.clone());

        let task_handle = tokio::spawn(async move {
            game_loop.run(inbound_receiver, cmd_sender).await;
        });

        self.game_loops
            .insert(room_id.to_string(), (inbound_sender, task_handle));

        Ok(turn_order)
    }

    pub fn send_game_event_to_room(&self, room_id: &str, event: GameEvent) -> AppResult<()> {
        if let Some(entry) = self.game_loops.get(room_id) {
            let (sender, _) = entry.value();
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
            .map(|entry| entry.value().clone())
            .ok_or(AppError::ConnectionNotInRoom)?;

        if let Some(entry) = self.game_loops.get(&room_id) {
            let (sender, _) = entry.value();
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
            .map(|entry| entry.value().clone())
            .ok_or(AppError::ConnectionNotInRoom)
    }

    pub fn cleanup_game_loop(&self, room_id: &str) {
        if let Some((_, (sender, task_handle))) = self.game_loops.remove(room_id) {
            println!("🛑 Stopping game loop for room {}", room_id);

            drop(sender);

            tokio::spawn(async move {
                tokio::select! {
                    _ = task_handle => {
                        println!("✅ Game loop ended gracefully");
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {
                        println!("⚠️ Game loop took too long to stop");
                        // Task will be dropped and aborted here
                    }
                }
            });
        }

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
