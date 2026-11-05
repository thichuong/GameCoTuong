---
name: xiangqi_engine_tuning
description: Guidelines for modifying the Xiangqi AI engine – search algorithms, evaluation tuning, and transposition table. Covers AlphaBetaEngine, SimpleEvaluator, and performance-critical hot paths.
---

# Xiangqi Engine Tuning Skill

## Scope

This skill covers modifications to files in `cotuong_core/src/engine/` and its configuration UI:
- `cotuong_core/src/engine/mod.rs` – Traits (`Evaluator`, `Searcher`) and structs (`Move`, `SearchLimit`, `SearchStats`)
- `cotuong_core/src/engine/search.rs` – `AlphaBetaEngine` (Iterative Deepening Negamax + pruning)
- `cotuong_core/src/engine/eval.rs` – `SimpleEvaluator` (material, PST, mobility, king safety, structure)
- `cotuong_core/src/engine/movegen.rs` – `EngineMoveGen` (engine-specific move generation with scoring)
- `cotuong_core/src/engine/config.rs` – `EngineConfig` (tunable parameters & JSON deserialization)
- `cotuong_core/src/engine/tt.rs` – `TranspositionTable` (Zobrist hash table)
- `cotuong_core/src/engine/move_list.rs` – `MoveList` (stack-allocated `[Move; 128]`)
- `cotuong_core/src/engine/zobrist.rs` – `ZobristKeys` (XorShift64 position hashing)
- `cotuong_core/src/wasm.rs` – `WasmEngine` wrapper exposing search and config updates to JS/WASM
- `src/components/ConfigModal.tsx` & `src/types/game.ts` – Frontend UI for tuning Red/Black AI configs

## Architecture Context

### Search Pipeline
1. `search()` → Iterative Deepening (depth 1 → `max_depth`, with Aspiration Windows)
2. `alpha_beta()` → Negamax with α-β pruning & PVS (Principal Variation Search)
   - TT probe → Razoring → Futility Pruning → Null-Move Pruning (NMP) → ProbCut → Singular Extension → Late Move Reductions (LMR) / Dynamic Limiting
3. `quiescence()` → Captures-only search to mitigate the horizon effect
4. Move ordering: TT move → Captures (MVV-LVA) → Killer moves → Countermove → History heuristic

### Engine Move Generation (`movegen.rs`)
- **`EngineMoveGen`**: Dedicated engine move generator with inline move scoring (separate from `logic/generator.rs`).
- Uses `AttackTables` for fast bitboard/lookup generation + `EngineConfig` for scoring weights.
- **`MoveGenContext`**: Holds board state, `MoveList`, config, killer moves, countermove, and history table reference during generation.
- Methods: `generate_moves()` (all pseudo-legal/legal moves with scores), `generate_captures()` (captures only for quiescence).
- Piece-specific generators: `gen_rook_moves`, `gen_cannon_moves`, `gen_horse_moves`, `gen_elephant_moves`, `gen_advisor_moves`, `gen_king_moves`, `gen_pawn_moves`.

### Key Data Structures
- `MoveList`: `[Move; 128]` on stack – **NEVER** use `Vec<Move>` in the search loop
- `TranspositionTable`: Power-of-2 hash table with `TTEntry { key, best_move, score, depth, flag }`
- `AlphaBetaEngine` fields: `killer_moves`, `history_table`, `countermove_table`, `lmr_table`, `mate_scores`, `move_limits`
- All precomputed tables are initialized in `new()` via `precompute_*` methods

### Evaluation Components (`eval.rs` & `eval_constants.rs`)
- Base Material & Piece-Square Tables (PST): Updated incrementally in `Board.incremental_score`
- Custom Piece Value Scaling: Applied dynamically when `EngineConfig` piece values differ from defaults
- Mobility: Capped move counting for Chariot (Rook), Horse, Cannon, and Soldier (Pawn)
- King Safety: Exposed general penalty, cannon threat with 0 or 1 screen (mount) along the general's file/rank
- Defensive Structure: Bonus for connected Advisors and Elephants
- Hanging Piece Penalty: Penalty for undefended attacked pieces

## Rules

### Performance (Critical)
1. **Zero allocation in hot paths**: `alpha_beta()`, `quiescence()`, and `EngineMoveGen` must NEVER allocate heap memory.
2. **Use `MoveList`** (stack-allocated `[Move; 128]`) instead of `Vec<Move>` inside the engine.
3. **Incremental updates**: Board Zobrist hash and material+PST scores are updated incrementally in `Board::apply_move()` and `Board::undo_move()`. Never recompute from scratch during search.
4. **Check time sparingly**: `check_time()` checks elapsed time every 4096 nodes (`self.stats.nodes & 4095 == 0`) to avoid timer call overhead.

### Testing & Verification
1. Run `cargo test -p cotuong_core` after any engine change (includes `mate_test` and `tests/mate_solving.rs`).
2. Run `cargo test --release -p cotuong_core -- engine::bench_test --nocapture` to check NPS (nodes-per-second).
3. Always verify both native and WASM targets:
   - `cargo fmt --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings`

## Common Tasks

### Adding a new tunable EngineConfig parameter
1. Add the field and default value to `EngineConfig` in `cotuong_core/src/engine/config.rs`
2. Wire the parameter into `AlphaBetaEngine` (`search.rs`), `SimpleEvaluator` (`eval.rs`), or `EngineMoveGen` (`movegen.rs`)
3. Add the field to `EngineConfig` and `DEFAULT_ENGINE_CONFIG` in `src/types/game.ts`
4. Add a `Slider` or selector control in `src/components/ConfigModal.tsx` if user-facing
5. Rebuild WASM (`npm run build:wasm`) and run `./test_all.sh`
