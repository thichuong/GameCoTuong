# GameCoTuong – Architecture Overview

> Cờ Tướng (Xiangqi / Chinese Chess) engine & full-stack multiplayer application viết bằng Rust.

```mermaid
graph TB
    subgraph Workspace["Rust Workspace (Cargo)"]
        direction TB
        Core["cotuong_core<br/>(Library Crate)"]
        Server["server<br/>(Binary Crate)"]
        Client["client<br/>(WASM Binary)"]
        Shared["shared<br/>(Library Crate)"]
    end

    Client -->|"uses"| Core
    Client -->|"uses"| Shared
    Server -->|"uses"| Core
    Server -->|"uses"| Shared
    Client -- "WebSocket<br/>JSON (GameMessage ↔ ServerMessage)" --> Server

    style Core fill:#2d6a4f,color:#fff
    style Server fill:#1d3557,color:#fff
    style Client fill:#e76f51,color:#fff
    style Shared fill:#6c757d,color:#fff
```

---

## 1. Workspace Structure

```
GameCoTuong/
├── Cargo.toml          # Workspace root – clippy pedantic/nursery, LTO fat release
├── cotuong_core/       # AI Engine + Game Logic (library)
├── server/             # Multiplayer WebSocket Server (binary)
├── client/             # WASM Frontend – Leptos (binary)
├── shared/             # Protocol messages (library)
└── test_all.sh         # Integration test runner
```

Workspace-level lints **deny** tất cả `warnings`, `clippy::all`, `clippy::pedantic`, `clippy::nursery`, `clippy::unwrap_used`, `clippy::expect_used`, `clippy::indexing_slicing`.

---

## 2. `cotuong_core` – AI Engine & Game Logic

Crate trung tâm, cung cấp toàn bộ luật chơi và AI engine. Được compile cả native (server) và WASM (client).

### 2.1. Module Map

```
cotuong_core/src/
├── lib.rs              # Re-exports: engine, logic, worker
├── worker.rs           # gloo-worker Web Worker bridge (WASM)
├── engine/
│   ├── mod.rs          # Traits: Evaluator, Searcher; Structs: Move, SearchLimit, SearchStats
│   ├── config.rs       # EngineConfig – JSON-configurable parameters
│   ├── search.rs       # AlphaBetaEngine – Negamax search (~900 lines)
│   ├── eval.rs         # SimpleEvaluator – Board evaluation (~420 lines)
│   ├── movegen.rs      # EngineMoveGen – Engine-specific move generation with scoring
│   ├── tt.rs           # TranspositionTable – Hash-indexed cache
│   ├── zobrist.rs      # ZobristKeys – Position hashing (XorShift64 RNG)
│   ├── move_list.rs    # MoveList – Stack-allocated [Move; 128]
│   ├── bench_test.rs   # Performance benchmarks
│   ├── mate_test.rs    # Checkmate detection tests
│   └── search_test_snippet.rs
└── logic/
    ├── mod.rs
    ├── board.rs         # Board, Piece, Color, PieceType, BoardCoordinate, Bitboard (u128)
    ├── game.rs          # GameState – Turn management, move history, undo, repetition
    ├── generator.rs     # MoveGenerator – Legal move generation (lookup table-based)
    ├── rules.rs         # Move validation, check detection, flying general
    ├── lookup.rs        # AttackTables – Precomputed rook/cannon/horse/elephant/... moves
    ├── eval_constants.rs # Piece values, PST tables, mobility weights
    ├── opening.rs       # Opening book (hardcoded FEN-based)
    └── repetition_test.rs
```

### 2.2. Board Representation

```mermaid
classDiagram
    class Board {
        +squares: [Option~Piece~; 90]
        +piece_bb: [[Bitboard; 7]; 2]
        +color_bb: [Bitboard; 2]
        +hash: u64
        +incremental_score: [i32; 2]
        +to_fen_string(turn) String
        +from_fen(fen) Result
        +apply_move(mv, turn)
        +undo_move(mv, captured, turn)
    }

    class Piece {
        +piece_type: PieceType
        +color: Color
    }

    class BoardCoordinate {
        +row: usize
        +col: usize
        +new(row, col) Option~Self~
        +index() usize
    }

    Board --> Piece : contains 90 squares
    Board --> BoardCoordinate : uses for addressing

    note for Board "Bitboard = u128 (90 squares on 10×9 grid)"
```

| Feature | Implementation |
|---|---|
| Grid | 10 rows × 9 cols = 90 squares |
| Square Storage | `[Option<Piece>; 90]` cho truy cập nhanh |
| Bitboard | `u128` – 1 bit/ô, dùng cho piece tracking & attack detection |
| Hash | Zobrist hashing (`u64`) – incremental update khi move/undo |
| Score | Incremental score `[i32; 2]` (Red/Black) – cập nhật khi move |
| FEN | Hỗ trợ import/export chuẩn Xiangqi FEN |

### 2.3. AI Engine – `AlphaBetaEngine`

```mermaid
flowchart TD
    Start["search()"] --> ID["Iterative Deepening<br/>(depth 1 → max_depth)"]
    ID --> AB["alpha_beta()<br/>Negamax with α-β pruning"]

    AB --> TT{"Transposition<br/>Table Probe?"}
    TT -->|Hit| TTCut["TT Cutoff / Best Move"]
    TT -->|Miss| Pruning

    subgraph Pruning["Pruning Techniques"]
        NMP["Null-Move Pruning<br/>(R=3 reduction)"]
        PC["ProbCut<br/>(statistical cutoff)"]
        LMR["Late Move Reduction<br/>(Precomputed tables)"]
        SE["Singular Extension<br/>(Skip excluded move)"]
    end

    Pruning --> MoveGen["EngineMoveGen<br/>generate_moves()<br/>Move Ordering:<br/>1. TT Move<br/>2. Captures (MVV-LVA)<br/>3. Killer Moves<br/>4. History Heuristic"]

    MoveGen --> Recurse["Recursive α-β"]
    Recurse --> QS["quiescence()<br/>Captures only"]

    AB --> Store["TT Store"]
    AB --> TimeCheck{"Time<br/>Expired?"}
    TimeCheck -->|Yes| Return["Return Best"]
    TimeCheck -->|No| Recurse

    style AB fill:#2d6a4f,color:#fff
    style QS fill:#1d3557,color:#fff
```

| Technique | Details |
|---|---|
| Search | Negamax Alpha-Beta với Iterative Deepening |
| Move Ordering | TT move → Captures (MVV-LVA) → Killer moves → History heuristic |
| Null-Move Pruning | Reduction R=3, skip khi in-check |
| ProbCut | Statistical forward pruning dựa trên shallow search |
| LMR | Late Move Reduction – bảng precomputed `[[u8; 64]; 64]` |
| Singular Extension | Extend nước đi duy nhất tốt đáng kể |
| Quiescence | Search captures-only để tránh horizon effect |
| Transposition Table | Hash table power-of-2 size, replace-if-deeper scheme |
| Zobrist Hashing | `XorShift64` RNG, `OnceLock` singleton, incremental update |
| Time Control | `Depth(u8)` hoặc `Time(u64)` ms, check mỗi 4096 nodes |
| Repetition Detection | History hash tracking |
| Opening Book | FEN-based lookup (hardcoded starting positions) |

### 2.4. `EngineMoveGen` – Engine Move Generation

Module chuyên biệt cho engine, tách riêng khỏi `MoveGenerator` trong `logic/`:

| Feature | Description |
|---|---|
| `MoveGenContext` | Struct chứa board state + move list + config cho quá trình sinh nước đi |
| `EngineMoveGen` | Wrapper sử dụng `AttackTables` + `EngineConfig` để sinh và **chấm điểm** nước đi |
| Move Scoring | Hash move → MVV-LVA captures → Killer moves → History heuristic |
| Piece-specific | `gen_rook_moves`, `gen_cannon_moves`, `gen_horse_moves`, `gen_elephant_moves`, `gen_advisor_moves`, `gen_king_moves`, `gen_pawn_moves` |
| `generate_captures` | Chỉ sinh nước ăn quân – dùng cho quiescence search |

### 2.5. Evaluation – `SimpleEvaluator`

| Component | Description |
|---|---|
| Material | Tổng giá trị quân: Tướng=6000, Xe=600, Pháo=285, Mã=270, Tượng/Sĩ=120, Tốt=30 |
| PST | Piece-Square Tables cho 7 loại quân – flip cho Đen |
| Mobility | Đếm nước đi hợp lệ cho Xe, Mã, Pháo, Tốt (capped) |
| King Safety | Penalty cho Tướng bị lộ, Pháo đối mặt Tướng có giá đỡ |
| Structure | Bonus cho Tượng/Sĩ liên kết |
| Incremental | Score cơ bản (material + PST) được cập nhật incremental trong Board |

### 2.6. Move Generation (`logic/generator.rs`)

- **`MoveGenerator`**: Sinh tất cả nước đi hợp lệ cho 1 bên, sử dụng `AttackTables` lookup.
- **`AttackTables`**: Precomputed tại startup (`OnceLock`):
  - Rook/Cannon: Occupancy-indexed attack tables `[[u16; 1024]; 10]`
  - Horse: `[(target, leg_blocker); 90]`
  - Elephant: `[(target, eye_blocker); 90]`
  - Advisor/General/Soldier: `[targets; 90]`
- **`MoveList`**: Stack-allocated `[Move; 128]`, zero-alloc trong hot path.
- **`has_legal_moves()`**: Early-return kiểm tra nhanh có nước đi hợp lệ (dùng cho mate detection).

### 2.7. Web Worker (`worker.rs`)

`GameWorker` implement `gloo_worker::Worker` – chạy AI search trên background thread (WASM):
- **Input**: `ComputeMove(GameState, SearchLimit, EngineConfig, Vec<Move>)`
- **Output**: `MoveFound(Move, SearchStats)`

---

## 3. `server` – Multiplayer WebSocket Server

### 3.1. Stack

| Layer | Technology |
|---|---|
| HTTP/WS | Axum 0.7 + WebSocket upgrade |
| Async Runtime | Tokio (full features) |
| Concurrency | `DashMap` (lock-free) + `tokio::sync::RwLock` / `Mutex` |
| Logging | `tracing` + `tracing-subscriber` (structured logging, env-filter) |
| Serialization | serde_json |

### 3.2. Architecture

```mermaid
flowchart LR
    P1["Player 1<br/>(WebSocket)"]
    P2["Player 2<br/>(WebSocket)"]

    subgraph Server
        WS["ws.rs<br/>WebSocket Handler"]

        subgraph AppState["game_manager/mod.rs"]
            Players["players: DashMap<br/>id → Player"]
            Games["games: DashMap<br/>game_id → RwLock-GameSession"]
            PTG["player_to_game: DashMap<br/>player_id → game_id"]
            Queue["matchmaking_queue:<br/>Mutex-HashSet"]
        end

        subgraph Modules["game_manager/"]
            LC["lifecycle.rs<br/>Player lifecycle"]
            MM["matchmaking.rs<br/>Queue + pairing"]
            MH["move_handler.rs<br/>Move validation"]
            SS["session.rs<br/>Structs"]
            TS["tests.rs"]
        end
    end

    P1 <-->|GameMessage| WS
    P2 <-->|GameMessage| WS
    WS --> AppState
    AppState --> Modules

    style AppState fill:#1d3557,color:#fff
```

### 3.3. Module Map

```
server/src/
├── main.rs                     # Entry point: tracing init, cleanup task, Axum router
├── ws.rs                       # WebSocket upgrade, message routing, rate limiting
└── game_manager/
    ├── mod.rs                  # AppState struct (DashMap-based), check_rate_limit()
    ├── session.rs              # Player, GameSession structs, Tx type, has_any_valid_move()
    ├── lifecycle.rs            # add_player, remove_player, handle_surrender,
    │                           # handle_play_again, leave_game, handle_player_left,
    │                           # spawn_cleanup_task
    ├── matchmaking.rs          # find_match, start_game (random color assignment)
    ├── move_handler.rs         # handle_move, handle_verify_move, resolve_conflict,
    │                           # notify_game_end
    └── tests.rs                # Unit tests for game manager logic
```

| Component | Responsibility |
|---|---|
| `ws.rs` | WebSocket upgrade, message routing (deserialize `GameMessage` → dispatch), rate limiting |
| `AppState` | Stateful game manager – DashMap-based concurrent access, rate limiting per player |
| `GameSession` | Per-game state: Board, turn, players, pending moves, rematch readiness, last activity |
| `Player` | WebSocket sender channel (`Tx`) + last message timestamp (rate limiting) |
| Matchmaking | Queue-based: `FindMatch` → pair 2 players → `start_game()` (random color) |
| Move Validation | Distributed: sender submits → relay to opponent → opponent cross-validates → resolve conflicts |
| Game End | Checkmate detection, surrender, disconnect, draw |
| Lifecycle | Player cleanup on disconnect, stale game cleanup task, rematch handling |
| Cleanup Task | Background `spawn_cleanup_task()` – tự động xóa game sessions không hoạt động |

### 3.4. Message Flow

```mermaid
sequenceDiagram
    participant P1 as Player 1
    participant S as Server
    participant P2 as Player 2

    P1->>S: FindMatch
    P2->>S: FindMatch
    S->>P1: MatchFound(opponent_id, color, game_id)
    S->>P2: MatchFound(opponent_id, color, game_id)
    S->>P1: GameStart(Board)
    S->>P2: GameStart(Board)

    P1->>S: MakeMove(move, fen)
    S->>P2: OpponentMove(move, fen)
    S->>P2: VerifyMove(fen)
    P2->>S: VerifyMove(fen, is_valid)
    Note over S: If conflict → resolve_conflict()

    Note over P1,P2: Game End
    S->>P1: GameEnd(winner, reason)
    S->>P2: GameEnd(winner, reason)
    P1->>S: PlayAgain
    P2->>S: PlayAgain
    Note over S: Both ready → start new game

    Note over P1,P2: Player leaves after game
    P1->>S: PlayerLeft
    S->>P2: OpponentLeftGame
```

---

## 4. `client` – WASM Frontend

### 4.1. Stack

| Layer | Technology |
|---|---|
| Framework | Leptos 0.6 (CSR mode) |
| Compile Target | `wasm32-unknown-unknown` via Trunk |
| AI Worker | `gloo-worker` Web Worker (non-blocking UI) |
| Network | `web_sys::WebSocket` |
| Rendering | HTML Canvas (`CanvasRenderingContext2d`) |

### 4.2. Module Map

```
client/src/
├── main.rs                 # Entry point: mount App component
├── network.rs              # NetworkClient (WebSocket wrapper)
├── app/
│   ├── mod.rs              # Shared enums: Difficulty (5 levels), GameMode, OnlineStatus
│   ├── game_app.rs         # Main App component (~444 lines) – orchestrates all game modes
│   ├── controls.rs         # ControlsArea – mode/side/difficulty selectors, action buttons
│   ├── config.rs           # ConfigPanel, Slider, Dropdown, FloatSlider – AI parameter tuning
│   ├── export.rs           # handle_file_upload, export_config (JSON), export_csv
│   ├── log.rs              # LogPanel (move history), ThinkingIndicator
│   ├── online.rs           # OnlineStatusPanel – online mode UI & matchmaking controls
│   └── styles.rs           # GAME_STYLES – embedded CSS constants
├── components/
│   ├── mod.rs
│   └── board.rs            # BoardView – Canvas rendering
└── bin/
    └── worker.rs           # Web Worker entry point
```

### 4.3. Game Modes & Difficulty

| Mode | Description |
|---|---|
| `HumanVsComputer` | Người chơi vs AI (Web Worker) |
| `ComputerVsComputer` | AI vs AI (tự động, có nút Pause/Resume) |
| `HumanVsHuman` | 2 người chơi local (hotseat) |
| `Online` | Multiplayer qua WebSocket |

| Difficulty | Time Limit |
|---|---|
| Level 1 | 1 giây |
| Level 2 | 2 giây |
| Level 3 | 5 giây |
| Level 4 | 10 giây |
| Level 5 | 20 giây |

### 4.4. Online Status Flow

```mermaid
stateDiagram-v2
    [*] --> None
    None --> Finding : FindMatch
    Finding --> None : CancelFindMatch
    Finding --> MatchFound : MatchFound
    MatchFound --> Playing : GameStart
    Playing --> GameEnded : GameEnd
    Playing --> OpponentDisconnected : OpponentDisconnected
    GameEnded --> None : PlayerLeft / OpponentLeftGame
    OpponentDisconnected --> None : cleanup
```

### 4.5. Rendering Pipeline

Board được render trên HTML Canvas:
1. Vẽ grid 10×9 với các đường kẻ, sông, cung
2. Vẽ quân cờ tại vị trí (circle + text)
3. Highlight: ô được chọn, nước đi hợp lệ, nước đi cuối cùng
4. Interactive: click-to-select, click-to-move

---

## 5. `shared` – Protocol Layer

Chứa 2 enum được serialize/deserialize qua JSON:

### `GameMessage` (Client → Server)
| Variant | Purpose |
|---|---|
| `FindMatch` | Yêu cầu tìm trận |
| `CancelFindMatch` | Hủy tìm trận |
| `MakeMove { move_data, fen }` | Gửi nước đi |
| `VerifyMove { fen, is_valid }` | Xác nhận nước đi đối thủ |
| `Surrender` | Đầu hàng |
| `RequestDraw` / `AcceptDraw` | Đề nghị / chấp nhận hòa |
| `PlayAgain` | Chơi lại (rematch) |
| `PlayerLeft` | Rời trận sau khi game kết thúc |

### `ServerMessage` (Server → Client)
| Variant | Purpose |
|---|---|
| `MatchFound { opponent_id, your_color, game_id }` | Đã ghép trận |
| `GameStart(Box<Board>)` | Bắt đầu game (Board được Box để giảm stack size) |
| `OpponentMove { move_data, fen }` | Đối thủ đi |
| `GameStateCorrection { fen, turn }` | Sửa state khi conflict |
| `GameEnd { winner, reason }` | Kết thúc game |
| `Error(String)` | Lỗi |
| `WaitingForMatch` | Đang chờ đối thủ |
| `OpponentDisconnected` | Đối thủ mất kết nối (during game) |
| `OpponentLeftGame` | Đối thủ rời trận (after game ended) |

---

## 6. Data Flow Overview

```mermaid
flowchart TD
    subgraph Client["Client (WASM)"]
        UI["App Component<br/>(Leptos Signals)"]
        Canvas["BoardView<br/>(Canvas)"]
        Worker["GameWorker<br/>(Web Worker)"]
        Net["NetworkClient<br/>(WebSocket)"]
    end

    subgraph Core["cotuong_core"]
        GS["GameState"]
        Engine["AlphaBetaEngine"]
        EMG["EngineMoveGen"]
        Board["Board"]
        MG["MoveGenerator"]
    end

    subgraph Srv["Server"]
        WS["WebSocket Handler<br/>+ Rate Limiting"]
        GM["AppState<br/>(DashMap)"]
        LC["Lifecycle<br/>Manager"]
        MM["Matchmaking"]
    end

    UI --> Canvas
    UI -->|"Trigger AI"| Worker
    Worker --> Engine
    Engine --> EMG
    Engine --> GS
    GS --> Board
    GS --> MG
    Worker -->|"MoveFound"| UI

    UI -->|"Online mode"| Net
    Net <-->|"JSON"| WS
    WS --> GM
    GM --> LC
    GM --> MM
    GM -->|"validate"| Board

    style Engine fill:#2d6a4f,color:#fff
    style GM fill:#1d3557,color:#fff
    style UI fill:#e76f51,color:#fff
```

---

## 7. Build & Run

| Target | Command | Notes |
|---|---|---|
| Server | `cargo run -p server` | Mặc định `127.0.0.1:3000` (cấu hình qua `HOST`/`PORT` env vars) |
| Client | `trunk serve` (trong `client/`) | Cần `trunk` + `wasm32-unknown-unknown` target |
| Tests | `./test_all.sh` hoặc `cargo test --workspace` | Bao gồm unit + integration tests |
| Release | Profile: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"` | Tối ưu size & performance |

---

## 8. Key Design Decisions

1. **Rust Full-Stack**: Cùng ngôn ngữ cho engine, server, client → chia sẻ types, zero runtime overhead.
2. **WASM + Web Worker**: AI chạy trên worker thread → UI không bị block khi engine search.
3. **DashMap Concurrency**: Server dùng lock-free `DashMap` thay vì global `Mutex` → không bottleneck.
4. **Bitboard u128**: 90 ô (10×9) fit trong `u128` → bitwise operations nhanh.
5. **Stack-allocated MoveList**: `[Move; 128]` trên stack → zero heap allocation trong search loop.
6. **Incremental Evaluation**: Board cập nhật hash + score khi move/undo → tránh recompute.
7. **Precomputed Lookup Tables**: `AttackTables` + `ZobristKeys` dùng `OnceLock` singleton → tính 1 lần dùng mãi.
8. **Distributed Move Validation**: Server yêu cầu cả 2 player validate → tăng bảo mật, giảm tải server.
9. **Modular Server Architecture**: `game_manager` tách thành `lifecycle`, `matchmaking`, `move_handler`, `session` → dễ bảo trì.
10. **Structured Logging**: Server dùng `tracing` với env-filter → debug hiệu quả, không ảnh hưởng performance.
11. **Rate Limiting**: Server giới hạn 10 messages/giây/player → chống spam, bảo vệ server.
12. **Separated Engine MoveGen**: `EngineMoveGen` tách riêng khỏi `MoveGenerator` logic → engine có move scoring, logic chỉ sinh nước hợp lệ.
