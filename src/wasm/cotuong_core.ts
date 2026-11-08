import { PolyfillEngine, PolyfillGameState } from './enginePolyfill.js';

export const WasmEngine = PolyfillEngine;
export type WasmEngine = PolyfillEngine;

export const WasmGameState = PolyfillGameState;
export type WasmGameState = PolyfillGameState;

export function initSync(): boolean {
  return true;
}

export default async function __wbg_init(): Promise<boolean> {
  return true;
}
