# Installation Guide for Co Tuong (Xiangqi)

This guide will help you set up and run the Co Tuong game, which has been rewritten using the Bevy game engine.

## Prerequisites

Before you begin, ensure you have the following installed:

1.  **Rust and Cargo**:
    - Install via rustup: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
    - Verify installation: `cargo --version`

2.  **System Dependencies (Linux)**:
    - Bevy requires certain system libraries to be installed. On Ubuntu/Debian, run:
      ```bash
      sudo apt-get update
      sudo apt-get install g++ pkg-config libx11-dev libasound2-dev libudev-dev libxkbcommon-dev
      ```
    - For other distributions, refer to the [Bevy Linux Dependencies](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md).

## Building and Running

1.  **Clone the Repository** (if you haven't already):
    ```bash
    git clone <repository-url>
    cd GameCoTuong
    ```

2.  **Run the Game**:
    - To run the game in development mode:
      ```bash
      cargo run
      ```
    - To run with optimizations (faster):
      ```bash
      cargo run --release
      ```

## Troubleshooting

-   **"Linking with `cc` failed"**: Ensure you have `g++` or `clang` installed.
-   **"Package `alsa` not found"**: Install `libasound2-dev`.
-   **"Package `libudev` not found"**: Install `libudev-dev`.
-   **WASM Build**:
    - To build for the web (WASM), you need the `wasm32-unknown-unknown` target:
      ```bash
      rustup target add wasm32-unknown-unknown
      cargo build --target wasm32-unknown-unknown
      ```
    - Note: Running Bevy in WASM requires `wasm-bindgen` and a web server.

## Configuration

The game configuration (e.g., engine settings) can be adjusted in the UI or by modifying `src/resources.rs` (for default values).
