# Isaac Four Souls - Multiplayer Game Server

A real-time multiplayer WebSocket server implementation of "The Binding of Isaac: Four Souls" card game, built with Rust and Tokio.

## Features

- **Real-time multiplayer gameplay** via WebSocket connections
- **Actor-based architecture** for scalable game state management
- **Reliable messaging system** with automatic retry and ordering
- **Turn-based game mechanics** with priority passing
- **Lobby system** for room creation and player matchmaking
- **Card game engine** with loot deck management

## Architecture

The server uses an actor-based architecture with the following key components:

### Core Actors

- **LobbyActor**: Manages room creation, player joining, and game initialization
- **GameActor**: Handles in-game logic, turn management, and card interactions
- **ConnectionActor**: Manages individual player connections and message routing
- **ActorRegistry**: Centralized registry for actor communication and lifecycle management

### Game Components

- **Board**: Manages game state including player hands, loot deck, and discard pile
- **TurnOrder**: Handles turn sequencing and player rotation
- **StateBroadcaster**: Sends game state updates to all players
- **GameCoordinator**: Coordinates game events and state transitions

## Quick Start

### Prerequisites

- Rust 1.70+ with Cargo
- No additional dependencies required

### Running the Server

```bash
# Clone the repository
git clone <repository-url>
cd isaac-four-souls

# Run the server
cargo run

# The server will start on 127.0.0.1:8080
```

### Testing with a WebSocket Client

```bash
# Using websocat (install with: cargo install websocat)
websocat ws://127.0.0.1:8080

# Example messages to send:
{"Ping": null}
{"CreateRoom": {"room_name": "My Game", "first_player_name": "Player1"}}
{"JoinRoom": {"player_name": "Player2", "room_id": "room-id-here"}}
```

## Game Flow

### Lobby Phase

1. Players connect and receive a unique connection ID
2. Create or join game rooms
3. Players mark themselves as ready
4. Game starts automatically when all players are ready

### Game Phase

1. **Turn Structure**: Each turn has multiple phases:

   - Untap/Start Step
   - Loot Step (draw cards)
   - Action Step (play cards)
   - End Step
   - Turn End (pass to next player)

2. **Priority System**: Players can pass priority during each phase
3. **Card Management**: Automatic deck shuffling and hand management

## WebSocket API

### Client Messages

```json
// Lobby Messages
{"Ping": null}
{"Chat": {"message": "Hello!"}}
{"CreateRoom": {"room_name": "Game Room", "first_player_name": "Alice"}}
{"JoinRoom": {"player_name": "Bob", "room_id": "room-123"}}
{"LeaveRoom": null}
{"PlayerReady": null}

// Game Messages
{"TurnPass": null}
{"PriorityPass": null}
```

### Server Responses

```json
// Connection Events
{"ConnectionId": {"connection_id": "conn-123"}}
{"Pong": null}

// Lobby Events
{"RoomCreated": {"room_id": "room-123", "player_id": "player-456"}}
{"PlayerJoined": {"player_name": "Alice", "player_id": "player-456"}}
{"RoomGameStart": {"turn_order": ["player1", "player2"]}}

// Game Events
{"PublicBoardState": {
  "hand_sizes": {"player1": 3, "player2": 2},
  "loot_deck_size": 15,
  "current_phase": "ActionStep",
  "active_player": "player1"
}}

{"PrivateBoardState": {
  "hand": [{"name": "A Penny", "description": "Gain 1¢"}]
}}

// Error Handling
{"Error": {
  "error_type": "RoomFull",
  "message": "Room is full (maximum 4 players)",
  "code": 400
}}
```

## Project Structure

```
src/
├── actors/              # Actor system implementation
│   ├── actor_registry.rs   # Central actor management
│   ├── connection_actor.rs # Individual connection handling
│   ├── game_actor.rs      # Game logic coordination
│   └── lobby_actor.rs     # Room and lobby management
├── game/               # Game logic and state
│   ├── board.rs           # Game board and player state
│   ├── card_loader.rs     # Card database loading
│   ├── cards_types.rs     # Card type definitions
│   ├── game_coordinator.rs # Game event processing
│   ├── game_state.rs      # Core game state management
│   └── state_broadcaster.rs # State synchronization
├── network/            # Networking and communication
│   ├── connection_handler.rs # WebSocket connection handling
│   ├── connection_manager.rs # Connection lifecycle
│   ├── messages.rs        # Message serialization
│   ├── reliable_messaging.rs # Message delivery guarantees
│   └── server.rs          # Main server implementation
├── data/               # Game data files
│   └── cards/
│       └── loot.json      # Loot card definitions
├── errors.rs           # Error types and handling
├── lib.rs             # Library exports
└── main.rs            # Application entry point
```

## Key Features

### Reliable Messaging

- Automatic message ordering and delivery guarantees
- Retry logic with exponential backoff
- Duplicate detection and handling

### Error Handling

- Comprehensive error types with user-friendly messages
- Graceful degradation for network issues
- Detailed logging for debugging

### Scalability

- Actor-based architecture allows for horizontal scaling
- Non-blocking I/O with Tokio async runtime
- Efficient memory usage with reference counting

## Configuration

### Game Settings

- **Max Players per Room**: 4 (configurable in `Room::DEFAULT_MAX_PLAYERS`)
- **Min Players to Start**: 2 (configurable in `Room::DEFAULT_MIN_PLAYERS`)
- **Starting Hand Size**: 3 cards per player
- **Starting Health**: 2 HP per player

### Server Settings

- **Default Port**: 8080
- **Connection Timeout**: Configurable via Tokio settings
- **Message Retry Count**: 3 attempts (in `ConnectionActor::send_reliable`)

## Development

### Adding New Card Types

1. Define the card in `src/data/cards/loot.json`
2. Update card loading logic in `card_loader.rs`
3. Implement card effects in the game coordinator

### Adding New Game Phases

1. Extend `TurnPhases` enum in `game_state.rs`
2. Update phase transition logic
3. Add phase-specific event handling

### Testing

```bash
# Run all tests
cargo test

# Run with logging
RUST_LOG=debug cargo run

# Check code formatting
cargo fmt --check

# Run clippy for linting
cargo clippy
```

## Dependencies

- **tokio**: Async runtime and networking
- **tokio-tungstenite**: WebSocket server implementation
- **serde**: JSON serialization/deserialization
- **uuid**: Unique identifier generation
- **rand**: Random number generation for game mechanics
- **dashmap**: Concurrent hash map for actor registry
- **once_cell**: Lazy static initialization
- **thiserror**: Error handling macros

## Contributing

1. Fork the repository
2. Create a feature branch
3. Implement your changes with tests
4. Run `cargo fmt` and `cargo clippy`
5. Submit a pull request

## License

[Specify your license here]

## Roadmap

- [ ] Implement monster cards and combat system
- [ ] Add treasure and character card types
- [ ] Implement item activation mechanics
- [ ] Add persistent game state storage
- [ ] Create web-based game client
- [ ] Add spectator mode
- [ ] Implement game replays and statistics
