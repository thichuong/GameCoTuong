---
name: wasm_client_ui
description: Guidelines for modifying the React + TypeScript frontend and wasm-bindgen bridge – UI components, 1:1 responsive Canvas rendering, game modes, Web Worker AI integration, and PWA assets.
---

# WASM Client & React UI Skill

## Scope

This skill covers the WebAssembly bindings and the React 18 + TypeScript + Tailwind CSS v4 frontend:
- `cotuong_core/src/wasm.rs` – `#[wasm_bindgen]` exports (`WasmGameState`, `WasmEngine`)
- `src/wasm/` – Generated `wasm-bindgen` JS/TS glue and `.wasm` binary (`npm run build:wasm`)
- `src/main.tsx` – React application entry point
- `src/App.tsx` – Main `App` component orchestrating WASM state, game modes, Web Worker, and overlays
- `src/types/game.ts` – Shared TypeScript types (`Piece`, `Square`, `GameMode`, `Difficulty`, `GameStatus`, `HistoryRecord`, `SearchStats`, `EngineConfig`)
- `src/workers/ai.worker.ts` – Background Web Worker running `WasmEngine.search()`
- `src/components/BoardCanvas.tsx` – Responsive 1:1 square HTML5 Canvas board renderer with perspective flipping
- `src/components/ControlsArea.tsx` – Mode, player side, difficulty selectors, and action toolbar
- `src/components/ThinkingIndicator.tsx` – Board-centered AI thinking HUD overlay with live timer and search telemetry
- `src/components/LogPanel.tsx` – Collapsible bottom move log dock and slide-up drawer sheet
- `src/components/ConfigModal.tsx` – Modal for tuning Red and Black `EngineConfig` parameters + JSON import/export
- `src/utils/export.ts` – CSV move history export and JSON config export utilities
- `src/utils/sound.ts` – `SoundManager` for move, capture, check, and checkmate audio effects

## Architecture Context

### Technology Stack
- **React 18 + TypeScript 5** – Declarative UI with strict typing
- **Vite 5 + Tailwind CSS v4** – Fast bundler and utility-first dark lacquerware styling
- **wasm-bindgen + serde-wasm-bindgen** – Zero-boilerplate Rust ↔ JavaScript interop
- **Web Worker (`ai.worker.ts`)** – Runs `WasmEngine` in a dedicated background thread so the UI never freezes
- **HTML5 Canvas (`CanvasRenderingContext2D`)** – High-DPI (`devicePixelRatio`) traditional Xiangqi board rendering

### Component Hierarchy
```
App (src/App.tsx)
├── Header (Title, Check Warning Badge, Active Turn Indicator)
├── ControlsArea (Mode, Side, Difficulty, Play/Pause CvC, New Game, Undo, Log Toggle, CSV Export, AI Config)
├── BoardCanvas (1:1 aspect ratio, auto-sized via Math.min(window.innerWidth, window.innerHeight))
│   ├── ThinkingIndicator (Overlay HUD shown on the board while AI computes)
│   └── Checkmate / Stalemate Victory Overlay
├── LogPanel (Collapsed bottom summary dock + expandable slide-up sheet)
└── ConfigModal (Per-side Red/Black AI parameter sliders & JSON import/export)
```

### Game Modes & Difficulty Levels
| Mode | Description |
|---|---|
| `HumanVsComputer` | Player vs AI (player can choose Red / Đi tiên or Black / Đi hậu) |
| `ComputerVsComputer` | AI Red vs AI Black with independent `EngineConfig` and Play/Pause control |
| `HumanVsHuman` | Local 2-player hotseat on the same device |

| Difficulty | Label | AI Time Limit |
|---|---|---|
| `1` | Nhập môn | `1000 ms` (1s) |
| `2` | Cơ bản | `2000 ms` (2s) |
| `3` | Trung cấp | `5000 ms` (5s) |
| `4` | Cao thủ | `10000 ms` (10s) |
| `5` | Đại sư | `20000 ms` (20s) |

### Web Worker Communication (`App.tsx` ↔ `ai.worker.ts`)
```
App (Main Thread)                                  ai.worker.ts (Worker Thread)
      |                                                         |
      |── 1. Check state.get_book_move() (instant opening)      |
      |── 2. postMessage({ type: 'SEARCH', stateJson,           |
      |        fen, timeLimitMs, configJson }) ───────────────→ |
      |                                                         |── WasmGameState.from_json(stateJson)
      |                                                         |── WasmEngine.search(state, timeLimitMs)
      |   ←── postMessage({ type: 'SEARCH_SUCCESS', result }) ──|
      |                                                         |
```

### Canvas Rendering (`BoardCanvas.tsx`)
- **Strict 1:1 Square Sizing**: Dynamically calculates `Math.max(200, Math.min(window.innerWidth, window.innerHeight))` on window resize and scales by `window.devicePixelRatio` for crisp rendering.
- **Coordinate Flipping**: Automatically flips board coordinates when `playerSide === 'black'` so the player's pieces are always at the bottom.
- **Visual Elements**: Rosewood/Hinoki board gradient, double outer border, 10×9 grid lines, Chu River / Han Border (`楚 河` / `漢 界`), palace diagonals, fiducial L-markers, carved wooden piece discs, dashed origin & glowing destination markers for the last move, jade legal-move dots, and vermilion capture rings.

## Common Tasks

### Rebuilding WASM bindings after modifying `cotuong_core`
```bash
npm run build:wasm
```
This compiles `cotuong_core` for `wasm32-unknown-unknown` in release mode and runs `wasm-bindgen --target web --out-dir src/wasm`.

### Adding a new method to `WasmGameState` or `WasmEngine`
1. Add the `#[wasm_bindgen]` method in `cotuong_core/src/wasm.rs`
2. Run `npm run build:wasm` to regenerate `src/wasm/cotuong_core.js` and `src/wasm/cotuong_core.d.ts`
3. Update TypeScript interfaces in `src/types/game.ts` if new DTOs are returned
4. Call the method from `src/App.tsx` or `src/workers/ai.worker.ts`

### Modifying UI components
1. Keep components modular under `src/components/`
2. Maintain the dark wood/lacquerware palette (`#14110F`, `#1C1815`, `#F5EFE6`, amber/vermilion accents)
3. Verify TypeScript compilation and bundle build with `npm run lint && npm run build`
