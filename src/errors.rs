use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
pub enum AppError {
    #[error("Player '{player_name}' is already in a room")]
    PlayerAlreadyInRoom { player_name: String },

    #[error("Connection is not in any room")]
    ConnectionNotInRoom,

    // Room-related errors
    #[error("Room '{room_id}' not found")]
    RoomNotFound { room_id: String },

    #[error("Room '{room_id}' is full (max: {max_players})")]
    RoomFull { room_id: String, max_players: usize },

    #[error("Room '{room_id}' is already in game")]
    RoomInGame { room_id: String },

    #[error("Room name cannot be empty")]
    RoomNameEmpty,

    #[error("Cannot start game - only {ready_count}/{total_count} players ready")]
    PlayersNotReady {
        ready_count: usize,
        total_count: usize,
    },

    // Connection-related errors
    #[error("Connection '{connection_id}' not found")]
    ConnectionNotFound { connection_id: String },

    #[error("Failed to send message to connection '{connection_id}'")]
    MessageSendFailed { connection_id: String },

    // Game-related errors
    #[error("Game loop for room '{room_id}' not found")]
    GameMessageLoopNotFound { room_id: String },

    #[error("Failed to send event to game loop: {reason}")]
    GameEventSendFailed { reason: String },

    #[error("Turn order not initialized")]
    TurnOrderNotInitialized,

    // Validation errors
    #[error("Invalid player name: {reason}")]
    InvalidPlayerName { reason: String },

    #[error("Invalid room name: {reason}")]
    InvalidRoomName { reason: String },

    // Serialization errors
    #[error("Failed to serialize response: {message}")]
    SerializationError { message: String },

    #[error("WebSocket error: {message}")]
    WebSocketError { message: String },

    #[error("Unknown message: {message}")]
    UnknownMessage { message: String },

    #[error("Game ended unexpectedly")]
    GameEndedUnexpectedly,

    #[error("Player not found")]
    PlayerNotFound,

    #[error("Empty loot deck with reshuffle")]
    EmptyLootDeck,

    #[error("Invalid card: not in player's hand")]
    CardNotInHand,

    #[error("Invalid Priority pass")]
    InvalidPriorityPass,

    #[error("Invalid Turn Pass")]
    InvalidTurnPass,

    #[error("Game ended")]
    GameEnded,

    #[error("Not player's turn")]
    NotPlayerTurn,

    #[error("Internal server error: {message}")]
    Internal { message: String },
}

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Copy)]
pub enum ErrorCategory {
    ClientError,
    ServerError,
    ValidationError,
    GameError,
}

impl AppError {
    pub fn category(&self) -> ErrorCategory {
        match self {
            AppError::RoomNotFound { .. }
            | AppError::PlayerAlreadyInRoom { .. }
            | AppError::RoomFull { .. }
            | AppError::RoomInGame { .. }
            | AppError::ConnectionNotInRoom { .. }
            | AppError::TurnOrderNotInitialized
            | AppError::UnknownMessage { .. } => ErrorCategory::ClientError,

            AppError::InvalidPlayerName { .. }
            | AppError::InvalidRoomName { .. }
            | AppError::RoomNameEmpty => ErrorCategory::ValidationError,

            AppError::ConnectionNotFound { .. }
            | AppError::MessageSendFailed { .. }
            | AppError::GameMessageLoopNotFound { .. }
            | AppError::GameEventSendFailed { .. }
            | AppError::SerializationError { .. }
            | AppError::WebSocketError { .. }
            | AppError::Internal { .. }
            | AppError::GameEndedUnexpectedly { .. } => ErrorCategory::ServerError,

            AppError::PlayersNotReady { .. }
            | AppError::NotPlayerTurn
            | AppError::PlayerNotFound
            | AppError::EmptyLootDeck
            | AppError::CardNotInHand
            | AppError::InvalidPriorityPass
            | AppError::InvalidTurnPass
            | AppError::GameEnded => ErrorCategory::GameError,
        }
    }

    pub fn should_log(&self) -> bool {
        matches!(self.category(), ErrorCategory::ServerError)
    }

    pub fn status_code(&self) -> u16 {
        match self.category() {
            ErrorCategory::GameError => 200,
            ErrorCategory::ClientError => 400,
            ErrorCategory::ValidationError => 422,
            ErrorCategory::ServerError => 500,
        }
    }

    pub fn variant_name(&self) -> &'static str {
        match self {
            AppError::PlayerAlreadyInRoom { .. } => "PlayerAlreadyInRoom",
            AppError::ConnectionNotInRoom => "ConnectionNotInRoom",
            AppError::RoomNotFound { .. } => "RoomNotFound",
            AppError::RoomFull { .. } => "RoomFull",
            AppError::RoomInGame { .. } => "RoomInGame",
            AppError::RoomNameEmpty => "RoomNameEmpty",
            AppError::PlayersNotReady { .. } => "PlayersNotReady",
            AppError::ConnectionNotFound { .. } => "ConnectionNotFound",
            AppError::MessageSendFailed { .. } => "MessageSendFailed",
            AppError::GameMessageLoopNotFound { .. } => "GameMessageLoopNotFound",
            AppError::GameEventSendFailed { .. } => "GameEventSendFailed",
            AppError::TurnOrderNotInitialized => "TurnOrderNotInitialized",
            AppError::InvalidPlayerName { .. } => "InvalidPlayerName",
            AppError::InvalidRoomName { .. } => "InvalidRoomName",
            AppError::SerializationError { .. } => "SerializationError",
            AppError::NotPlayerTurn => "NotPlayerTurn",
            AppError::GameEndedUnexpectedly => "GameEndedUnexpectedly",
            AppError::WebSocketError { .. } => "WebSocketError",
            AppError::UnknownMessage { .. } => "UnknownMessage",
            AppError::Internal { .. } => "Internal",
            AppError::CardNotInHand { .. } => "CardNotInHand",
            AppError::PlayerNotFound { .. } => "PlayerNotFound",
            AppError::EmptyLootDeck { .. } => "EmptyLootDeck",
            AppError::InvalidPriorityPass { .. } => "InvalidPriorityPass",
            AppError::InvalidTurnPass { .. } => "InvalidTurnPass",
            AppError::GameEnded { .. } => "GameEnded",
        }
    }

    pub fn user_friendly_message(&self) -> String {
        match self {
            AppError::RoomFull { max_players, .. } => {
                format!("Room is full (maximum {} players)", max_players)
            }
            AppError::RoomNotFound { .. } => {
                "The room you're looking for doesn't exist".to_string()
            }
            AppError::ConnectionNotInRoom => "You need to join a room first".to_string(),
            AppError::SerializationError { .. } => "Invalid message format".to_string(),
            _ => self.to_string(), // Use the error's display message
        }
    }
}

pub mod validation {
    use super::AppError;

    pub fn validate_player_name(name: &str) -> Result<(), AppError> {
        if name.trim().is_empty() {
            return Err(AppError::InvalidPlayerName {
                reason: "Player name cannot be empty".to_string(),
            });
        }
        if name.len() > 50 {
            return Err(AppError::InvalidPlayerName {
                reason: "Player name cannot exceed 50 characters".to_string(),
            });
        }
        if name
            .chars()
            .any(|c| !c.is_alphanumeric() && c != '_' && c != '-')
        {
            return Err(AppError::InvalidPlayerName {
                reason: "Player name can only contain letters, numbers, underscore, and dash"
                    .to_string(),
            });
        }
        Ok(())
    }

    pub fn validate_room_name(name: &str) -> Result<(), AppError> {
        if name.trim().is_empty() {
            return Err(AppError::RoomNameEmpty);
        }
        if name.len() > 100 {
            return Err(AppError::InvalidRoomName {
                reason: "Room name cannot exceed 100 characters".to_string(),
            });
        }
        Ok(())
    }
}
