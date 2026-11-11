---
name: xiangqi_engine_tuning
description: Guidelines for modifying the Xiangqi AI engine and NNUE trainer – search algorithms, Dual-Head HalfKP NNUE v3 evaluation, standard SIMD model format, transposition table, and cotuong_trainer pipeline.
---

# Xiangqi Engine Tuning & NNUE Skill

## Scope

This skill covers modifications to files in `cotuong_core/src/engine/`, `cotuong_trainer/`, and the configuration UI:
- `cotuong_core/src/engine/mod.rs` – Traits (`Evaluator`, `Searcher`) and structs (`Move`, `SearchLimit`, `SearchStats`)
- `cotuong_core/src/engine/search.rs` – `AlphaBetaEngine` (Iterative Deepening Negamax + PVS + pruning + Cooperative Lazy SMP multi-threading)
- `cotuong_core/src/engine/eval.rs` – `SimpleEvaluator` & `EvalCache` (evaluates board via `NnueWeights` and scales by `nnue_weight`)
- `cotuong_core/src/engine/nnue.rs` – `NnueWeights`, `NnueAccumulator`, Dual-Head HalfKP v3 architecture (standard integer SIMD & batched forward pass)
- `cotuong_core/src/engine/movegen.rs` – `EngineMoveGen` (engine-specific move generation with scoring)
- `cotuong_core/src/engine/config.rs` – `EngineConfig` (`mate_score`, `tt_size_mb`, `nnue_weight`, `threads`)
- `cotuong_core/src/engine/tt.rs` – `TranspositionTable` (lockless atomic Zobrist hash table `AtomicTTEntry` with configurable per-worker MB size)
- `cotuong_core/src/engine/move_list.rs` – `MoveList` (stack-allocated `[Move; 128]`)
- `cotuong_core/src/engine/zobrist.rs` – `ZobristKeys` (XorShift64 position hashing)
- `cotuong_trainer/src/` – `main.rs`, `dataset.rs`, `protocol.rs` (NNUE training, UCCI/UCI teacher distillation, sparring, and quantization)
- `src/components/ConfigModal.tsx` & `src/types/game.ts` – Frontend UI for tuning Red/Black `EngineConfig` (with draft state committed on "Áp Dụng & Đóng")

## Architecture Context

### Search Pipeline & Lazy SMP (`cotuong_core/src/engine/search.rs`)
1. `search()` → Iterative Deepening (depth 1 → `max_depth`, with Aspiration Windows and Lazy SMP thread role specialization by `thread_id`: Master, Depth Skew, Root Move Jitter, Wide Window)
2. `alpha_beta()` → Negamax with α-β pruning & PVS (Principal Variation Search)
   - Three-fold repetition detection → Lockless Atomic TT probe → Razoring / Reverse Futility Pruning → Null-Move Pruning (NMP) → Late Move Reductions (LMR) → Check Extensions
3. `quiescence()` → Captures and check-evasion search to mitigate the horizon effect
4. Move ordering: TT move → Captures (MVV-LVA) → Killer moves (2 slots/ply) → Countermove & History heuristic

### Dual-Head HalfKP NNUE v3 (`cotuong_core/src/engine/nnue.rs`)
- **Feature Set (`L1_INPUT_SIZE = 10,530`)**: `9 king palace squares × 13 piece types (6 friendly non-king + 7 enemy) × 90 board squares`.
- **Incremental Accumulator (`NnueAccumulator` on `Board`)**:
  - `red: [i16; 192]`, `black: [i16; 192]`
  - `psqt_red: [i32; 8]`, `psqt_black: [i32; 8]` (8 game-phase material & positional buckets)
  - Updated incrementally in $O(1)$ on `Board::apply_move()` and `Board::undo_move()`.
- **Deep SCReLU Positional Head**:
  - `L1 (2×192 = 384)` → SCReLU activation (`clamp(0, 256)^2 >> 8`) with sparse zero-skipping over column-major `w2_col` → `L2 (32)` → `L3 (32)` → `Output (1)`.
  - Evaluates both Red and Black perspectives simultaneously in `evaluate_dual_forward()`.
- **Standard Quantized NNUE (`cotuong_core/assets/default.nnue` & `public/assets/default.nnue`)**:
  - `default.nnue` (magic `XNNU` v3, ~4.0 MB): Standard SIMD quantized network (W1 is `i16`, L2/L3 are `i8`, L4 is `i16`, pure integer arithmetic).
  - Embedded in native builds (`embed-model` feature) and decoupled in WASM builds (`--no-default-features`, ~335 KB `.wasm`) where weights are cached in IndexedDB (`src/utils/modelStorage.ts`) and loaded dynamically via `WasmEngine::load_weights_bytes()`.

### EngineConfig Parameters (`cotuong_core/src/engine/config.rs`)
| Parameter | Type | Default | Description |
|---|---|---|---|
| `mate_score` | `i32` | `300_000` | Base checkmate score (`mate_score - ply`) |
| `tt_size_mb` | `usize` | `512` | Per-worker Transposition Table size in megabytes (`16..=1024` in UI) |
| `nnue_weight` | `i32` | `100` | Percentage scaling factor (`50..=150`) applied to raw NNUE evaluation |
| `threads` | `usize` | `1` (Rust) / `getDefaultThreads()` (UI) | Number of parallel search threads (`1..=16`) in Web Worker pool |

## Rules

### Performance (Critical)
1. **Zero allocation in hot paths**: `alpha_beta()`, `quiescence()`, `evaluate_dual_forward()`, and `EngineMoveGen` must NEVER allocate heap memory.
2. **Use `MoveList`** (stack-allocated `[Move; 128]`) instead of `Vec<Move>` inside the engine.
3. **Incremental updates**: Board Zobrist hash and `NnueAccumulator` (`red`, `black`, `psqt_red`, `psqt_black`) are updated incrementally in `Board::apply_move()` and `Board::undo_move()`. Never recompute from scratch during search unless a King moves.

### Testing & Verification
1. Run `cargo test --workspace` after any engine or trainer change.
2. Run `cargo test --release -p cotuong_core -- engine::bench_test --nocapture` to check NPS (nodes-per-second).
3. Always verify both native and WASM targets plus TypeScript tests:
   - `cargo fmt --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings`
   - `npm test`

## Common Tasks

### Adding a new tunable EngineConfig parameter
1. Add the field and default value to `EngineConfig` in `cotuong_core/src/engine/config.rs`
2. Wire the parameter into `AlphaBetaEngine` (`search.rs`), `SimpleEvaluator` (`eval.rs`), or `EngineMoveGen` (`movegen.rs`)
3. Add the field to `EngineConfig` and `DEFAULT_ENGINE_CONFIG` in `src/types/game.ts` and update `src/types/config.test.ts`
4. Add UI controls in `src/components/ConfigModal.tsx`
5. Rebuild WASM (`npm run build:wasm`) and run `./test_all.sh`
