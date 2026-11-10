import init, { WasmEngine, WasmGameState } from '../wasm/cotuong_core.js';
import type { SearchResult } from '../types/game.js';
import { getOrFetchNnueModel } from '../utils/modelStorage.js';

let isWasmInitialized = false;
let engine: WasmEngine | null = null;
let currentConfigJson: string | null = null;
let currentThreadId: number = 0;
let modelLoadingPromise: Promise<Uint8Array | null> | null = null;
let messageQueue: Promise<void> = Promise.resolve();

async function ensureModelLoaded(eng: WasmEngine): Promise<void> {
  try {
    if (eng.is_nnue_active()) {
      return;
    }
  } catch (err) {
    console.warn('Error checking NNUE status:', err);
    return;
  }

  if (!modelLoadingPromise) {
    modelLoadingPromise = getOrFetchNnueModel();
  }

  try {
    const weights = await modelLoadingPromise;
    if (weights && weights.length > 0 && !eng.is_nnue_active()) {
      eng.load_weights_bytes(weights);
    }
  } catch (err) {
    console.warn('Could not load dynamic NNUE model weights:', err);
  }
}

async function ensureInitialized(configJson?: string, threadId: number = 0): Promise<WasmEngine> {
  if (!isWasmInitialized) {
    await init();
    isWasmInitialized = true;
  }

  if (!engine) {
    engine = new WasmEngine(configJson ?? null);
    currentConfigJson = configJson ?? null;
  } else if (configJson && configJson !== currentConfigJson) {
    try {
      engine.update_config(configJson);
      currentConfigJson = configJson;
    } catch {
      try {
        engine.clear_tt();
        engine.free();
      } catch {
        // ignore
      }
      engine = new WasmEngine(configJson);
      currentConfigJson = configJson;
    }
  }

  if (currentThreadId !== threadId) {
    engine.set_thread_id(threadId);
    currentThreadId = threadId;
  }

  // Pre-load NNUE neural weights from IndexedDB/Network
  await ensureModelLoaded(engine);

  return engine;
}

async function handleMessage(e: MessageEvent): Promise<void> {
  const {
    type,
    stateJson,
    fen,
    timeLimitMs,
    maxDepth,
    configJson,
    threadId = 0,
    allowedMovesFlat,
  } = e.data;

  if (type === 'CLEAR_TT') {
    if (engine) {
      engine.clear_tt();
    }
    return;
  }

  if (type === 'RESET_CONFIG') {
    try {
      if (engine && configJson) {
        try {
          engine.update_config(configJson);
          engine.clear_tt();
          currentConfigJson = configJson;
        } catch {
          try {
            engine.free();
          } catch {
            // ignore
          }
          engine = null;
          currentConfigJson = null;
          engine = await ensureInitialized(configJson, threadId);
        }
      } else if (!engine) {
        engine = await ensureInitialized(configJson, threadId);
      }
      self.postMessage({ type: 'RESET_CONFIG_SUCCESS', threadId });
    } catch (err) {
      console.error(`Failed to reset engine memory in worker #${threadId}:`, err);
      self.postMessage({ type: 'RESET_CONFIG_ERROR', error: String(err), threadId });
    }
    return;
  }

  if (type === 'INIT') {
    try {
      await ensureInitialized(configJson, threadId);
      self.postMessage({ type: 'INIT_SUCCESS', threadId });
    } catch (err) {
      self.postMessage({ type: 'INIT_ERROR', error: String(err), threadId });
    }
    return;
  }

  if (type === 'SEARCH') {
    let state: WasmGameState | null = null;
    try {
      const eng = await ensureInitialized(configJson, threadId);
      state = stateJson
        ? WasmGameState.from_json(stateJson)
        : WasmGameState.from_fen(fen);

      // Execute search natively via Rust WebAssembly engine
      // Supports root move partitioning when allowedMovesFlat is specified
      const rawResult = (
        allowedMovesFlat && allowedMovesFlat.length > 0
          ? eng.search_with_partition(
              state,
              BigInt(timeLimitMs),
              maxDepth ?? undefined,
              allowedMovesFlat
            )
          : eng.search(
              state,
              BigInt(timeLimitMs),
              maxDepth ?? undefined
            )
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
          `AI thread #${threadId} search candidate move was not strictly legal, picking fallback legal move.`
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

      if (result) {
        self.postMessage({ type: 'SEARCH_SUCCESS', result, threadId });
      } else {
        self.postMessage({ type: 'NO_MOVE', threadId });
      }
    } catch (err) {
      console.error(`Error during AI search in worker thread #${threadId}:`, err);
      // Clean up poisoned engine instance on any runtime trap/error
      if (engine) {
        try {
          engine.free();
        } catch {
          // ignore
        }
        engine = null;
      }
      currentConfigJson = null;
      self.postMessage({ type: 'SEARCH_ERROR', error: String(err), threadId });
    } finally {
      if (state) {
        try {
          state.free();
        } catch {
          // ignore
        }
      }
    }
  }
}

self.onmessage = (e: MessageEvent) => {
  messageQueue = messageQueue
    .then(() => handleMessage(e))
    .catch((err) => {
      console.error('Unhandled error in worker queue:', err);
    });
};
