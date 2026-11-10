import test from 'node:test';
import assert from 'node:assert/strict';
import { DEFAULT_ENGINE_CONFIG } from './game.js';

test('DEFAULT_ENGINE_CONFIG matches cotuong_core Rust defaults', () => {
  assert.equal(DEFAULT_ENGINE_CONFIG.mate_score, 300000, 'Rust default mate_score is 300_000');
  assert.equal(DEFAULT_ENGINE_CONFIG.tt_size_mb, 512, 'Rust default tt_size_mb is 512 MB');
  assert.equal(DEFAULT_ENGINE_CONFIG.nnue_weight, 100, 'Rust default nnue_weight is 100%');
});

test('EngineConfig serializes cleanly to JSON for cotuong_core Rust deserializer', () => {
  const jsonStr = JSON.stringify(DEFAULT_ENGINE_CONFIG);
  const parsed = JSON.parse(jsonStr);

  assert.equal(parsed.mate_score, 300000);
  assert.equal(parsed.tt_size_mb, 512);
  assert.equal(parsed.nnue_weight, 100);

  // Ensure no phantom legacy fields
  assert.equal(parsed.pruning_method, undefined);
  assert.equal(parsed.pruning_multiplier, undefined);
  assert.equal(parsed.eval_mode, undefined);
  assert.equal(parsed.model_type, undefined);
});
