---
name: game_logic_rules
description: Guidelines for modifying Xiangqi game rules, board representation, move generation, and validation. Covers Board, GameState, MoveGenerator, and rule enforcement.
---

# Game Logic & Rules Skill

## Scope

This skill covers modifications to files in `cotuong_core/src/logic/`:
- [board.rs](file:///home/exblackhole/Desktop/GameCoTuong/cotuong_core/src/logic/board.rs) – Board, Piece, Color, PieceType, Bitboard
- [game.rs](file:///home/exblackhole/Desktop/GameCoTuong/cotuong_core/src/logic/game.rs) – GameState, turn management, move history, undo
- [generator.rs](file:///home/exblackhole/Desktop/GameCoTuong/cotuong_core/src/logic/generator.rs) – MoveGenerator (legal move generation)
- [rules.rs](file:///home/exblackhole/Desktop/GameCoTuong/cotuong_core/src/logic/rules.rs) – Move validation, check, flying general
- [lookup.rs](file:///home/exblackhole/Desktop/GameCoTuong/cotuong_core/src/logic/lookup.rs) – AttackTables (precomputed moves)
- [eval_constants.rs](file:///home/exblackhole/Desktop/GameCoTuong/cotuong_core/src/logic/eval_constants.rs) – Piece values, PST, weights
- [opening.rs](file:///home/exblackhole/Desktop/GameCoTuong/cotuong_core/src/logic/opening.rs) – Opening book (FEN-based)

## Architecture Context

### Board Representation
- **Grid**: 10 rows × 9 cols = 90 squares
- **Squares**: `[Option<Piece>; 90]` – direct square access
- **Bitboards**: `piece_bb: [[Bitboard; 7]; 2]` (per piece type, per color) + `color_bb: [Bitboard; 2]`
- **Bitboard type**: `u128` – bit index = `row * 9 + col`
- **Coordinate**: `BoardCoordinate { row: usize, col: usize }` – bounds-checked via `new()`, unchecked via `new_unchecked()`
- **Hash**: Zobrist `u64` – updated incrementally on `apply_move()` / `undo_move()`
- **Score**: `incremental_score: [i32; 2]` – updated incrementally

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
  - Index: piece position on rank/file (0..9)
  - Occupancy: 10-bit mask of all pieces on same rank/file
  - Returns: 10-bit attack mask
- **Non-sliding pieces**: Precomputed target arrays for each of 90 squares
  - Horse: `[(target_sq, leg_blocker_sq)]`
  - Elephant: `[(target_sq, eye_blocker_sq)]`
  - Advisor/General/Soldier: `[target_sq]`

### Xiangqi-Specific Rules
| Rule | Implementation |
|---|---|
| Palace restriction | General + Advisors confined to 3×3 palace (rows 0-2 or 7-9, cols 3-5) |
| River restriction | Elephants cannot cross river (rows 0-4 for Red, 5-9 for Black) |
| Horse leg blocking | Horse blocked if piece at intermediate square |
| Elephant eye blocking | Elephant blocked if piece at diagonal intermediate square |
| Cannon jumping | Cannon captures by jumping exactly 1 piece; moves on empty paths |
| Flying General | Two generals cannot face each other on same column with no pieces between |
| Self-check | A move cannot leave own general in check |

## Rules

### Board Modification
1. **Always update all representations**: When modifying board state, update `squares[]`, `piece_bb`, `color_bb`, `hash`, and `incremental_score` consistently.
2. Use `Board::apply_move()` and `Board::undo_move()` — they handle all bookkeeping.
3. For setup/testing, use `Board::add_piece()` / `Board::remove_piece()` / `Board::set_piece()`.

### Move Generation
1. `MoveGenerator` uses `AttackTables::get()` for lookup.
2. Generated moves are checked with `is_valid_move()` which includes self-check validation.
3. `has_legal_moves()` is optimized for early-return (used in mate/stalemate detection).
4. `can_piece_make_any_legal_move()` checks if a specific piece has any legal move.
5. **Note**: `logic/generator.rs` (`MoveGenerator`) is for game logic. `engine/movegen.rs` (`EngineMoveGen`) is the engine-specific variant with move scoring — see `xiangqi_engine_tuning` skill.

### GameState Management
1. `GameState` holds: `board`, `turn`, `status`, `move_history`, `position_history` (for repetition), `move_generator`.
2. `make_move()` validates, applies, records history, checks repetition, updates status.
3. `undo_move()` restores previous state from history.
4. `GameStatus`: `Playing`, `Checkmate(Color)`, `Stalemate`.

### FEN Support
- `Board::to_fen_string(turn)` → standard Xiangqi FEN string
- `Board::from_fen(fen)` → `Result<(Board, Color), String>`
- FEN format: piece placement (rows separated by `/`) + space + turn char (`w`/`b`)

## Common Tasks

### Adding a new piece rule
1. Add validation logic in `rules.rs` (e.g., `validate_new_piece()`)
2. Update `validate_piece_logic()` match arm
3. Add move generation in `generator.rs` (both `generate_*_moves()` and `check_*_moves()`)
4. If needed, add precomputed data in `lookup.rs`
5. Update `EngineMoveGen` in `engine/movegen.rs` if engine-specific generation is affected
6. Test with specific board positions

### Modifying board representation
1. Update `Board` struct in `board.rs`
2. Ensure `apply_move()`, `undo_move()`, `add_piece()`, `remove_piece()` all updated
3. Update `calculate_initial_hash()` and `calculate_initial_score()` if needed
4. Verify FEN export/import still works
5. Run full test suite

### Adding a new game end condition
1. Add variant to `GameStatus` enum in `game.rs`
2. Update `GameState::update_status()` detection logic
3. Update server `game_manager/move_handler.rs` / `game_manager/lifecycle.rs` to handle new status
4. Update client `app/game_app.rs` to display new condition
5. Update client `app/online.rs` if it affects online mode
