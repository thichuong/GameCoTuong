---
name: wasm_client_ui
description: Guidelines for modifying the Leptos WASM client – UI components, canvas rendering, game modes, Web Worker integration, and WebSocket networking.
---

# WASM Client & UI Skill

## Scope

This skill covers the modularized client application:
- [client/src/main.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/main.rs) – Entry point
- [client/src/app/mod.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/app/mod.rs) – Shared enums: `Difficulty`, `GameMode`, `OnlineStatus`
- [client/src/app/game_app.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/app/game_app.rs) – Main `App` component (~444 lines)
- [client/src/app/controls.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/app/controls.rs) – `ControlsArea` (mode/side/difficulty/actions)
- [client/src/app/config.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/app/config.rs) – `ConfigPanel`, `Slider`, `FloatSlider`, `Dropdown`
- [client/src/app/export.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/app/export.rs) – `handle_file_upload`, `export_config`, `export_csv`
- [client/src/app/log.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/app/log.rs) – `LogPanel`, `ThinkingIndicator`
- [client/src/app/online.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/app/online.rs) – `OnlineStatusPanel`
- [client/src/app/styles.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/app/styles.rs) – `GAME_STYLES` CSS constants
- [client/src/network.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/network.rs) – `NetworkClient` (WebSocket)
- [client/src/components/board.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/components/board.rs) – `BoardView` (Canvas rendering)
- [client/src/bin/worker.rs](file:///home/exblackhole/Desktop/GameCoTuong/client/src/bin/worker.rs) – Web Worker entry
- [cotuong_core/src/worker.rs](file:///home/exblackhole/Desktop/GameCoTuong/cotuong_core/src/worker.rs) – `GameWorker` (gloo-worker)

## Architecture Context

### Technology Stack
- **Leptos 0.6** (CSR mode) – reactive UI framework for Rust/WASM
- **Trunk** – WASM build tool + dev server
- **gloo-worker** – Web Worker abstraction for background AI computation
- **web_sys** – Rust bindings for Web APIs (Canvas, WebSocket, DOM, Audio)
- Compile target: `wasm32-unknown-unknown`

### Module Layout
```
client/src/app/
├── mod.rs        # Shared enums: Difficulty (5 levels), GameMode (4 modes), OnlineStatus (6 states)
├── game_app.rs   # Main App component – orchestrates signals, effects, game logic
├── controls.rs   # ControlsArea – mode selector, side selector, difficulty, play/undo/CSV buttons
├── config.rs     # ConfigPanel – per-side engine config UI with Slider/FloatSlider/Dropdown
├── export.rs     # File import/export: JSON config upload, JSON config download, CSV export
├── log.rs        # LogPanel (move history) + ThinkingIndicator
├── online.rs     # OnlineStatusPanel – online matchmaking/game UI
└── styles.rs     # GAME_STYLES – embedded CSS string constants
```

### Component Hierarchy
```
App() → Main component (game_app.rs)
├── Signals: game_state, game_mode, player_side, difficulty, is_thinking, ...
├── Effects: AI move trigger, online message handling
├── BoardView → Canvas component (components/board.rs)
├── ControlsArea → Game controls (controls.rs)
├── LogPanel → Move history display (log.rs)
├── ThinkingIndicator → AI computing indicator (log.rs)
├── OnlineStatusPanel → Online game status & controls (online.rs)
└── ConfigPanel → Engine configuration sliders (config.rs)
    ├── Slider → Integer parameter input
    ├── FloatSlider → Float parameter input
    └── Dropdown → Enum selection input
```

### Shared Enums (app/mod.rs)
```rust
enum Difficulty { Level1, Level2, Level3, Level4, Level5 }  // 1s, 2s, 5s, 10s, 20s
enum GameMode { HumanVsComputer, ComputerVsComputer, HumanVsHuman, Online }
enum OnlineStatus { None, Finding, MatchFound, Playing, OpponentDisconnected, GameEnded }
```

### Game Modes
| Mode | Flow |
|---|---|
| `HumanVsComputer` | Player clicks → move applied → AI Worker triggered → AI move applied |
| `ComputerVsComputer` | AI Worker triggered for each side alternately (Pause/Resume) |
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

### Module Organization
1. **New UI components** go in their own file under `client/src/app/` (not inline in `game_app.rs`)
2. **Shared enums** (used by both `game_app.rs` and sub-components) go in `app/mod.rs`
3. **Export/import logic** (non-UI) goes in `export.rs`
4. **Styles** are defined as CSS string constants in `styles.rs`
5. **Online-specific UI** goes in `online.rs`

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
1. Create a new file in `client/src/app/` (e.g., `my_component.rs`)
2. Define function component: `#[component] pub fn MyComponent(props...) -> impl IntoView { view! { ... } }`
3. Add `pub mod my_component;` in `app/mod.rs`
4. Wire into `App()` in `game_app.rs` via `ReadSignal`/`WriteSignal` props
5. Add CSS styles in `styles.rs` if needed

### Adding a new game mode
1. Add variant to `GameMode` enum in `app/mod.rs`
2. Add UI controls in `ControlsArea` (`controls.rs`)
3. Implement mode-specific logic in `App()` effects (`game_app.rs`)
4. Handle turn transitions and AI triggers appropriately

### Modifying online mode behavior
1. Edit `OnlineStatusPanel` in `online.rs` for UI changes
2. Edit `App()` in `game_app.rs` for message handling effect changes
3. Update `OnlineStatus` enum in `app/mod.rs` if adding new states
4. Coordinate with server protocol changes in `shared/src/lib.rs`

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
