import init, { WasmGameState } from '../wasm/cotuong_core.js';
import wasmUrl from '../wasm/cotuong_core_bg.wasm?url';
import { PikafishNnueEngine } from '../engine/pikafishNnue.js';
import type { EngineConfig, SearchResult } from '../types/game.js';

let isWasmInitialized = false;
let engine: PikafishNnueEngine | null = null;
let currentConfigJson: string | null = null;

async function ensureInitialized(
  configJson?: string
): Promise<PikafishNnueEngine> {
  if (!isWasmInitialized) {
    await init(wasmUrl);
    isWasmInitialized = true;
  }

  if (!engine || (configJson && configJson !== currentConfigJson)) {
    const parsedConfig = configJson
      ? (JSON.parse(configJson) as EngineConfig)
      : null;
    engine = new PikafishNnueEngine(parsedConfig);
    currentConfigJson = configJson ?? null;
  }

  return engine;
}

self.onmessage = async (e: MessageEvent) => {
  const { type, fen, stateJson, timeLimitMs, maxDepth, configJson } = e.data;

  if (type === 'SEARCH') {
    try {
      const eng = await ensureInitialized(configJson);
      const state = stateJson
        ? WasmGameState.from_json(stateJson)
        : WasmGameState.from_fen(fen);

      const boardFen = state.to_fen();
      state.free();

      // Execute Pure NNUE v3 + Pikafish/Fairy-Stockfish search
      const result: SearchResult | null = eng.searchFen(
        boardFen,
        Number(timeLimitMs),
        maxDepth ?? null
      );

      if (result) {
        self.postMessage({ type: 'SEARCH_SUCCESS', result });
      } else {
        self.postMessage({ type: 'NO_MOVE' });
      }
    } catch (err) {
      console.error('Error during AI search in worker:', err);
      self.postMessage({ type: 'SEARCH_ERROR', error: String(err) });
    }
  }
};


