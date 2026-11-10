import init, { WasmEngine, WasmGameState } from '../wasm/cotuong_core.js';
import type { SearchResult } from '../types/game.js';
import { getOrFetchNnueModel } from '../utils/modelStorage.js';

let isWasmInitialized = false;
let engine: WasmEngine | null = null;
let currentConfigJson: string | null = null;
let modelLoadingPromise: Promise<Uint8Array | null> | null = null;

async function ensureModelLoaded(eng: WasmEngine): Promise<void> {
  if (eng.is_nnue_active()) {
    return;
  }

  if (!modelLoadingPromise) {
    modelLoadingPromise = getOrFetchNnueModel();
  }

  try {
    const weights = await modelLoadingPromise;
    if (weights && weights.length > 0) {
      eng.load_weights_bytes(weights);
    }
  } catch (err) {
    console.warn('Could not load dynamic NNUE model weights:', err);
  }
}

async function ensureInitialized(configJson?: string): Promise<WasmEngine> {
  if (!isWasmInitialized) {
    await init();
    isWasmInitialized = true;
  }

  if (!engine || (configJson && configJson !== currentConfigJson)) {
    engine = new WasmEngine(configJson ?? null);
    currentConfigJson = configJson ?? null;
  }

  // Pre-load NNUE neural weights from IndexedDB/Network
  await ensureModelLoaded(engine);

  return engine;
}

self.onmessage = async (e: MessageEvent) => {
  const { type, stateJson, fen, timeLimitMs, maxDepth, configJson } = e.data;

  if (type === 'CLEAR_TT') {
    if (engine) {
      engine.clear_tt();
    }
    return;
  }

  if (type === 'SEARCH') {
    try {
      const eng = await ensureInitialized(configJson);
      const state = stateJson
        ? WasmGameState.from_json(stateJson)
        : WasmGameState.from_fen(fen);

      // Execute search natively via Rust WebAssembly engine
      // Full game state history is preserved for three-fold repetition detection
      const rawResult = eng.search(
        state,
        BigInt(timeLimitMs),
        maxDepth ?? undefined
      ) as SearchResult | null;

      const result = rawResult ?? null;

      // Safety guard: ensure returned move is strictly legal and never causes ThreeFoldRepetition
      if (
        result &&
        !state.is_move_legal(
          result.move.from_row,
          result.move.from_col,
          result.move.to_row,
          result.move.to_col
        )
      ) {
        console.warn(
          'AI search candidate move was not strictly legal, picking fallback legal move.'
        );
        const flatMoves = state.get_strictly_legal_moves_flat() as number[];
        if (flatMoves.length >= 4) {
          result.move = {
            from_row: flatMoves[0]!,
            from_col: flatMoves[1]!,
            to_row: flatMoves[2]!,
            to_col: flatMoves[3]!,
          };
        }
      }

      state.free();

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
