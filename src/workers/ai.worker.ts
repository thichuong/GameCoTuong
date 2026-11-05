import init, { WasmEngine, WasmGameState } from '../wasm/cotuong_core.js';
import wasmUrl from '../wasm/cotuong_core_bg.wasm?url';
import type { SearchResult } from '../types/game.js';

let isWasmInitialized = false;
let engine: WasmEngine | null = null;
let currentConfigJson: string | null = null;

async function ensureInitialized(configJson?: string): Promise<WasmEngine> {
  if (!isWasmInitialized) {
    await init(wasmUrl);
    isWasmInitialized = true;
  }

  if (!engine || (configJson && configJson !== currentConfigJson)) {
    engine = new WasmEngine(configJson ?? null);
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

      // Execute search
      const result: SearchResult | null = eng.search(
        state,
        BigInt(timeLimitMs),
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

