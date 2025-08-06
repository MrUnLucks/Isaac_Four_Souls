pub mod game;
pub mod network;

// Re-export commonly used items for convenience
pub use network::room::{Room, RoomError};
pub use network::room_manager::{RoomManager, RoomManagerError};
