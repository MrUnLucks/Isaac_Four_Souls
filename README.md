# Isaac Four Souls - Multiplayer Game Server

A Rust-based WebSocket server for playing Isaac Four Souls, the official card game based on The Binding of Isaac. This server handles multiplayer game sessions with real-time communication between players.

## 🎮 Features

- **Real-time Multiplayer**: WebSocket-based communication for instant game updates
- **Room System**: Create and join game rooms with up to 4 players
- **Player Management**: Track player resources (health, coins, souls)
- **Game State Management**: Handle lobby, game start, and turn phases
- **Chat System**: In-game messaging between players
- **Ready System**: Players can ready up to start games

## 🏗️ Architecture

### Core Components

- **Room System** (`src/game/room.rs`): Manages individual game rooms and player sessions
- **Room Manager** (`src/game/room_manager.rs`): Coordinates multiple rooms and player connections
- **Player Resources** (`src/game/resources.rs`): Tracks player health, coins, and souls
- **WebSocket Server** (`src/network/websocket_server.rs`): Handles client connections and message routing
- **Message System** (`src/network/messages.rs`): Serializes/deserializes game messages
- **Connection Manager** (`src/network/connection_manager.rs`): Manages WebSocket connections

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+
- Cargo

### Installation

1. Clone the repository:

```bash
git clone <repository-url>
cd isaac-four-souls
```

2. Build the project:

```bash
cargo build --release
```

3. Run the server:

```bash
cargo run
```

The server will start on `127.0.0.1:8080` by default.

## 📡 WebSocket API

### Message Types

#### Client → Server Messages

```rust
// Ping the server
{"Ping": null}

// Create a new room
{
  "CreateRoom": {
    "room_name": "My Game Room",
    "first_player_name": "Player1"
  }
}

// Join an existing room
{
  "JoinRoom": {
    "connection_id": "conn_123",
    "player_name": "Player2",
    "room_id": "room_456"
  }
}

// Send a chat message
{
  "Chat": {
    "message": "Hello everyone!"
  }
}

// Mark player as ready
{
  "PlayerReady": {
    "player_id": "player_789"
  }
}

// Leave current room
{
  "LeaveRoom": {
    "connection_id": "conn_123"
  }
}
```

#### Server → Client Responses

```rust
// Pong response
"Pong"

// Room created successfully
{
  "RoomCreated": {
    "room_id": "room_456"
  }
}

// Player joined room
{
  "PlayerJoined": {
    "player_name": "Player2"
  }
}

// Chat message broadcast
{
  "ChatMessage": {
    "player_name": "Player1",
    "message": "Hello everyone!"
  }
}

// Players ready status
{
  "PlayersReady": {
    "players_ready": ["player_1", "player_2"]
  }
}

// Game started
"GameStarted"

// Error response
{
  "Error": {
    "message": "PlayerNotFound" // or "RoomNotFound", "UnknownResponse"
  }
}
```

## 🎯 Game Rules Implementation

### Player Resources

Each player starts with:

- **Health**: 2 HP (Isaac's default)
- **Coins**: 3 coins
- **Souls**: 0 souls (need 4 to win)
- **Max Coins**: 99 (Isaac rule)

### Room Settings

- **Max Players**: 4 per room
- **Min Players**: 2 to start a game
- **Room States**: Lobby → Starting → InGame → Finished

## 🧪 Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run specific test modules
cargo test room_tests
cargo test room_manager_tests
cargo test messages_tests

# Run with output
cargo test -- --nocapture
```

### Test Coverage

- ✅ Room creation and management
- ✅ Player joining/leaving
- ✅ Ready system and game start
- ✅ Message serialization/deserialization
- ✅ Error handling
- ✅ Connection management

## 🛠️ Development

### Project Structure

```
src/
├── game/
│   ├── mod.rs              # Game module exports
│   ├── room.rs             # Individual room management
│   ├── room_manager.rs     # Multi-room coordination
│   ├── resources.rs        # Player resource tracking
│   └── order.rs            # Turn order system (TODO)
├── network/
│   ├── mod.rs              # Network module exports
│   ├── websocket_server.rs # Main WebSocket server
│   ├── connection_manager.rs # Connection handling
│   └── messages.rs         # Message types and handling
├── lib.rs                  # Library exports
└── main.rs                 # Server entry point
```

### Dependencies

- `tokio` - Async runtime
- `tokio-tungstenite` - WebSocket implementation
- `serde` - JSON serialization
- `uuid` - Unique ID generation
- `futures-util` - Async utilities
- `rand` - Random number generation

## 🔄 Current Status

### ✅ Implemented Features

- WebSocket server with connection management
- Room creation and joining system
- Player ready system and game start detection
- Chat messaging
- Player resource tracking
- Comprehensive error handling
- Unit test suite

### 🚧 In Progress

- Turn order system (see `src/game/order.rs`)
- Game action handling
- Card system integration

### 📋 Planned Features

- Complete game logic implementation
- Card deck management
- Victory condition handling
- Spectator mode
- Admin commands

## 📄 License

This project is for educational and non-commercial use only. Isaac Four Souls is a trademark of Edmund McMillen and Maestro Media.

## 🔗 Related

- [Isaac Four Souls Official Rules](https://www.isaac-four-souls.com/)
- [The Binding of Isaac](https://bindingofisaac.com/)
