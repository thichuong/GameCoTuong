/* @ts-self-types="./cotuong_core.d.ts" */
import {
  WasmEngine,
  WasmGameState,
  getNnueWeights,
} from '../engine/pikafishNnue.ts';

export function initSync(_module) {
  getNnueWeights();
  return {};
}

export default async function __wbg_init(_module_or_path) {
  getNnueWeights();
  return {};
}

export { WasmEngine, WasmGameState };
