---
name: wasm_client_ui
description: Guidelines for modifying the WebAssembly/Polyfill bridge, Web Worker AI integration, and state synchronization between cotuong_core (Rust) and the React TypeScript frontend.
---

# WASM Bridge, Polyfill & Worker Integration Skill

> **Note**: For visual styling, Tailwind CSS v4 rules, 1:1 `BoardCanvas` rendering, and responsive `sides`/`stacked` layout guidelines, see the dedicated **`react_ui_design`** skill (`.agent/skills/react_ui_design/SKILL.md`).

## Scope

This skill covers the WebAssembly/Polyfill data bridge, background Web Worker, and state synchronization layer:
- `cotuong_core/src/wasm.rs` – `#[wasm_bindgen]` exports (`WasmGameState`, `WasmEngine`) and DTO serialization
- `src/wasm/cotuong_core.ts` & `src/wasm/enginePolyfill.ts` – WASM bridge entry and pure TypeScript fallback/polyfill implementation (`PolyfillGameState`, `PolyfillEngine`)
- `src/workers/ai.worker.ts` – Background Web Worker running `WasmEngine.search()` off the main thread
- `src/types/game.ts` & `src/types/config.test.ts` – Shared TypeScript DTO interfaces (`Piece`, `Square`, `GameMode`, `PuzzleStage`, `PuzzleSubMode`, `Difficulty`, `GameStatus`, `HistoryRecord`, `SearchStats`, `ModelType`, `EngineConfig`)
- `src/App.tsx` – Game state synchronization (`syncGameState`), opening book check, AI worker message lifecycle, and fallback move recovery
- `src/utils/fen.ts` & `src/utils/export.ts` – FEN serialization/validation and CSV/JSON data export utilities

## Architecture Context

### Web Worker Communication (`App.tsx` ↔ `ai.worker.ts`)
```
App (Main Thread)                                  ai.worker.ts (Worker Thread)
       |                                                         |
       |── 1. Check state.get_book_move() (standard game only)   |
       |      & verify with state.is_move_legal(...)             |
       |── 2. postMessage({ type: 'SEARCH', stateJson,           |
       |        fen, timeLimitMs, configJson }) ───────────────→ |
       |                                                         |── ensureInitialized(configJson)
       |                                                         |── WasmGameState.from_json(stateJson)
       |                                                         |   (or WasmGameState.from_fen(fen))
       |                                                         |── WasmEngine.search(state, timeLimitMs)
       |   ←── postMessage({ type: 'SEARCH_SUCCESS', result }) ──|
       |                                                         |
       |── 3. state.make_move(...) + annotate_last_move(...)     |
       |      (with get_strictly_legal_moves_flat() fallback)    |
```

### WASM ↔ Polyfill Parity (`wasm.rs` & `enginePolyfill.ts`)
Both `WasmGameState` (Rust WASM) and `PolyfillGameState` (TypeScript) expose the exact same API surface:
- **Constructors / Deserializers**: `new()`, `from_fen(fen)`, `from_json(json_str)`
- **Serializers**: `to_fen()`, `to_json()`
- **Queries**: `turn()`, `is_in_check()`, `status()`, `get_board()`, `get_legal_moves(row, col)`, `is_move_legal(from_r, from_c, to_r, to_c)`, `get_strictly_legal_moves_flat()`, `get_book_move()`, `get_history()`, `history_len()`
- **Mutations**: `make_move(from_r, from_c, to_r, to_c)`, `undo_move()`, `annotate_last_move(note)`

Both `WasmEngine` and `PolyfillEngine` expose:
- `new(config_json?)`
- `update_config(config_json)`
- `search(game_state, time_limit_ms, max_depth?)`

## Common Tasks

### Adding a new method to `WasmGameState` or `WasmEngine`
1. Add the `#[wasm_bindgen]` method in `cotuong_core/src/wasm.rs`
2. Add the matching method in `src/wasm/enginePolyfill.ts` (`PolyfillGameState` / `PolyfillEngine`) and add a unit test in `src/wasm/enginePolyfill.test.ts`
3. Update TypeScript interfaces in `src/types/game.ts` if new DTOs are returned
4. Rebuild WASM (`npm run build:wasm`) if Rust toolchain is active, and verify with `npm test && npm run lint && npm run build`

### Updating `EngineConfig` schema across the bridge
1. Update `EngineConfig` struct in `cotuong_core/src/engine/config.rs`
2. Update `EngineConfig` interface and `DEFAULT_ENGINE_CONFIG` in `src/types/game.ts`
3. Update validation/controls in `src/components/ConfigModal.tsx` and tests in `src/types/config.test.ts`
4. Run `npm test && npm run lint && npm run build`
