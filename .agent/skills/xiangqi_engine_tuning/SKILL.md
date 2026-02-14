---
name: xiangqi_engine_tuning
description: Guidelines for modifying the Xiangqi AI engine – search algorithms, evaluation tuning, and transposition table. Covers AlphaBetaEngine, SimpleEvaluator, and performance-critical hot paths.
---

# Xiangqi Engine Tuning Skill

## Scope

This skill covers modifications to files in `cotuong_core/src/engine/`:
- [search.rs](file:///home/exblackhole/Desktop/GameCoTuong/cotuong_core/src/engine/search.rs) – AlphaBetaEngine (Negamax + pruning)
- [eval.rs](file:///home/exblackhole/Desktop/GameCoTuong/cotuong_core/src/engine/eval.rs) – SimpleEvaluator
- [movegen.rs](file:///home/exblackhole/Desktop/GameCoTuong/cotuong_core/src/engine/movegen.rs) – EngineMoveGen (engine-specific move generation with scoring)
- [config.rs](file:///home/exblackhole/Desktop/GameCoTuong/cotuong_core/src/engine/config.rs) – EngineConfig (tunable parameters)
- [tt.rs](file:///home/exblackhole/Desktop/GameCoTuong/cotuong_core/src/engine/tt.rs) – TranspositionTable
- [move_list.rs](file:///home/exblackhole/Desktop/GameCoTuong/cotuong_core/src/engine/move_list.rs) – Stack-allocated move storage
- [zobrist.rs](file:///home/exblackhole/Desktop/GameCoTuong/cotuong_core/src/engine/zobrist.rs) – ZobristKeys (position hashing)

## Architecture Context

### Search Pipeline
1. `search()` → Iterative Deepening (depth 1 → `max_depth`)
2. `alpha_beta()` → Negamax with α-β pruning
   - TT probe → NMP → ProbCut → Singular Extension → LMR
3. `quiescence()` → Captures only (horizon effect prevention)
4. Move ordering: TT move → Captures (MVV-LVA) → Killers → History

### Engine Move Generation (`movegen.rs`)
- **`EngineMoveGen`**: Separate from `logic/generator.rs` `MoveGenerator`. Handles engine-specific move generation with scoring.
- Uses `AttackTables` for move generation + `EngineConfig` for scoring weights.
- **`MoveGenContext`**: Holds board state, move list, config, and history table reference during generation.
- Methods: `generate_moves()` (all moves with scoring), `generate_captures()` (captures only for quiescence).
- Piece-specific generators: `gen_rook_moves`, `gen_cannon_moves`, `gen_horse_moves`, `gen_elephant_moves`, `gen_advisor_moves`, `gen_king_moves`, `gen_pawn_moves`.
- `add_move()`: Assigns score based on TT move → MVV-LVA captures → Killer moves → History heuristic.
- `get_piece_value()`: Returns configurable piece values for MVV-LVA scoring.

### Key Data Structures
- `MoveList`: `[Move; 128]` on stack – **NEVER** use `Vec<Move>` in search loop
- `TranspositionTable`: Power-of-2 hash table with `TTEntry { key, best_move, score, depth, flag }`
- `AlphaBetaEngine` fields: `killer_moves`, `history_table`, `lmr_table`, `mate_scores`, `move_limits`
- All precomputed tables are initialized in `new()` — use `precompute_*` methods

### Evaluation Components
- Material counting (incremental via `Board.incremental_score`)
- Piece-Square Tables (PST) defined in `eval_constants.rs`
- Mobility (capped move counting for Rook, Horse, Cannon, Pawn)
- King safety (cannon mount penalty, exposed king)
- Structure bonus (connected advisors/elephants)

## Rules

### Performance (Critical)
1. **Zero allocation in hot paths**: `alpha_beta()`, `quiescence()`, `generate_moves_internal()` must NEVER allocate heap memory.
2. **Use `MoveList`** (stack-allocated `[Move; 128]`) instead of `Vec<Move>` for move generation inside the engine.
3. **Incremental updates**: Board hash and scores are updated incrementally. Never call `calculate_initial_hash()` or `calculate_initial_score()` during search.
4. **Check time sparingly**: `check_time()` uses `self.stats.nodes % 4096 == 0` to avoid syscall overhead.

### Move Ordering Tuning
- `add_move()` in `movegen.rs` assigns scores based on: TT move (hash_move score), MVV-LVA captures, killer moves, history heuristic
- History table: `[[i32; 90]; 2]` indexed by `[color][to_index]`
- Killer moves: 2 slots per ply `[[Option<Move>; 2]; 64]`
- `EngineMoveGen` uses `EngineConfig` values for all scoring thresholds

### Evaluation Tuning
- All weights are in `EngineConfig` and `eval_constants.rs`
- Piece values: Pawn=30, Advisor=120, Elephant=120, Horse=270, Cannon=285, Rook=600, King=6000
- PST tables: `[[i32; 9]; 10]` for each piece type. Red-centric, flipped for Black via `9 - row`.
- Config supports JSON loading via `load_from_json()` with scaling factors

### Testing & Verification
1. Run `cargo test -p cotuong_core` after any engine change
2. Check `bench_test.rs` for NPS (nodes-per-second) regression
3. Check `mate_test.rs` for checkmate detection correctness
4. Always verify: `cargo fmt && cargo check && cargo clippy`

## Common Tasks

### Adding a new pruning technique
1. Add method to `AlphaBetaEngine` (e.g., `fn futility_pruning(...)`)
2. Call it within `alpha_beta()` at the appropriate point in the search tree
3. Add config parameters to `EngineConfig` if tunable
4. Update `precompute_*` if precomputed data needed
5. Test with mate positions to ensure no search instability

### Modifying move ordering/scoring
1. Edit `add_move()` in `movegen.rs` to change scoring logic
2. Edit `generate_moves_internal()` for generation order changes
3. Adjust `EngineConfig` scoring constants if needed
4. Benchmark with `bench_test.rs` to verify NPS impact

### Tuning piece values
1. Modify constants in `eval_constants.rs` OR add config overrides in `EngineConfig`
2. The `evaluate()` function in `SimpleEvaluator` uses incremental scores from `Board` for material+PST
3. Non-incremental components (mobility, king safety) are computed fresh each call

### Adding a new evaluation term
1. Add weight constant to `eval_constants.rs`
2. Implement computation in `SimpleEvaluator::evaluate()`
3. Add config parameter to `EngineConfig` if tunable
4. Ensure the computation is efficient (avoid heap allocation, complex iteration)
