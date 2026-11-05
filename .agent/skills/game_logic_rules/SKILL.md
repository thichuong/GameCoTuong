---
name: game_logic_rules
description: Guidelines for modifying Xiangqi game rules, board representation, move generation, and validation. Covers Board, GameState, MoveGenerator, and rule enforcement.
---

# Game Logic & Rules Skill

## Scope

This skill covers modifications to files in `cotuong_core/src/logic/` and its WASM wrapper:
- `cotuong_core/src/logic/board.rs` – `Board`, `Piece`, `Color`, `PieceType`, `BoardCoordinate`, `Bitboard` (`u128`)
- `cotuong_core/src/logic/game.rs` – `GameState`, `GameStatus`, `MoveRecord`, turn management, move history, undo, repetition detection
- `cotuong_core/src/logic/generator.rs` – `MoveGenerator` (legal move generation using precomputed lookup tables)
- `cotuong_core/src/logic/rules.rs` – Move validation, check detection (`is_in_check`), flying general rule
- `cotuong_core/src/logic/lookup.rs` – `AttackTables` (precomputed sliding and non-sliding attack masks)
- `cotuong_core/src/logic/eval_constants.rs` – Piece values, Piece-Square Tables (PST), evaluation weights
- `cotuong_core/src/logic/opening.rs` – Opening book (`get_book_move`, FEN-based)
- `cotuong_core/src/wasm.rs` – `WasmGameState` bindings exposing game logic to the React/TypeScript frontend

## Architecture Context

### Board Representation
- **Grid**: 10 rows × 9 cols = 90 squares
- **Squares**: `[Option<Piece>; 90]` – direct square access
- **Bitboards**: `piece_bb: [[Bitboard; 7]; 2]` (per piece type, per color) + `color_bb: [Bitboard; 2]`
- **Bitboard type**: `u128` – bit index = `row * 9 + col`
- **Coordinate**: `BoardCoordinate { row: usize, col: usize }` – bounds-checked via `new()`, unchecked via `new_unchecked()`
- **Hash**: Zobrist `u64` – updated incrementally on `apply_move()` / `undo_move()`
- **Score**: `incremental_score: [i32; 2]` – updated incrementally for Red and Black

### Move Validation Flow
```
is_valid_move(board, from, to, turn)
├── Check: from has piece of correct color
├── Check: to is not occupied by friendly piece
├── validate_piece_logic(board, from, to, turn)
│   ├── validate_general / validate_advisor / validate_elephant / ...
│   └── Each validates movement pattern + path blocking
├── Simulate move on board
├── Check: own king not in check after move (self-check)
└── Check: no flying general
```

### Attack Tables (Precomputed)
- Initialized once via `OnceLock` singleton (`AttackTables::get()`)
- **Sliding pieces** (Rook/Cannon): Occupancy-indexed tables `[[u16; 1024]; 10]`
  - Index: piece position on rank/file (`0..9` or `0..10`)
  - Occupancy: bitmask of all pieces on the same rank/file
  - Returns: attack bitmask
- **Non-sliding pieces**: Precomputed target arrays for each of 90 squares
  - Horse: `[(target_sq, leg_blocker_sq)]`
  - Elephant: `[(target_sq, eye_blocker_sq)]`
  - Advisor/General/Soldier: `[target_sq]`

### Xiangqi-Specific Rules
| Rule | Implementation |
|---|---|
| Palace restriction | General + Advisors confined to 3×3 palace (rows 0–2 or 7–9, cols 3–5) |
| River restriction | Elephants cannot cross river (rows 0–4 for Black, 5–9 for Red) |
| Horse leg blocking | Horse blocked if piece at orthogonal adjacent square |
| Elephant eye blocking | Elephant blocked if piece at diagonal midpoint square |
| Cannon jumping | Cannon captures by jumping over exactly 1 piece (screen); moves along empty lines |
| Flying General | Two generals cannot face each other on the same file with no pieces between |
| Self-check | A move cannot leave own general in check |
| Repetition rule | Perpetual check or perpetual chase is forbidden / handled via position history |

## Rules

### Board Modification
1. **Always update all representations**: When modifying board state, update `squares[]`, `piece_bb`, `color_bb`, `hash`, and `incremental_score` consistently.
2. Use `Board::apply_move()` and `Board::undo_move()` — they handle all incremental bookkeeping.
3. For setup/testing, use `Board::add_piece()` / `Board::remove_piece()` / `Board::set_piece()`.

### Move Generation
1. `MoveGenerator` uses `AttackTables::get()` for fast lookup.
2. Generated moves are validated against self-check and flying general constraints.
3. `has_legal_moves()` is optimized for early-return (used in checkmate/stalemate detection).
4. `can_piece_make_any_legal_move()` checks if a specific piece has any legal move.
5. **Note**: `logic/generator.rs` (`MoveGenerator`) is used for game state & UI legal move highlighting. `engine/movegen.rs` (`EngineMoveGen`) is the engine-specific generator with move ordering scores — see `xiangqi_engine_tuning` skill.

### GameState Management
1. `GameState` holds: `board`, `turn`, `status`, `history` (`Vec<MoveRecord>`), `position_history` (for repetition detection).
2. `make_move()` validates, applies, records move and hash history, checks repetition, and updates `GameStatus`.
3. `undo_move()` restores the previous state from `history`.
4. `GameStatus`: `Playing`, `Checkmate(Color)`, `Stalemate`.

### FEN & JSON Support
- `Board::to_fen_string(turn)` → standard Xiangqi FEN string
- `Board::from_fen(fen)` → `Result<(Board, Color), String>`
- `WasmGameState::to_json()` / `WasmGameState::from_json()` → serializes full `GameState` (including `history` and `position_history`) so the AI Web Worker has full repetition context.

## Common Tasks

### Adding or modifying a piece rule
1. Update validation logic in `cotuong_core/src/logic/rules.rs`
2. Update `validate_piece_logic()` match arm
3. Update move generation in `cotuong_core/src/logic/generator.rs`
4. If needed, update precomputed tables in `cotuong_core/src/logic/lookup.rs`
5. Update `EngineMoveGen` in `cotuong_core/src/engine/movegen.rs` if engine move generation is affected
6. Add unit tests and run `cargo test -p cotuong_core`

### Adding a new game end condition
1. Add variant to `GameStatus` enum in `cotuong_core/src/logic/game.rs`
2. Update `GameState` status detection logic in `game.rs`
3. Update `WasmGameState::status()` serialization in `cotuong_core/src/wasm.rs`
4. Rebuild WASM bindings (`npm run build:wasm`)
5. Update `GameStatus` interface in `src/types/game.ts` and end-of-game overlay in `src/App.tsx`
