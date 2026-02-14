---
name: multiplayer_server
description: Guidelines for modifying the multiplayer WebSocket server – game session management, matchmaking, distributed validation, and the shared protocol.
---

# Multiplayer Server & Protocol Skill

## Scope

This skill covers:
- [server/src/main.rs](file:///home/exblackhole/Desktop/GameCoTuong/server/src/main.rs) – Axum server setup, tracing init, cleanup task
- [server/src/ws.rs](file:///home/exblackhole/Desktop/GameCoTuong/server/src/ws.rs) – WebSocket handler, message routing, rate limiting
- [server/src/game_manager/mod.rs](file:///home/exblackhole/Desktop/GameCoTuong/server/src/game_manager/mod.rs) – AppState struct, rate limiting
- [server/src/game_manager/session.rs](file:///home/exblackhole/Desktop/GameCoTuong/server/src/game_manager/session.rs) – Player, GameSession, Tx, has_any_valid_move()
- [server/src/game_manager/lifecycle.rs](file:///home/exblackhole/Desktop/GameCoTuong/server/src/game_manager/lifecycle.rs) – Player add/remove, surrender, play again, cleanup
- [server/src/game_manager/matchmaking.rs](file:///home/exblackhole/Desktop/GameCoTuong/server/src/game_manager/matchmaking.rs) – find_match, start_game
- [server/src/game_manager/move_handler.rs](file:///home/exblackhole/Desktop/GameCoTuong/server/src/game_manager/move_handler.rs) – Move processing, verification, conflict resolution
- [server/src/game_manager/tests.rs](file:///home/exblackhole/Desktop/GameCoTuong/server/src/game_manager/tests.rs) – Unit tests
- [shared/src/lib.rs](file:///home/exblackhole/Desktop/GameCoTuong/shared/src/lib.rs) – GameMessage, ServerMessage enums
- [client/src/network.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/network.rs) – NetworkClient (WebSocket wrapper)

## Architecture Context

### Server Stack
- **Axum 0.7**: HTTP server + WebSocket upgrade
- **Tokio**: Async runtime (full features)
- **DashMap 6.1**: Lock-free concurrent hashmap for game sessions, players, player-game mapping
- **tokio::sync::Mutex**: For matchmaking queue (`HashSet<String>`)
- **tokio::sync::RwLock**: For individual `GameSession` write access inside DashMap
- **tracing + tracing-subscriber**: Structured logging with env-filter support

### Module Layout
```
server/src/game_manager/
├── mod.rs           # AppState struct, check_rate_limit()
├── session.rs       # Player, GameSession structs, Tx type
├── lifecycle.rs     # add_player, remove_player, handle_surrender,
│                    # handle_play_again, leave_game, handle_player_left,
│                    # spawn_cleanup_task
├── matchmaking.rs   # find_match, start_game (random color assignment)
├── move_handler.rs  # handle_move, handle_verify_move, resolve_conflict,
│                    # notify_game_end
└── tests.rs         # Unit tests
```

### AppState Structure
```rust
pub struct AppState {
    pub players: DashMap<String, Player>,              // player_id → Player (tx + rate limit)
    pub games: DashMap<String, RwLock<GameSession>>,   // game_id → session
    pub player_to_game: DashMap<String, String>,       // player_id → game_id
    pub matchmaking_queue: Mutex<HashSet<String>>,
}
```

### Player & GameSession Structures
```rust
pub struct Player {
    pub tx: Tx,                    // mpsc::UnboundedSender<ServerMessage>
    pub last_msg_at: Instant,      // Rate limiting timestamp
}

pub struct GameSession {
    pub red_player: String,
    pub black_player: String,
    pub board: Board,
    pub turn: Color,
    pub game_ended: bool,
    pub red_ready_for_rematch: bool,
    pub black_ready_for_rematch: bool,
    pub pending_move: Option<(String, Move, String)>,  // (player_id, move, fen)
    pub last_activity: Instant,
}
```

### Distributed Move Validation Flow
1. Player A sends `MakeMove { move_data, fen }`
2. Server validates: correct turn, correct player
3. Server stores move as pending, forwards to Player B as `OpponentMove`
4. Server asks Player B to `VerifyMove { fen }`
5. Player B replies `VerifyMove { fen, is_valid: true/false }`
6. If both agree → apply move, switch turn, check for checkmate
7. If conflict → `resolve_conflict()` → server-side validation using Board rules

### Message Routing (ws.rs)
```
GameMessage::FindMatch       → state.find_match(player_id)
GameMessage::MakeMove        → state.handle_move(player_id, mv, fen)
GameMessage::VerifyMove      → state.handle_verify_move(player_id, fen, is_valid)
GameMessage::CancelFindMatch → queue.remove(&player_id)
GameMessage::Surrender       → state.handle_surrender(player_id)
GameMessage::PlayAgain       → state.handle_play_again(player_id)
GameMessage::PlayerLeft      → state.handle_player_left(player_id)
```

### Rate Limiting
- `check_rate_limit()` in `AppState` enforces max 10 messages/second per player
- Uses `Player.last_msg_at` timestamp comparison
- Checked in `ws.rs` before processing any `GameMessage`

### Cleanup Task
- `spawn_cleanup_task()` runs as background Tokio task
- Periodically removes stale `GameSession` entries based on `last_activity`

## Rules

### Concurrency
1. Use `DashMap` for all concurrent state access — never `std::sync::Mutex` for hot-path data.
2. When modifying a `GameSession`, acquire `RwLock` write lock and release ASAP.
3. Player message sending via `mpsc::UnboundedSender<ServerMessage>` — fire-and-forget, ignore send errors (player disconnected).
4. Always `drop(queue)` explicitly after matchmaking queue operations to release the lock before starting games.

### Protocol Changes
1. **Both enums must stay in sync**: `GameMessage` (client→server) and `ServerMessage` (server→client) in `shared/src/lib.rs`.
2. Adding a new variant requires updates in:
   - `shared/src/lib.rs` – add enum variant
   - `server/src/ws.rs` – add match arm for routing
   - `server/src/game_manager/` – implement handler in appropriate module
   - `client/src/app/online.rs` – handle incoming ServerMessage
   - `client/src/app/game_app.rs` – update message processing effect
   - `client/src/network.rs` – ensure NetworkClient can send new GameMessage
3. All messages are serialized/deserialized via `serde_json`.

### Logging
1. Use `tracing::info!`, `tracing::debug!`, `tracing::warn!`, `tracing::error!` with structured fields.
2. Always include `player_id` field in log spans: `tracing::info!(player_id = %id, "message")`.
3. Default log filter: `server=debug,tower_http=info` (configurable via `RUST_LOG` env var).

### Error Handling
1. WebSocket message parsing uses pattern matching — malformed messages logged and skipped.
2. Player disconnect handled in `remove_player()` (`lifecycle.rs`) — cleans up games, notifies opponents.
3. DashMap access uses safe methods; no `.unwrap()` on lookup results.

### Testing
1. Tests in `game_manager/tests.rs` use `mpsc::unbounded_channel()` to simulate player connections.
2. Helper functions: `expect_msg_timeout()`, `drain_setup_messages()`.
3. Test async with `#[tokio::test]` and `tokio::time::timeout`.

## Common Tasks

### Adding a new message type
1. Add variant to `GameMessage` and/or `ServerMessage` in `shared/src/lib.rs`
2. Add match arm in `ws.rs::handle_socket()`
3. Implement handler method in `AppState` in the appropriate module:
   - Game lifecycle → `lifecycle.rs`
   - Matchmaking → `matchmaking.rs`
   - Move-related → `move_handler.rs`
4. Update client to send/receive the new message (`app/online.rs`, `app/game_app.rs`)
5. Add test in `game_manager/tests.rs`

### Adding a new game feature (e.g., timer, spectating)
1. Add required fields to `GameSession` struct in `session.rs`
2. Add initialization in `start_game()` (`matchmaking.rs`)
3. Add protocol messages for the feature (`shared/src/lib.rs`)
4. Implement game logic in appropriate `game_manager/` module
5. Handle cleanup in `remove_player()` / `leave_game()` (`lifecycle.rs`)

### Modifying matchmaking
1. Current: simple queue (`HashSet`), pair first 2 players with random color assignment
2. The `find_match()` method in `matchmaking.rs` locks queue, checks for partner, calls `start_game()`
3. To add ELO/rating: store rating in `Player` struct (`session.rs`), sort queue by rating in `matchmaking.rs`
