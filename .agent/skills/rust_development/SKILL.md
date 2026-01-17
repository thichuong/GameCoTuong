---
name: rust_development
description: Senior-level guidelines for high-performance Rust development (Xiangqi Engine), covering optimization, memory safety, and testing.
---

# Development Requirements & Skills

## 1. Code Quality & Idiomatic Rust
- **Strict Linting**: Treat warnings as errors. Run `cargo clippy` frequently.
    - *Constraint*: Resolve complexity lints (e.g., `too_many_lines`) by architectural refactoring, not by ignoring logic.
- **Formatting**: Use `cargo fmt` to maintain standard styling.
- **Error Handling**: 
    - Avoid `.unwrap()` or `.expect()` in runtime logic (especially in `server` and `cotuong_core`). Use `Result` propagation or `Option` handling.
    - *Exception*: Tests and static initialization constants allowed.

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

## 4. Testing & Verification
- **Unit Tests**: Focus on logic verification for `Board` rules and `Evaluator` scoring.
- **Integration Tests**: Run `./test_all.sh` before submission.
- **Benchmarking**: When modifying `search.rs` or `eval.rs`, verify that nodes-per-second (NPS) has not degraded.

## 5. Context Optimization
- **Ignored Paths**: Do not read `dist/`, `client/dist/`, `*.lock`, or binary assets to save context tokens.