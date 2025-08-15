use std::ops::{Deref, DerefMut};

use crate::{AppError, AppResult, Room, TurnOrder};

pub struct RoomActor {
    room: Room,
    turn_order: TurnOrder,
}

impl RoomActor {
    pub fn new(room_name: &str) -> Self {
        Self {
            room: Room::new(room_name.to_string()),
            //Initialize empty on creation room to skip type checks w/o inserting Option<TurnOrder>
            turn_order: TurnOrder::new(vec!["Undefined".to_string()]),
        }
    }

    pub fn start_game(&mut self) -> AppResult<TurnOrder> {
        if self.can_start_game() {
            self.room.set_state_in_game();
            self.turn_order = TurnOrder::new(self.room.get_players_id());
            Ok(self.turn_order.clone())
        } else {
            Err(AppError::PlayersNotReady {
                ready_count: self.player_ready_count(),
                total_count: self.player_count(),
            })
        }
    }

    pub fn pass_turn(&mut self, player_id: &str) -> AppResult<String> {
        if self.turn_order.is_player_turn(player_id) {
            let next_player_id = self.turn_order.advance_turn();
            Ok(next_player_id)
        } else {
            Err(AppError::NotPlayerTurn {
                player_id: self.turn_order.active_player_id.clone(),
            })
        }
    }
}

// Get Room methods on RoomActor
impl Deref for RoomActor {
    type Target = Room;

    fn deref(&self) -> &Self::Target {
        &self.room
    }
}

impl DerefMut for RoomActor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.room
    }
}
