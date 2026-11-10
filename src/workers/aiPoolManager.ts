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
  private sharedBuffer: SharedArrayBuffer | null = null;
  private currentWorkerTtMb: number = 256;

  constructor(initialThreads: number = 1, initialWorkerTtMb: number = 256) {
    this.currentWorkerTtMb = initialWorkerTtMb;
    const totalMb = initialWorkerTtMb * initialThreads;
    this.ensureSharedBuffer(totalMb);
    this.adjustPoolSize(initialThreads);
  }

  /**
   * Ensures the SharedArrayBuffer of the requested total size in MB exists.
   * If SharedArrayBuffer is unavailable (e.g. non-isolated context), returns null gracefully.
   */
  public ensureSharedBuffer(totalTtSizeMb: number): SharedArrayBuffer | null {
    if (typeof SharedArrayBuffer === 'undefined') {
      return null;
    }
    const safeMb = Math.max(16, Math.min(8192, totalTtSizeMb));
    const targetBytes = safeMb * 1024 * 1024;
    if (!this.sharedBuffer || this.sharedBuffer.byteLength !== targetBytes) {
      try {
        this.sharedBuffer = new SharedArrayBuffer(targetBytes);
      } catch (err) {
        console.warn('Failed to allocate SharedArrayBuffer:', err);
        this.sharedBuffer = null;
      }
    }
    return this.sharedBuffer;
  }

  /**
   * Dynamically resizes the worker pool to match the desired thread count
   */
  public adjustPoolSize(targetThreads: number, initConfigJson?: string): void {
    const desired = Math.max(1, Math.min(16, targetThreads));
    if (this.workers.length === desired) {
      return;
    }

    const totalMb = this.currentWorkerTtMb * desired;
    const sBuf = this.ensureSharedBuffer(totalMb);
    const perWorkerBytes = this.currentWorkerTtMb * 1024 * 1024;

    // Grow pool if needed
    while (this.workers.length < desired) {
      const threadId = this.workers.length;
      const worker = new Worker(new URL('./ai.worker.ts', import.meta.url), {
        type: 'module',
      });
      const configPayload = initConfigJson ?? JSON.stringify({ tt_size_mb: this.currentWorkerTtMb, threads: desired });
      worker.postMessage({
        type: 'RESET_CONFIG',
        configJson: configPayload,
        threadId,
        sharedBuffer: sBuf,
        bufferOffset: threadId * perWorkerBytes,
        bufferLength: perWorkerBytes,
        totalThreads: desired,
      });
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

    const totalThreads = Math.max(1, this.workers.length);
    const totalMb = this.currentWorkerTtMb * totalThreads;
    const sBuf = this.ensureSharedBuffer(totalMb);
    const perWorkerBytes = this.currentWorkerTtMb * 1024 * 1024;

    const worker = new Worker(new URL('./ai.worker.ts', import.meta.url), {
      type: 'module',
    });
    worker.postMessage({
      type: 'INIT',
      threadId,
      sharedBuffer: sBuf,
      bufferOffset: threadId * perWorkerBytes,
      bufferLength: perWorkerBytes,
      totalThreads,
      configJson: JSON.stringify({ tt_size_mb: this.currentWorkerTtMb, threads: totalThreads }),
    });
    this.workers[threadId] = worker;
  }

  /**
   * Broadcasts CLEAR_TT command to all active workers in the pool and zero-fills SharedArrayBuffer
   */
  public clearMemory(): void {
    if (this.sharedBuffer) {
      try {
        new Uint8Array(this.sharedBuffer).fill(0);
      } catch {
        // ignore
      }
    }
    for (const worker of this.workers) {
      worker.postMessage({ type: 'CLEAR_TT' });
    }
  }

  /**
   * Resets and re-allocates engine memory across all worker threads.
   * tt_size_mb is treated as the memory for each worker.
   * Total SharedArrayBuffer allocated = tt_size_mb * threads.
   */
  public resetConfig(config: Partial<EngineConfig>): void {
    const desiredThreads = config.threads ?? this.workers.length;
    const threadCount = Math.max(1, Math.min(16, desiredThreads));
    const workerTtMb = config.tt_size_mb ?? this.currentWorkerTtMb;
    this.currentWorkerTtMb = workerTtMb;

    const totalMb = workerTtMb * threadCount;
    const sBuf = this.ensureSharedBuffer(totalMb);
    const perWorkerBytes = workerTtMb * 1024 * 1024;
    const workerConfig = { ...config, tt_size_mb: workerTtMb, threads: threadCount };
    const configJson = JSON.stringify(workerConfig);

    const prevCount = this.workers.length;
    if (config.threads) {
      this.adjustPoolSize(config.threads, configJson);
    }

    // Post RESET_CONFIG with partitioned SharedArrayBuffer to all active workers
    for (let i = 0; i < Math.min(prevCount, this.workers.length); i++) {
      this.workers[i]!.postMessage({
        type: 'RESET_CONFIG',
        configJson,
        threadId: i,
        sharedBuffer: sBuf,
        bufferOffset: i * perWorkerBytes,
        bufferLength: perWorkerBytes,
        totalThreads: threadCount,
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

    // Update shared buffer and worker configuration based on per-worker TT size
    let configJson = req.configJson;
    if (configJson) {
      try {
        const parsed = JSON.parse(configJson);
        if (typeof parsed.tt_size_mb === 'number') {
          const workerTtMb = parsed.tt_size_mb;
          this.currentWorkerTtMb = workerTtMb;
          const totalMb = workerTtMb * threadCount;
          this.ensureSharedBuffer(totalMb);
          configJson = JSON.stringify({ ...parsed, tt_size_mb: workerTtMb, threads: threadCount });
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
      const totalThreads = Math.max(1, this.workers.length);
      const totalMb = this.currentWorkerTtMb * totalThreads;
      const sBuf = this.ensureSharedBuffer(totalMb);
      const perWorkerBytes = this.currentWorkerTtMb * 1024 * 1024;

      worker.postMessage({
        type: 'SEARCH',
        stateJson,
        fen: req.fen,
        timeLimitMs: req.timeLimitMs,
        maxDepth: req.maxDepth,
        configJson: req.configJson,
        threadId,
        allowedMovesFlat,
        sharedBuffer: sBuf,
        bufferOffset: threadId * perWorkerBytes,
        bufferLength: perWorkerBytes,
        totalThreads,
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
    this.sharedBuffer = null;
  }

  public get threadCount(): number {
    return this.workers.length;
  }
}
