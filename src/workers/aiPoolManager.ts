import type { SearchResult, EngineConfig } from '../types/game.js';
import type { WasmGameState } from '../wasm/cotuong_core.js';

export interface SearchRequest {
  state: WasmGameState;
  stateJson?: string;
  fen?: string;
  timeLimitMs: number;
  maxDepth?: number;
  configJson?: string;
  threads: number;
}

export class AiPoolManager {
  private workers: Worker[] = [];

  constructor(initialThreads: number = 1) {
    this.adjustPoolSize(initialThreads);
  }

  /**
   * Dynamically resizes the worker pool to match the desired thread count
   */
  public adjustPoolSize(targetThreads: number, initConfigJson?: string): void {
    const desired = Math.max(1, Math.min(16, targetThreads));
    if (this.workers.length === desired) {
      return;
    }

    // Grow pool if needed
    while (this.workers.length < desired) {
      const threadId = this.workers.length;
      const worker = new Worker(new URL('./ai.worker.ts', import.meta.url), {
        type: 'module',
      });
      if (initConfigJson) {
        worker.postMessage({ type: 'RESET_CONFIG', configJson: initConfigJson, threadId });
      } else {
        worker.postMessage({ type: 'INIT', threadId });
      }
      this.workers.push(worker);
    }

    // Shrink pool if thread count was decreased
    while (this.workers.length > desired) {
      const worker = this.workers.pop();
      worker?.terminate();
    }
  }

  /**
   * Respawns a crashed or poisoned worker thread to keep the pool healthy
   */
  public respawnWorker(threadId: number): void {
    if (threadId < 0 || threadId >= this.workers.length) {
      return;
    }
    const oldWorker = this.workers[threadId];
    try {
      oldWorker?.terminate();
    } catch {
      // ignore
    }

    const worker = new Worker(new URL('./ai.worker.ts', import.meta.url), {
      type: 'module',
    });
    worker.postMessage({ type: 'INIT', threadId });
    this.workers[threadId] = worker;
  }

  /**
   * Broadcasts CLEAR_TT command to all active workers in the pool
   */
  public clearMemory(): void {
    for (const worker of this.workers) {
      worker.postMessage({ type: 'CLEAR_TT' });
    }
  }

  /**
   * Resets and re-allocates engine memory across all worker threads.
   * This ensures previous Transposition Tables are freed and new TT memory size is allocated immediately.
   */
  public resetConfig(config: Partial<EngineConfig>): void {
    const desiredThreads = config.threads ?? this.workers.length;
    const threadCount = Math.max(1, Math.min(16, desiredThreads));
    const totalTt = config.tt_size_mb ?? 512;
    const perWorkerTt = Math.max(16, Math.floor(totalTt / threadCount));
    const workerConfig = { ...config, tt_size_mb: perWorkerTt };
    const configJson = JSON.stringify(workerConfig);

    const prevCount = this.workers.length;
    if (config.threads) {
      this.adjustPoolSize(config.threads, configJson);
    }

    // Only post RESET_CONFIG to workers that were already running before adjustPoolSize
    for (let i = 0; i < Math.min(prevCount, this.workers.length); i++) {
      this.workers[i]!.postMessage({
        type: 'RESET_CONFIG',
        configJson,
        threadId: i,
      });
    }
  }

  /**
   * Executes multi-threaded search across the worker pool
   */
  public async search(req: SearchRequest): Promise<SearchResult | null> {
    const threadCount = Math.max(1, Math.min(16, req.threads));
    this.adjustPoolSize(threadCount);

    if (this.workers.length === 0) {
      return null;
    }

    // Distribute total TT size proportionally across threads to prevent memory exhaustion in browsers
    let configJson = req.configJson;
    if (configJson) {
      try {
        const parsed = JSON.parse(configJson);
        if (typeof parsed.tt_size_mb === 'number') {
          const perWorkerTt = Math.max(16, Math.floor(parsed.tt_size_mb / threadCount));
          configJson = JSON.stringify({ ...parsed, tt_size_mb: perWorkerTt });
        }
      } catch {
        // ignore JSON parse error
      }
    }
    const effectiveReq = { ...req, configJson };

    // Single thread execution shortcut
    if (this.workers.length === 1) {
      return this.searchSingle(this.workers[0]!, effectiveReq, 0);
    }

    // Multi-threaded search with Cooperative Lazy SMP across all threads
    return this.searchParallel(effectiveReq);
  }

  private searchSingle(
    worker: Worker,
    req: SearchRequest,
    threadId: number,
    allowedMovesFlat?: number[]
  ): Promise<SearchResult | null> {
    return new Promise((resolve, reject) => {
      const onMessage = (e: MessageEvent) => {
        if (e.data.type === 'SEARCH_SUCCESS') {
          cleanup();
          resolve(e.data.result as SearchResult);
        } else if (e.data.type === 'NO_MOVE') {
          cleanup();
          resolve(null);
        } else if (e.data.type === 'SEARCH_ERROR') {
          cleanup();
          this.respawnWorker(threadId);
          reject(new Error(e.data.error || 'Search error in worker'));
        }
      };

      const onError = (err: ErrorEvent) => {
        cleanup();
        this.respawnWorker(threadId);
        reject(err);
      };

      const cleanup = () => {
        worker.removeEventListener('message', onMessage);
        worker.removeEventListener('error', onError);
      };

      worker.addEventListener('message', onMessage);
      worker.addEventListener('error', onError);

      const stateJson = req.stateJson || req.state.to_json();
      worker.postMessage({
        type: 'SEARCH',
        stateJson,
        fen: req.fen,
        timeLimitMs: req.timeLimitMs,
        maxDepth: req.maxDepth,
        configJson: req.configJson,
        threadId,
        allowedMovesFlat,
      });
    });
  }

  /**
   * Cooperative Multi-Search Lazy SMP:
   * All threads search concurrently with specialized roles per threadId:
   * - Thread 0: Master (standard aspiration search)
   * - Thread 1: Depth Skew (helper exploring deep branches)
   * - Thread 2: Alternative Root Move Jitter (helper testing secondary refutations)
   * - Thread 3: Wide Window Searcher (helper exploring wider score swings)
   * This guarantees 100% CPU utilization across all threads and avoids NPS drop-off late game.
   */
  private async searchParallel(req: SearchRequest): Promise<SearchResult | null> {
    const numWorkers = this.workers.length;
    const flatMoves = req.state.get_strictly_legal_moves_flat() as number[];
    const totalMoves = Math.floor(flatMoves.length / 4);

    // If only 1 or no legal move exists, let worker 0 handle it directly
    if (totalMoves <= 1) {
      return this.searchSingle(this.workers[0]!, req, 0);
    }

    const stateJson = req.stateJson || req.state.to_json();
    const searchPromises: Promise<SearchResult | null>[] = [];

    for (let i = 0; i < numWorkers; i++) {
      const worker = this.workers[i]!;
      searchPromises.push(
        this.searchSingle(worker, { ...req, stateJson }, i).catch((err) => {
          console.warn(`Parallel worker #${i} failed:`, err);
          return null;
        })
      );
    }

    const results = await Promise.all(searchPromises);
    const validResults = results.filter((r): r is SearchResult => r !== null);

    if (validResults.length === 0) {
      return null;
    }

    // Aggregate statistics from all parallel workers
    let totalNodes = 0;
    let maxTimeMs = 0;
    let maxDepth = 0;
    // Default candidate to worker 0 (Master)
    let bestResult: SearchResult = validResults[0]!;

    for (const r of validResults) {
      totalNodes += r.stats.nodes;
      if (r.stats.time_ms > maxTimeMs) {
        maxTimeMs = r.stats.time_ms;
      }
      if (r.stats.depth > maxDepth) {
        maxDepth = r.stats.depth;
      }

      // Pick move with higher score; if scores equal, prefer deeper search
      const currentBestScore = bestResult.stats.score ?? -999_999;
      const rScore = r.stats.score ?? -999_999;

      if (
        rScore > currentBestScore ||
        (rScore === currentBestScore && r.stats.depth > bestResult.stats.depth)
      ) {
        bestResult = r;
      }
    }

    return {
      move: bestResult.move,
      stats: {
        depth: maxDepth,
        nodes: totalNodes,
        time_ms: maxTimeMs,
        score: bestResult.stats.score,
      },
    };
  }

  /**
   * Terminate all worker processes when disposing
   */
  public terminate(): void {
    for (const worker of this.workers) {
      worker.terminate();
    }
    this.workers = [];
  }

  public get threadCount(): number {
    return this.workers.length;
  }
}
