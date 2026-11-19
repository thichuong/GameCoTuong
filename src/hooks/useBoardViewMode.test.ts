import test from 'node:test';
import assert from 'node:assert/strict';
import {
  parseBoardViewMode,
  loadStoredBoardViewMode,
} from './useBoardViewMode.js';

test('parseBoardViewMode accepts valid modes and defaults invalid inputs to 2.5d', () => {
  assert.equal(parseBoardViewMode('2d'), '2d');
  assert.equal(parseBoardViewMode('2.5d'), '2.5d');
  assert.equal(parseBoardViewMode('3d'), '3d');

  assert.equal(parseBoardViewMode(null), '2.5d');
  assert.equal(parseBoardViewMode(undefined), '2.5d');
  assert.equal(parseBoardViewMode(''), '2.5d');
  assert.equal(parseBoardViewMode('4d'), '2.5d');
  assert.equal(parseBoardViewMode('2D'), '2.5d');
});

test('loadStoredBoardViewMode falls back cleanly to 2.5d in non-browser environment', () => {
  assert.equal(loadStoredBoardViewMode(), '2.5d');
});
