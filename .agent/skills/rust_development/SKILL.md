---
name: rust_development
description: Senior-level guidelines for Rust development, covering code quality, testing, and context optimization.
---

# Development Requirements & Skills

## Code Quality Assurance
- **Idiomatic Rust**: Write code that strictly follows Rust best practices and idioms.
- **Linting**: Run `cargo clippy` after every feature implementation.
    - *Action*: Resolve specific lints architecturally (e.g., split functions for `too_many_lines`, use structs for `too_many_arguments`).
- **Formatting**: Always execute `cargo fmt` to standardize code style.

## Testing & Verification
- **Test Suite**: Run `./test_all.sh` to ensure all tests pass before considering a task complete.

## Context Optimization
- **Ignored Paths**: Do not read files in `dist/` or `client/dist/` to save context tokens.
