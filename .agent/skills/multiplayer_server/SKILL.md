---
name: multiplayer_server
description: Guidelines for modifying the multiplayer WebSocket server – game session management, matchmaking, distributed validation, and the shared protocol.
---

# Multiplayer Server & Protocol Skill

## Scope

This skill covers:
- [server/src/main.rs](file:///home/exblackhole/Desktop/GameCoTuong/server/src/main.rs) – Axum server setup
- [server/src/ws.rs](file:///home/exblackhole/Desktop/GameCoTuong/server/src/ws.rs) – WebSocket handler
- [server/src/game_manager.rs](file:///home/exblackhole/Desktop/GameCoTuong/server/src/game_manager.rs) – AppState, GameSession, matchmaking, validation
- [shared/src/lib.rs](file:///home/exblackhole/Desktop/GameCoTuong/shared/src/lib.rs) – GameMessage, ServerMessage enums
- [client/src/network.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/network.rs) – NetworkClient (WebSocket wrapper)

## Architecture Context

### Server Stack
- **Axum 0.7**: HTTP server + WebSocket upgrade
- **Tokio**: Async runtime (full features)
- **DashMap**: Lock-free concurrent hashmap for game sessions, players, player-game mapping
- **tokio::sync::Mutex**: For matchmaking queue (`HashSet<String>`)
- **tokio::sync::RwLock**: For individual `GameSession` write access inside DashMap

### AppState Structure
```rust
pub struct AppState {
    pub players: DashMap<String, Tx>,           // player_id → message sender
    pub games: DashMap<String, RwLock<GameSession>>,  // game_id → session
    pub player_games: DashMap<String, String>,  // player_id → game_id
    pub matchmaking_queue: Mutex<HashSet<String>>,
}
```

### GameSession Structure
```rust
pub struct GameSession {
    pub board: Board,
    pub turn: Color,
    pub red_player: Player,
    pub black_player: Player,
    pub pending_move: Option<Move>,
    pub pending_fen: Option<String>,
    pub move_count: u32,
    pub last_move: Option<Move>,
}
```

### Distributed Move Validation Flow
1. Player A sends `MakeMove { move_data, fen }`
2. Server validates: correct turn, correct player
3. Server stores move as pending, forwards to Player B as `OpponentMove`
4. Server asks Player B to `VerifyMove { fen }`
5. Player B replies `VerifyMove { fen, is_valid: true/false }`
6. If both agree → apply move, switch turn
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

## Rules

### Concurrency
1. Use `DashMap` for all concurrent state access — never `std::sync::Mutex` for hot-path data.
2. When modifying a `GameSession`, acquire `RwLock` write lock and release ASAP.
3. Player message sending via `mpsc::UnboundedSender<ServerMessage>` — fire-and-forget, ignore send errors (player disconnected).

### Protocol Changes
1. **Both enums must stay in sync**: `GameMessage` (client→server) and `ServerMessage` (server→client) in `shared/src/lib.rs`.
2. Adding a new variant requires updates in:
   - `shared/src/lib.rs` – add enum variant
   - `server/src/ws.rs` – add match arm for routing
   - `server/src/game_manager.rs` – implement handler
   - `client/src/app.rs` – handle incoming ServerMessage
   - `client/src/network.rs` – ensure NetworkClient can send new GameMessage
3. All messages are serialized/deserialized via `serde_json`.

### Error Handling
1. WebSocket message parsing uses `if let Ok(...)` — malformed messages silently ignored.
2. Player disconnect handled in `state.remove_player()` — cleans up games, notifies opponents.
3. DashMap access uses safe methods; no `.unwrap()` on lookup results.

### Testing
1. Tests in `game_manager.rs` use `mpsc::unbounded_channel()` to simulate player connections.
2. Helper functions: `expect_msg_timeout()`, `drain_setup_messages()`.
3. Test async with `#[tokio::test]` and `tokio::time::timeout`.

## Common Tasks

### Adding a new message type
1. Add variant to `GameMessage` and/or `ServerMessage` in `shared/src/lib.rs`
2. Add match arm in `ws.rs::handle_socket()`
3. Implement handler method in `AppState` (game_manager.rs)
4. Update client to send/receive the new message
5. Add test in `game_manager.rs`

### Adding a new game feature (e.g., timer, spectating)
1. Add required fields to `GameSession` struct
2. Add initialization in `start_game()`
3. Add protocol messages for the feature
4. Implement game logic in AppState methods
5. Handle cleanup in `remove_player()`

### Modifying matchmaking
1. Current: simple queue (`HashSet`), pair first 2 players
2. The `find_match()` method locks queue, checks for partner, calls `start_game()`
3. To add ELO/rating: store rating in `Player` struct, sort queue by rating
