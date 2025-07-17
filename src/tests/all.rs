//! Unit tests for Isaac Four Souls game modules
//! Run with: cargo test

use serde_json::{from_str, to_string};
use std::collections::{HashMap, HashSet};

// Import the modules we're testing
use crate::game::room::{Room, RoomInfo, RoomState};
use crate::game::room_manager::{PlayerRoomInfo, ReadyPlayerResult, RoomManager};
use crate::network::connection_manager::ConnectionManager;
use crate::network::messages::{
    deserialize_message, handle_message, serialize_response, ServerError, ServerMessage,
    ServerResponse,
};

// Mock imports for testing
use futures_util::stream::SplitSink;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

#[cfg(test)]
mod room_tests {
    use super::*;

    #[test]
    fn test_room_creation() {
        let room = Room::new("Test Room".to_string());

        assert_eq!(room.name, "Test Room");
        assert_eq!(room.players.len(), 0);
        assert_eq!(room.max_players, 4);
        assert_eq!(room.min_players, 2);
        assert!(matches!(room.state, RoomState::Lobby));
        assert!(room.players_ready.is_empty());
        assert!(!room.id.is_empty());
    }

    #[test]
    fn test_add_player_success() {
        let mut room = Room::new("Test Room".to_string());

        let player_id = room.add_player("Alice".to_string()).unwrap();

        assert_eq!(room.players.len(), 1);
        assert_eq!(room.players.get(&player_id), Some(&"Alice".to_string()));
        assert!(!player_id.is_empty());
    }

    #[test]
    fn test_add_player_room_full() {
        let mut room = Room::new("Test Room".to_string());

        // Fill the room to capacity
        for i in 0..4 {
            room.add_player(format!("Player{}", i)).unwrap();
        }

        // Try to add one more player
        let result = room.add_player("Extra Player".to_string());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Room is full");
    }

    #[test]
    fn test_remove_player_success() {
        let mut room = Room::new("Test Room".to_string());
        let player_id = room.add_player("Alice".to_string()).unwrap();

        let removed_name = room.remove_player(&player_id).unwrap();

        assert_eq!(removed_name, "Alice");
        assert_eq!(room.players.len(), 0);
        assert!(!room.players_ready.contains(&player_id));
    }

    #[test]
    fn test_remove_player_not_found() {
        let mut room = Room::new("Test Room".to_string());

        let result = room.remove_player("nonexistent_id");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cannot find player to remove");
    }

    #[test]
    fn test_add_player_ready() {
        let mut room = Room::new("Test Room".to_string());
        let player_id = room.add_player("Alice".to_string()).unwrap();

        let ready_set = room.add_player_ready(&player_id).unwrap();

        assert!(ready_set.contains(&player_id));
        assert!(room.players_ready.contains(&player_id));
    }

    #[test]
    fn test_add_player_ready_not_in_room() {
        let mut room = Room::new("Test Room".to_string());

        let result = room.add_player_ready("nonexistent_id");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Player not in the room");
    }

    #[test]
    fn test_add_player_ready_already_ready() {
        let mut room = Room::new("Test Room".to_string());
        let player_id = room.add_player("Alice".to_string()).unwrap();
        room.add_player_ready(&player_id).unwrap();

        let result = room.add_player_ready(&player_id);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Player already ready");
    }

    #[test]
    fn test_can_start_game() {
        let mut room = Room::new("Test Room".to_string());
        let player1_id = room.add_player("Alice".to_string()).unwrap();
        let player2_id = room.add_player("Bob".to_string()).unwrap();

        // Game cannot start if not all players are ready
        assert!(!room.can_start_game());

        room.add_player_ready(&player1_id).unwrap();
        assert!(!room.can_start_game());

        room.add_player_ready(&player2_id).unwrap();
        assert!(room.can_start_game());
    }

    #[test]
    fn test_start_game_success() {
        let mut room = Room::new("Test Room".to_string());
        let player1_id = room.add_player("Alice".to_string()).unwrap();
        let player2_id = room.add_player("Bob".to_string()).unwrap();
        room.add_player_ready(&player1_id).unwrap();
        room.add_player_ready(&player2_id).unwrap();

        let result = room.start_game();

        assert!(result.is_ok());
        assert!(matches!(room.state, RoomState::InGame));
        assert!(room.players_ready.is_empty());
    }

    #[test]
    fn test_start_game_cannot_start() {
        let mut room = Room::new("Test Room".to_string());
        room.add_player("Alice".to_string()).unwrap();

        let result = room.start_game();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cannot start game");
    }

    #[test]
    fn test_get_room_info() {
        let mut room = Room::new("Test Room".to_string());
        room.add_player("Alice".to_string()).unwrap();

        let info = room.get_room_info();

        assert_eq!(info.name, "Test Room");
        assert_eq!(info.player_count, 1);
        assert_eq!(info.max_players, 4);
        assert!(matches!(info.state, RoomState::Lobby));
    }

    #[test]
    fn test_get_players_id() {
        let mut room = Room::new("Test Room".to_string());
        let player1_id = room.add_player("Alice".to_string()).unwrap();
        let player2_id = room.add_player("Bob".to_string()).unwrap();

        let player_ids = room.get_players_id();

        assert_eq!(player_ids.len(), 2);
        assert!(player_ids.contains(&player1_id));
        assert!(player_ids.contains(&player2_id));
    }
}

#[cfg(test)]
mod room_manager_tests {
    use super::*;

    #[test]
    fn test_room_manager_creation() {
        let manager = RoomManager::new();

        assert_eq!(manager.connection_to_room_info.len(), 0);
    }

    #[test]
    fn test_create_room_success() {
        let mut manager = RoomManager::new();

        let room_id = manager
            .create_room(
                "Test Room".to_string(),
                "conn1".to_string(),
                "Alice".to_string(),
            )
            .unwrap();

        assert!(!room_id.is_empty());
        assert!(manager.connection_to_room_info.contains_key("conn1"));

        let player_info = manager.connection_to_room_info.get("conn1").unwrap();
        assert_eq!(player_info.room_id, room_id);
    }

    #[test]
    fn test_create_room_empty_name() {
        let mut manager = RoomManager::new();

        let result = manager.create_room("".to_string(), "conn1".to_string(), "Alice".to_string());

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Room name cannot be empty");
    }

    #[test]
    fn test_create_room_player_already_in_room() {
        let mut manager = RoomManager::new();

        manager
            .create_room(
                "Room 1".to_string(),
                "conn1".to_string(),
                "Alice".to_string(),
            )
            .unwrap();

        let result = manager.create_room(
            "Room 2".to_string(),
            "conn1".to_string(),
            "Alice".to_string(),
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Player already in a room");
    }

    #[test]
    fn test_join_room_success() {
        let mut manager = RoomManager::new();

        let room_id = manager
            .create_room(
                "Test Room".to_string(),
                "conn1".to_string(),
                "Alice".to_string(),
            )
            .unwrap();

        let result = manager.join_room(&room_id, "conn2".to_string(), "Bob".to_string());

        assert!(result.is_ok());
        assert!(manager.connection_to_room_info.contains_key("conn2"));

        let player_info = manager.connection_to_room_info.get("conn2").unwrap();
        assert_eq!(player_info.room_id, room_id);
    }

    #[test]
    fn test_join_room_not_found() {
        let mut manager = RoomManager::new();

        let result = manager.join_room("nonexistent", "conn1".to_string(), "Alice".to_string());

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Room not found");
    }

    #[test]
    fn test_join_room_player_already_in_room() {
        let mut manager = RoomManager::new();

        let room_id = manager
            .create_room(
                "Test Room".to_string(),
                "conn1".to_string(),
                "Alice".to_string(),
            )
            .unwrap();

        let result = manager.join_room(&room_id, "conn1".to_string(), "Bob".to_string());

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Player already in a room");
    }

    #[test]
    fn test_leave_room_success() {
        let mut manager = RoomManager::new();

        let room_id = manager
            .create_room(
                "Test Room".to_string(),
                "conn1".to_string(),
                "Alice".to_string(),
            )
            .unwrap();

        let removed_name = manager.leave_room("conn1").unwrap();

        assert_eq!(removed_name, "Alice");
        assert!(!manager.connection_to_room_info.contains_key("conn1"));
    }

    #[test]
    fn test_leave_room_not_in_room() {
        let mut manager = RoomManager::new();

        let result = manager.leave_room("nonexistent");

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Player not in any room");
    }

    #[test]
    fn test_leave_room_cleanup_empty_room() {
        let mut manager = RoomManager::new();

        let room_id = manager
            .create_room(
                "Test Room".to_string(),
                "conn1".to_string(),
                "Alice".to_string(),
            )
            .unwrap();

        manager.leave_room("conn1").unwrap();

        // Room should be removed when empty
        assert!(manager.get_room_mut(&room_id).is_none());
    }

    #[test]
    fn test_ready_player_success() {
        let mut manager = RoomManager::new();

        let room_id = manager
            .create_room(
                "Test Room".to_string(),
                "conn1".to_string(),
                "Alice".to_string(),
            )
            .unwrap();

        manager
            .join_room(&room_id, "conn2".to_string(), "Bob".to_string())
            .unwrap();

        let player1_info = manager.connection_to_room_info.get("conn1").unwrap();
        let player1_id = &player1_info.room_player_id;

        let result = manager.ready_player(player1_id).unwrap();

        assert!(result.players_ready.contains(player1_id));
        assert!(!result.game_started);
    }

    #[test]
    fn test_ready_player_game_starts() {
        let mut manager = RoomManager::new();

        let room_id = manager
            .create_room(
                "Test Room".to_string(),
                "conn1".to_string(),
                "Alice".to_string(),
            )
            .unwrap();

        manager
            .join_room(&room_id, "conn2".to_string(), "Bob".to_string())
            .unwrap();

        let player1_info = manager.connection_to_room_info.get("conn1").unwrap();
        let player1_id = &player1_info.room_player_id;
        let player2_info = manager.connection_to_room_info.get("conn2").unwrap();
        let player2_id = &player2_info.room_player_id;

        manager.ready_player(player1_id).unwrap();
        let result = manager.ready_player(player2_id).unwrap();

        assert!(result.game_started);
    }

    #[test]
    fn test_get_player_room_from_connection_id() {
        let mut manager = RoomManager::new();

        let room_id = manager
            .create_room(
                "Test Room".to_string(),
                "conn1".to_string(),
                "Alice".to_string(),
            )
            .unwrap();

        let found_room_id = manager.get_player_room_from_connection_id("conn1").unwrap();
        assert_eq!(found_room_id, room_id);

        let not_found = manager.get_player_room_from_connection_id("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_player_list() {
        let mut manager = RoomManager::new();

        let room_id = manager
            .create_room(
                "Test Room".to_string(),
                "conn1".to_string(),
                "Alice".to_string(),
            )
            .unwrap();

        manager
            .join_room(&room_id, "conn2".to_string(), "Bob".to_string())
            .unwrap();

        let player_list = manager.get_player_list(&room_id).unwrap();
        assert_eq!(player_list.len(), 2);

        let empty_list = manager.get_player_list("nonexistent");
        assert!(empty_list.is_none());
    }
}

#[cfg(test)]
mod messages_tests {
    use super::*;

    #[test]
    fn test_deserialize_ping_message() {
        let json = r#"{"Ping": null}"#;
        let message = deserialize_message(json).unwrap();
        assert!(matches!(message, ServerMessage::Ping));
    }

    #[test]
    fn test_deserialize_chat_message() {
        let json = r#"{"Chat": {"message": "Hello world"}}"#;
        let message = deserialize_message(json).unwrap();

        if let ServerMessage::Chat { message } = message {
            assert_eq!(message, "Hello world");
        } else {
            panic!("Expected Chat message");
        }
    }

    #[test]
    fn test_deserialize_create_room_message() {
        let json = r#"{"CreateRoom": {"room_name": "Test Room", "first_player_name": "Alice"}}"#;
        let message = deserialize_message(json).unwrap();

        if let ServerMessage::CreateRoom {
            room_name,
            first_player_name,
        } = message
        {
            assert_eq!(room_name, "Test Room");
            assert_eq!(first_player_name, "Alice");
        } else {
            panic!("Expected CreateRoom message");
        }
    }

    #[test]
    fn test_deserialize_invalid_json() {
        let json = r#"{"InvalidMessage": "test"}"#;
        let result = deserialize_message(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_pong_response() {
        let response = ServerResponse::Pong;
        let json = serialize_response(&response).unwrap();
        assert_eq!(json, r#""Pong""#);
    }

    #[test]
    fn test_serialize_chat_response() {
        let response = ServerResponse::ChatMessage {
            player_name: "Alice".to_string(),
            message: "Hello".to_string(),
        };
        let json = serialize_response(&response).unwrap();
        assert!(json.contains("ChatMessage"));
        assert!(json.contains("Alice"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_serialize_error_response() {
        let response = ServerResponse::Error {
            message: ServerError::PlayerNotFound,
        };
        let json = serialize_response(&response).unwrap();
        assert!(json.contains("Error"));
        assert!(json.contains("PlayerNotFound"));
    }

    #[test]
    fn test_handle_ping_message() {
        let mut room_manager = RoomManager::new();
        let message = ServerMessage::Ping;

        let response = handle_message(message, &mut room_manager, "conn1");

        assert!(matches!(response, ServerResponse::Pong));
    }

    #[test]
    fn test_handle_create_room_message() {
        let mut room_manager = RoomManager::new();
        let message = ServerMessage::CreateRoom {
            room_name: "Test Room".to_string(),
            first_player_name: "Alice".to_string(),
        };

        let response = handle_message(message, &mut room_manager, "conn1");

        if let ServerResponse::RoomCreated { room_id } = response {
            assert!(!room_id.is_empty());
        } else {
            panic!("Expected RoomCreated response");
        }
    }

    #[test]
    fn test_handle_chat_message_no_room() {
        let mut room_manager = RoomManager::new();
        let message = ServerMessage::Chat {
            message: "Hello".to_string(),
        };

        let response = handle_message(message, &mut room_manager, "conn1");

        if let ServerResponse::Error { message } = response {
            assert!(matches!(message, ServerError::PlayerNotFound));
        } else {
            panic!("Expected Error response");
        }
    }

    #[test]
    fn test_handle_chat_message_success() {
        let mut room_manager = RoomManager::new();

        // Create room first
        room_manager
            .create_room(
                "Test Room".to_string(),
                "conn1".to_string(),
                "Alice".to_string(),
            )
            .unwrap();

        let message = ServerMessage::Chat {
            message: "Hello".to_string(),
        };

        let response = handle_message(message, &mut room_manager, "conn1");

        if let ServerResponse::ChatMessage {
            player_name,
            message,
        } = response
        {
            assert_eq!(player_name, "Alice");
            assert_eq!(message, "Hello");
        } else {
            panic!("Expected ChatMessage response");
        }
    }

    #[test]
    fn test_handle_join_room_message() {
        let mut room_manager = RoomManager::new();

        // Create room first
        let room_id = room_manager
            .create_room(
                "Test Room".to_string(),
                "conn1".to_string(),
                "Alice".to_string(),
            )
            .unwrap();

        let message = ServerMessage::JoinRoom {
            connection_id: "conn2".to_string(),
            player_name: "Bob".to_string(),
            room_id,
        };

        let response = handle_message(message, &mut room_manager, "conn2");

        if let ServerResponse::PlayerJoined { player_name } = response {
            assert_eq!(player_name, "Bob");
        } else {
            panic!("Expected PlayerJoined response");
        }
    }

    #[test]
    fn test_handle_leave_room_message() {
        let mut room_manager = RoomManager::new();

        // Create room first
        room_manager
            .create_room(
                "Test Room".to_string(),
                "conn1".to_string(),
                "Alice".to_string(),
            )
            .unwrap();

        let message = ServerMessage::LeaveRoom {
            connection_id: "conn1".to_string(),
        };

        let response = handle_message(message, &mut room_manager, "conn1");

        if let ServerResponse::PlayerLeft { player_name } = response {
            assert_eq!(player_name, "Alice");
        } else {
            panic!("Expected PlayerLeft response");
        }
    }

    #[test]
    fn test_handle_player_ready_message() {
        let mut room_manager = RoomManager::new();

        // Create room and add players
        let room_id = room_manager
            .create_room(
                "Test Room".to_string(),
                "conn1".to_string(),
                "Alice".to_string(),
            )
            .unwrap();

        room_manager
            .join_room(&room_id, "conn2".to_string(), "Bob".to_string())
            .unwrap();

        let player1_info = room_manager.connection_to_room_info.get("conn1").unwrap();
        let player1_id = player1_info.room_player_id.clone();

        let message = ServerMessage::PlayerReady {
            player_id: player1_id.clone(),
        };

        let response = handle_message(message, &mut room_manager, "conn1");

        if let ServerResponse::PlayersReady { players_ready } = response {
            assert!(players_ready.contains(&player1_id));
        } else {
            panic!("Expected PlayersReady response");
        }
    }
}

// Helper function to run all tests
#[cfg(test)]
mod test_runner {
    use super::*;

    #[tokio::test]
    async fn run_all_tests() {
        // This is more of a documentation of what tests exist
        // Individual tests are run with `cargo test`
        println!("All unit tests should be run with: cargo test");

        // You can also run specific test modules:
        // cargo test room_tests
        // cargo test room_manager_tests
        // cargo test messages_tests
    }
}
