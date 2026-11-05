---
name: rust_development
description: Senior-level guidelines for high-performance Rust & WebAssembly development (Xiangqi Engine), covering optimization, memory safety, and testing.
---

# Development Requirements & Skills

## 1. Code Quality & Idiomatic Rust
- **Strict Linting**: Treat warnings as errors. Run `cargo clippy` for both native and `wasm32-unknown-unknown` targets.
  - *Constraint*: Resolve complexity lints (e.g., `too_many_lines`) by architectural refactoring, not by ignoring logic.
  - Workspace-level lints deny: `warnings`, `clippy::all`, `clippy::pedantic`, `clippy::nursery`, `clippy::unwrap_used`, `clippy::expect_used`, `clippy::indexing_slicing`.
- **Formatting**: Use `cargo fmt` to maintain standard styling.
- **Error Handling**:
  - Never use `.unwrap()` or `.expect()` in production runtime logic (`cotuong_core/src/`). Use `Result` propagation or `Option` handling.
  - *Exception*: Unit/integration tests (`#[cfg(test)]`, `tests/`) and static initialization constants.
  - Across the WASM boundary (`cotuong_core/src/wasm.rs`), convert errors into `JsValue` via `.map_err(|e| JsValue::from_str(...))`.
- **No unnecessary `.clone()` on `String`**: Use `&str` slices for function parameters.

## 2. Performance & Engine Optimization (Critical)
- **Zero-Cost Abstractions**: Prefer iterators and closures that compile down to optimized loops over manual indexing where possible, unless benchmarking proves otherwise.
- **Allocation-Free Hot Paths**:
  - **Forbidden**: Do NOT perform heap allocations (e.g., `Vec::new`, `String::clone`) inside `alpha_beta()`, `quiescence()`, `evaluate()`, or `EngineMoveGen`.
  - **Alternative**: Use stack-allocated `MoveList` (`[Move; 128]`) and precomputed lookup tables (`OnceLock`).
- **Inlining**: Use `#[inline]` for small, frequently called helper functions (e.g., bitboard operations, coordinate converters).
- **Data Types**: Use compact integers (`u8`, `i16`, `i32`, `u128` for bitboards) to minimize cache footprint.

## 3. Safety & Panic Freedom
- **Panic Freedom**: The engine and WASM bindings must never panic during gameplay or search. Use safe indexing (`.get()`, `.get_mut()`) or iterators to comply with `clippy::indexing_slicing = "deny"`.
- **Nesting Limit**: Avoid deep nesting (max 2–3 levels of `if/else`). Extract complex logic into helper functions.

## 4. Testing & Verification
- **Unit & Mate Tests**: Verify `Board` rules, repetition detection, `SimpleEvaluator` scoring, and checkmate solving (`cotuong_core/tests/mate_solving.rs`).
- **Benchmarking**: When modifying `search.rs`, `eval.rs`, or `movegen.rs`, run `cargo test --release -p cotuong_core -- engine::bench_test --nocapture` to verify that nodes-per-second (NPS) has not degraded.
- **Full Suite Verification**: Run `./test_all.sh` to execute:
  1. `cargo fmt --check`
  2. `cargo clippy --workspace --all-targets -- -D warnings`
  3. `cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings`
  4. `cargo test --workspace`
  5. `npm run build` (`tsc && vite build`)

## 5. Context Optimization
- **Ignored Paths**: Do not read `dist/`, `target/`, `cotuong_core/target_test/`, `node_modules/`, `*.lock`, or binary `.wasm` / `.mp3` assets to save context tokens.

## 6. Module Architecture Guidelines
- **Core Logic (`cotuong_core/src/logic/`)**: Pure Xiangqi rules, board state, FEN parsing, opening book, and legal move generation (`MoveGenerator`).
- **AI Engine (`cotuong_core/src/engine/`)**: Search (`AlphaBetaEngine`), evaluation (`SimpleEvaluator`), scored move generation (`EngineMoveGen`), transposition table (`tt.rs`), and Zobrist hashing (`zobrist.rs`).
- **WASM Bridge (`cotuong_core/src/wasm.rs`)**: `#[wasm_bindgen]` wrappers (`WasmGameState`, `WasmEngine`) compiled to `src/wasm/` via `npm run build:wasm`.
- **React Frontend (`src/`)**: Modular UI components (`src/components/`), Web Worker (`src/workers/ai.worker.ts`), utilities (`src/utils/`), and strong TypeScript interfaces (`src/types/game.ts`).