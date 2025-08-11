use std::ops::{Deref, DerefMut};

use crate::{game::turn_order::TurnOrder, Room, RoomError};

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

    pub fn start_game(&mut self) -> Result<TurnOrder, RoomError> {
        if self.can_start_game() {
            self.room.set_state_in_game();
            self.turn_order = TurnOrder::new(self.room.get_players_id());
            Ok(self.turn_order.clone())
        } else {
            Err(RoomError::PlayersNotReady)
        }
    }

    pub fn pass_turn(&mut self, player_id: &str) -> Result<String, RoomError> {
        if self.turn_order.is_player_turn(player_id) {
            let next_player_id = self.turn_order.advance_turn();
            Ok(next_player_id)
        } else {
            Err(RoomError::NotPlayerTurn)
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
