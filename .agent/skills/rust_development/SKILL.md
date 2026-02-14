---
name: rust_development
description: Senior-level guidelines for high-performance Rust development (Xiangqi Engine), covering optimization, memory safety, and testing.
---

# Development Requirements & Skills

## 1. Code Quality & Idiomatic Rust
- **Strict Linting**: Treat warnings as errors. Run `cargo clippy` frequently.
    - *Constraint*: Resolve complexity lints (e.g., `too_many_lines`) by architectural refactoring, not by ignoring logic.
    - Workspace-level lints deny: `warnings`, `clippy::all`, `clippy::pedantic`, `clippy::nursery`, `clippy::unwrap_used`, `clippy::expect_used`, `clippy::indexing_slicing`.
- **Formatting**: Use `cargo fmt` to maintain standard styling.
- **Error Handling**:
    - Avoid `.unwrap()` or `.expect()` in runtime logic (especially in `server` and `cotuong_core`). Use `Result` propagation or `Option` handling.
    - *Exception*: Tests and static initialization constants allowed.
    - Use `if let` / `match` / `?` operator for error handling. Prefer `if let` for single-case pattern matching.
- **No unnecessary `.clone()` on `String`**: Use `&str` slices for function parameters. Use `String::with_capacity()` when building strings in loops.

## 2. Performance & Engine Optimization (Critical)
- **Zero-Cost Abstractions**: Prefer iterators and closures that compile down to optimized loops over manual indexing where possible, unless benchmarking proves otherwise.
- **Allocation-Free Hot Paths**:
    - **Forbidden**: Do NOT perform heap allocations (e.g., `Vec::new`, `String::clone`) inside the `search` loop or `evaluate` function.
    - **Alternative**: Use pre-allocated buffers, `Box<[T]>` created at startup, or stack-based arrays (e.g., `[Option<Move>; 64]`).
- **Inlining**: Use `#[inline]` for small, frequently called helper functions (e.g., bitboard operations, coordinate converters).
- **Data Types**: Use generic integers (`u8`, `i16`) appropriate for the domain to save memory bandwidth. Use `usize` for array indexing.

## 3. Safety & Unsafe Code
- **Unsafe Guidelines**:
    - Only use `unsafe` for proven performance bottlenecks (e.g., array access without bounds checking in `MoveGen`).
    - *Requirement*: Every `unsafe` block MUST have a `// SAFETY:` comment explaining why the invariant holds.
- **Panic Freedom**: The engine must never panic during a search. Verify array bounds explicitly if not using iterators.
- **Nesting Limit**: Avoid deep nesting (max 2-3 levels of `if/else`). Extract complex logic into helper functions.

## 4. Testing & Verification
- **Unit Tests**: Focus on logic verification for `Board` rules and `Evaluator` scoring.
- **Integration Tests**: Run `./test_all.sh` before submission.
- **Benchmarking**: When modifying `search.rs`, `eval.rs`, or `movegen.rs`, verify that nodes-per-second (NPS) has not degraded.
- **Mandatory Checks**: After any code change, always run:
  1. `cargo fmt` – auto-format
  2. `cargo check --workspace` – compilation check
  3. `cargo clippy --workspace` – lint check

## 5. Context Optimization
- **Ignored Paths**: Do not read `dist/`, `client/dist/`, `*.lock`, or binary assets to save context tokens.

## 6. Module Architecture Guidelines
- **Server modules**: `game_manager/` split by responsibility: `lifecycle.rs`, `matchmaking.rs`, `move_handler.rs`, `session.rs`.
- **Client modules**: `app/` split by UI concern: `game_app.rs`, `controls.rs`, `config.rs`, `export.rs`, `log.rs`, `online.rs`, `styles.rs`.
- **Engine modules**: Separate logic (`logic/generator.rs`) from engine-specific (`engine/movegen.rs`) move generation.
- When adding new functionality, place it in the appropriate existing module or create a new focused module rather than expanding existing large files.