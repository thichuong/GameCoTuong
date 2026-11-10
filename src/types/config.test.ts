import test from 'node:test';
import assert from 'node:assert/strict';
import { DEFAULT_ENGINE_CONFIG, calculatePerWorkerTtMb, calculateTotalTtMb } from './game.js';

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

test('calculatePerWorkerTtMb correctly partitions SharedArrayBuffer among workers', () => {
  // 512 MB total SharedArrayBuffer
  assert.equal(calculatePerWorkerTtMb(512, 1), 512, '1 thread gets full 512MB');
  assert.equal(calculatePerWorkerTtMb(512, 2), 256, '2 threads get 256MB each');
  assert.equal(calculatePerWorkerTtMb(512, 4), 128, '4 threads get 128MB each');
  assert.equal(calculatePerWorkerTtMb(512, 8), 64, '8 threads get 64MB each');

  // Enforces safe minimum bound of 16 MB
  assert.equal(calculatePerWorkerTtMb(64, 8), 16, '64MB across 8 threads maintains 16MB minimum');
  assert.equal(calculatePerWorkerTtMb(64, 16), 16, '64MB across 16 threads maintains 16MB minimum');

  // 2048 MB (2 GB)
  assert.equal(calculatePerWorkerTtMb(2048, 4), 512, '2GB across 4 threads gives 512MB each');
});

test('calculateTotalTtMb correctly multiplies worker TT size by thread count', () => {
  assert.equal(calculateTotalTtMb(256, 1), 256, '1 thread with 256MB = 256MB total');
  assert.equal(calculateTotalTtMb(256, 2), 512, '2 threads with 256MB = 512MB total');
  assert.equal(calculateTotalTtMb(512, 4), 2048, '4 threads with 512MB = 2048MB (2GB) total');
  assert.equal(calculateTotalTtMb(512, 8), 4096, '8 threads with 512MB = 4096MB (4GB) total');
});
