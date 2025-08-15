pub mod errors;
pub mod game;
pub mod network;

// Re-export commonly used items for convenience
pub use errors::{AppError, AppResult};
pub use game::turn_order::TurnOrder;
pub use network::connection_commands::{CommandProcessor, ConnectionCommand};
pub use network::connection_handler::ConnectionHandler;
pub use network::connection_manager::ConnectionManager;
pub use network::game_loop_registry::GameLoopRegistry;
pub use network::lobby::LobbyState;
pub use network::message_router::handle_text_message;
pub use network::room::Room;
pub use network::room_manager::RoomManager;
pub use network::server::WebsocketServer;
