use crate::player::manager::{Player, PlayerManager};
use rand::seq::SliceRandom;

pub struct PlayerOrder {
    order: Vec<String>,
    active_player_id: String,
}

pub enum TurnPhases {
    Waiting,
    TurnStart,
    TurnAction,
    TurnEnd,
    GameOver,
}

//TODO: reimplement with room system!
// impl PlayerOrder {
//     pub fn new(manager: PlayerManager) -> Self {
//         let mut rng = rand::rng();
//         let mut player_vec: Vec<&Player> = manager.list_connected_players();
//         println!("{:?}", player_vec);
//         player_vec.shuffle(&mut rng);
//         println!("{:?}", player_vec);
//         let order: Vec<String> = player_vec.iter().map(|player| player.id.clone()).collect();
//         let active_player_id = order[0].clone();
//         Self {
//             order,
//             active_player_id,
//         }
//     }
//     pub fn can_player_act(&self, player_id: String) -> bool {
//         self.active_player_id == player_id
//     }
// }
