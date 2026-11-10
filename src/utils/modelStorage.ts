// IndexedDB caching and streaming manager for NNUE model weights
// Ensures the 4.1MB neural model is downloaded only once and cached locally for instant startup.

const DB_NAME = 'xiangqi_model_storage';
const DB_VERSION = 1;
const STORE_NAME = 'models';
const MODEL_KEY = 'default_nnue_v3';
const MODEL_URL = '/assets/default.nnue';

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    if (typeof indexedDB === 'undefined') {
      return reject(new Error('IndexedDB not supported in this environment'));
    }

    const request = indexedDB.open(DB_NAME, DB_VERSION);

    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME);
      }
    };

    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

async function getCachedModel(db: IDBDatabase): Promise<Uint8Array | null> {
  return new Promise((resolve) => {
    try {
      const tx = db.transaction(STORE_NAME, 'readonly');
      const store = tx.objectStore(STORE_NAME);
      const req = store.get(MODEL_KEY);

      req.onsuccess = () => {
        if (req.result instanceof ArrayBuffer) {
          resolve(new Uint8Array(req.result));
        } else if (req.result instanceof Uint8Array) {
          resolve(req.result);
        } else {
          resolve(null);
        }
      };

      req.onerror = () => resolve(null);
    } catch {
      resolve(null);
    }
  });
}

async function storeModelInCache(db: IDBDatabase, data: ArrayBuffer): Promise<void> {
  return new Promise((resolve) => {
    try {
      const tx = db.transaction(STORE_NAME, 'readwrite');
      const store = tx.objectStore(STORE_NAME);
      store.put(data, MODEL_KEY);
      tx.oncomplete = () => resolve();
      tx.onerror = () => resolve();
    } catch {
      resolve();
    }
  });
}

/**
 * Loads the NNUE model weights from local IndexedDB cache or fetches from `/assets/default.nnue`.
 * Returns null if fetching fails (allowing graceful fallback to evaluation constants).
 */
export async function getOrFetchNnueModel(): Promise<Uint8Array | null> {
  let db: IDBDatabase | null = null;
  try {
    db = await openDb();
    const cached = await getCachedModel(db);
    if (cached && cached.length > 0) {
      return cached;
    }
  } catch (err) {
    console.warn('IndexedDB unavailable for NNUE cache, fetching directly:', err);
  }

  try {
    const response = await fetch(MODEL_URL);
    if (!response.ok) {
      console.warn(`Failed to fetch NNUE model: HTTP ${response.status}`);
      return null;
    }

    const buffer = await response.arrayBuffer();
    if (buffer.byteLength === 0) {
      return null;
    }

    // Cache the downloaded model asynchronously
    if (db) {
      void storeModelInCache(db, buffer);
    }

    return new Uint8Array(buffer);
  } catch (err) {
    console.warn('Network error while downloading NNUE model:', err);
    return null;
  }
}
