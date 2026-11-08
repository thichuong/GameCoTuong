---
name: game_logic_rules
description: Guidelines for modifying Xiangqi game rules, board representation, incremental NNUE accumulator updates, move generation, repetition rules, and FEN/puzzle validation.
---

# Game Logic & Rules Skill

## Scope

This skill covers modifications to files in `cotuong_core/src/logic/`, its WASM wrapper, and the TypeScript FEN/Polyfill equivalents:
- `cotuong_core/src/logic/board.rs` – `Board`, `Piece`, `Color`, `PieceType`, `BoardCoordinate`, `Bitboard` (`u128`), incremental `zobrist_hash` and `nnue_acc` (`NnueAccumulator`) updates
- `cotuong_core/src/logic/game.rs` – `GameState`, `GameStatus`, `MoveRecord`, turn management, move history, undo, three-fold repetition filtering (`get_strictly_legal_moves`, `is_move_legal`)
- `cotuong_core/src/logic/generator.rs` – `MoveGenerator` (legal move generation using precomputed lookup tables)
- `cotuong_core/src/logic/rules.rs` – Move validation, check detection (`is_in_check`), flying general rule (`is_flying_general`)
- `cotuong_core/src/logic/lookup.rs` – `AttackTables` (precomputed sliding and non-sliding attack masks)
- `cotuong_core/src/logic/eval_constants.rs` – Piece values and Piece-Square Tables (PST) used for prior initialization and material references
- `cotuong_core/src/logic/opening.rs` – Opening book (`get_book_move`, FEN-based)
- `cotuong_core/src/wasm.rs` – `WasmGameState` bindings exposing game logic to the React/TypeScript frontend
- `src/utils/fen.ts` & `src/wasm/enginePolyfill.ts` – Frontend FEN conversion, puzzle board setup validation (`validateBoardSetup`), `CLASSIC_PUZZLES`, and TypeScript polyfill game state

## Architecture Context

### Board Representation (`cotuong_core/src/logic/board.rs`)
- **Grid**: 10 rows × 9 cols = 90 squares (`grid: [Option<Piece>; 90]` for $O(1)$ square lookup)
- **Bitboards**: `bitboards: [Bitboard; 14]` (`color.index() * 7 + piece_type.index()`) + `occupied: Bitboard`
- **Line Occupancy**: `occupied_rows: [u16; 10]` (9 bits) and `occupied_cols: [u16; 9]` (10 bits) for fast sliding piece lookups
- **Bitboard type**: `u128` – bit index = `row * 9 + col`
- **Coordinate**: `BoardCoordinate { row: usize, col: usize }` – bounds-checked via `new()`, unchecked via `new_unchecked()`
- **Zobrist Hash**: `zobrist_hash: u64` – updated incrementally on `apply_move()` / `undo_move()`
- **NNUE Accumulator**: `nnue_acc: NnueAccumulator` – updated incrementally (`add_piece`, `remove_piece`) when pieces move, or refreshed (`refresh_red`, `refresh_black`) when a General moves inside the palace

### Move Validation & Repetition Flow
```
GameState::is_move_legal(from, to) / get_strictly_legal_moves()
├── MoveGenerator::generate_moves(board, turn)
│   ├── Uses AttackTables::get() for candidate targets
│   ├── Simulates move on cloned/incremental board
│   └── Filters out self-check (is_in_check) and flying general (is_flying_general)
└── Filters out moves that would cause an illegal three-fold position repetition
    (unless forced with no alternative legal moves)
```

### Attack Tables (Precomputed)
- Initialized once via `OnceLock` singleton (`AttackTables::get()`)
- **Sliding pieces** (Rook/Cannon): Occupancy-indexed tables `[[u16; 1024]; 10]`
- **Non-sliding pieces**: Precomputed target arrays for each of 90 squares
  - Horse: `[(target_sq, leg_blocker_sq)]`
  - Elephant: `[(target_sq, eye_blocker_sq)]`
  - Advisor/General/Soldier: `[target_sq]`

### Xiangqi-Specific Rules
| Rule | Implementation |
|---|---|
| Palace restriction | General + Advisors confined to 3×3 palace (rows 0–2 for Red, 7–9 for Black, cols 3–5) |
| River restriction | Elephants cannot cross river (rows 0–4 for Red, 5–9 for Black) |
| Horse leg blocking | Horse blocked if piece at orthogonal adjacent square |
| Elephant eye blocking | Elephant blocked if piece at diagonal midpoint square |
| Cannon jumping | Cannon captures by jumping over exactly 1 piece (screen); moves along empty lines |
| Flying General | Two generals cannot face each other on the same file with no pieces between |
| Self-check | A move cannot leave own general in check |
| Three-fold repetition | Repeated positions are tracked in `GameState` and prevented in `get_strictly_legal_moves()` |

## Rules

### Board Modification
1. **Always update all representations**: When modifying board state, update `grid`, `bitboards`, `occupied`, `occupied_rows`, `occupied_cols`, `zobrist_hash`, and `nnue_acc` consistently.
2. Use `Board::apply_move()` and `Board::undo_move()` — they handle all incremental bookkeeping including HalfKP NNUE accumulator updates.
3. When deserializing `GameState` from JSON (`WasmGameState::from_json`), always call `state.board.refresh_nnue()` since `nnue_acc` is skipped during serde serialization (`#[serde(skip)]`).

### FEN & Puzzle Setup Validation
- `Board::to_fen_string(turn)` → standard Xiangqi FEN string
- `Board::from_fen(fen)` → `Result<(Board, Color), String>`
- `src/utils/fen.ts` (`validateBoardSetup`): Validates custom puzzle setups (both Generals present in their 3×3 palaces, no flying general, Advisors/Elephants/Soldiers on legal squares, piece counts within standard limits).

## Common Tasks

### Adding or modifying a piece rule
1. Update validation logic in `cotuong_core/src/logic/rules.rs`
2. Update move generation in `cotuong_core/src/logic/generator.rs` and `cotuong_core/src/engine/movegen.rs`
3. Update `src/wasm/enginePolyfill.ts` if the rule affects JavaScript polyfill move generation
4. Add unit tests and run `cargo test -p cotuong_core` and `npm test`
