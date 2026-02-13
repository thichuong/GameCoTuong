---
name: wasm_client_ui
description: Guidelines for modifying the Leptos WASM client – UI components, canvas rendering, game modes, Web Worker integration, and WebSocket networking.
---

# WASM Client & UI Skill

## Scope

This skill covers:
- [client/src/app.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/app.rs) – Main App component + sub-components (1529 lines)
- [client/src/main.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/main.rs) – Entry point
- [client/src/network.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/network.rs) – NetworkClient (WebSocket)
- [client/src/components/board.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/components/board.rs) – BoardView (Canvas rendering)
- [client/src/bin/worker.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/bin/worker.rs) – Web Worker entry
- [cotuong_core/src/worker.rs](file:///home/exblackhole/Desktop/GameCoTuong/cotuong_core/src/worker.rs) – GameWorker (gloo-worker)

## Architecture Context

### Technology Stack
- **Leptos 0.6** (CSR mode) – reactive UI framework for Rust/WASM
- **Trunk** – WASM build tool + dev server
- **gloo-worker** – Web Worker abstraction for background AI computation
- **web_sys** – Rust bindings for Web APIs (Canvas, WebSocket, DOM)
- Compile target: `wasm32-unknown-unknown`

### Component Structure (app.rs)
```
App() → Main component
├── Signals: game_state, game_mode, player_side, difficulty, is_thinking, ...
├── Effects: AI move trigger, online message handling
├── BoardView → Canvas component
├── ControlsArea → Game mode selection, play controls
├── LogPanel → Move history display
├── ThinkingIndicator → AI computing indicator
├── OnlineStatusPanel → Online game status & controls
├── ConfigPanel → Engine configuration sliders
├── Slider / FloatSlider / Dropdown → Reusable input components
├── handle_file_upload() → Import EngineConfig from JSON
├── export_config() → Export EngineConfig to JSON
└── export_csv() → Export move history to CSV
```

### Game Modes
| Mode | Flow |
|---|---|
| `HumanVsComputer` | Player clicks → move applied → AI Worker triggered → AI move applied |
| `ComputerVsComputer` | AI Worker triggered for each side alternately |
| `HumanVsHuman` | Two players click alternately (local hotseat) |
| `Online` | Player connects via WebSocket → matchmaking → moves exchanged via server |

### Web Worker Communication
```
App (main thread)                     GameWorker (worker thread)
    |                                       |
    |── Input::ComputeMove(state,...) ──→   |
    |                                       |── AlphaBetaEngine::search()
    |   ←── Output::MoveFound(mv, stats) ──|
    |                                       |
```
- Worker is spawned via `gloo_worker::Spawnable::spawner()`
- Bridge callback updates reactive signals on main thread

### Canvas Rendering (BoardView)
- Uses `HtmlCanvasElement` + `CanvasRenderingContext2d`
- Renders: grid lines, river, palace diagonals, piece circles, text
- Interaction: Click events → coordinate calculation → move selection
- Highlights: selected piece, legal moves, last move indicator

## Rules

### Leptos Patterns
1. Use `create_signal()` for reactive state
2. Use `create_effect()` for side effects (AI triggers, message handling)
3. Components receive `ReadSignal`/`WriteSignal` as props
4. HTML rendered via `view! { ... }` macro
5. Event handlers use closures that capture signals

### WASM Considerations
1. No `std::time::Instant` – use `web_sys::Performance::now()` for timing
2. File I/O uses `FileReader` API, not `std::fs`
3. Networking uses `web_sys::WebSocket`, not `tokio`
4. Random numbers need `getrandom` with `js` feature
5. Console logging via `console_log` crate

### Web Worker
1. `GameWorker` in `cotuong_core/src/worker.rs` implements `gloo_worker::Worker`
2. Worker entry point: `client/src/bin/worker.rs`
3. Worker receives full `GameState` + `EngineConfig` for each computation
4. Engine instance persisted in worker (`Option<AlphaBetaEngine>`)
5. Config updates reuse existing engine via `update_config()`

### Network Client
1. `NetworkClient` wraps `web_sys::WebSocket`
2. Connects to `ws://localhost:3000/ws`
3. Message callback uses `Closure::forget()` — intentional leak for lifetime management
4. Outgoing messages serialized via `serde_json::to_string()`
5. Incoming messages deserialized and set on `WriteSignal<Option<ServerMessage>>`

## Common Tasks

### Adding a new UI component
1. Define function component in `app.rs`: `fn MyComponent(props...) -> impl IntoView { view! { ... } }`
2. Create signals for component state
3. Wire into parent via `ReadSignal`/`WriteSignal` props
4. Use CSS classes for styling (defined in `client/index.html` or separate CSS file)

### Adding a new game mode
1. Add variant to `GameMode` enum in `app.rs`
2. Add UI controls in `ControlsArea`
3. Implement mode-specific logic in `App()` effects
4. Handle turn transitions and AI triggers appropriately

### Modifying board rendering
1. Edit `BoardView` in `client/src/components/board.rs`
2. Canvas operations: `ctx.begin_path()`, `ctx.arc()`, `ctx.fill_text()`, etc.
3. Coordinate mapping: canvas pixels ↔ board coordinates
4. Ensure click hit-testing matches visual positions

### Building & running
```bash
# Install dependencies
rustup target add wasm32-unknown-unknown
cargo install trunk

# Dev server (hot reload)
cd client && trunk serve

# Production build
cd client && trunk build --release
```
