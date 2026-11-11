import test from 'node:test';
import assert from 'node:assert/strict';
import {
  BOARD_SIZE,
  PADDING,
  CELL_WIDTH,
  CELL_HEIGHT,
  getPieceSymbol,
  getVisualCoords,
  getLogicCoords,
} from './boardCoords.js';

test('getPieceSymbol returns traditional Chinese characters for Red and Black pieces', () => {
  assert.equal(getPieceSymbol('General', 'red'), '帥');
  assert.equal(getPieceSymbol('General', 'black'), '將');
  assert.equal(getPieceSymbol('Horse', 'red'), '傌');
  assert.equal(getPieceSymbol('Horse', 'black'), '馬');
  assert.equal(getPieceSymbol('Soldier', 'red'), '兵');
  assert.equal(getPieceSymbol('Soldier', 'black'), '卒');
});

test('getVisualCoords and getLogicCoords are inverses for Red perspective', () => {
  const { x, y } = getVisualCoords(0, 4, 'red');
  assert.equal(x, 4 * CELL_WIDTH + PADDING);
  assert.equal(y, 9 * CELL_HEIGHT + PADDING);

  const logic = getLogicCoords(x, y, 'red');
  assert.deepEqual(logic, [0, 4]);
});

test('getVisualCoords and getLogicCoords invert 180 degrees for Black perspective', () => {
  const { x, y } = getVisualCoords(0, 4, 'black');
  assert.equal(x, 4 * CELL_WIDTH + PADDING);
  assert.equal(y, 0 * CELL_HEIGHT + PADDING);

  const logic = getLogicCoords(x, y, 'black');
  assert.deepEqual(logic, [0, 4]);

  // Out of bounds returns null
  assert.equal(getLogicCoords(-50, -50, 'red'), null);
  assert.equal(getLogicCoords(BOARD_SIZE + 50, BOARD_SIZE + 50, 'black'), null);
});
