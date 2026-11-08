import test from 'node:test';
import assert from 'node:assert/strict';
import {
  extractCapturedPieces,
  extractOffBoardPieces,
  determineLayoutOrientation,
  calculateOptimalBoardSize,
  PIECE_VALUES,
  PIECE_SYMBOLS,
} from './capturedPieces.js';
import { createTwoGeneralsBoard } from './fen.js';
import type { HistoryRecord, Piece } from '../types/game.js';

test('extractCapturedPieces returns empty arrays for empty history', () => {
  const result = extractCapturedPieces([]);
  assert.equal(result.redCaptured.length, 0);
  assert.equal(result.blackCaptured.length, 0);
  assert.equal(result.groupedRedCaptured.length, 0);
  assert.equal(result.groupedBlackCaptured.length, 0);
  assert.equal(result.redCapturedValue, 0);
  assert.equal(result.blackCapturedValue, 0);
  assert.equal(result.advantageForRed, 0);
});

test('extractOffBoardPieces returns 15 Red and 15 Black pieces when only 2 Generals are on board', () => {
  const board = createTwoGeneralsBoard();
  const summary = extractOffBoardPieces(board);

  // 15 Red pieces off-board (held in Black tray)
  assert.equal(summary.redCaptured.length, 15);
  // 15 Black pieces off-board (held in Red tray)
  assert.equal(summary.blackCaptured.length, 15);
  // Generals are on board so General is not in captured list
  assert.ok(summary.groupedRedCaptured.every((g) => g.type !== 'General'));
  assert.ok(summary.groupedBlackCaptured.every((g) => g.type !== 'General'));
});

test('extractCapturedPieces categorizes captured red and black pieces accurately', () => {
  const redHorse: Piece = { type: 'Horse', color: 'red' };
  const blackCannon: Piece = { type: 'Cannon', color: 'black' };
  const blackChariot: Piece = { type: 'Chariot', color: 'black' };

  const history: HistoryRecord[] = [
    {
      from_row: 2,
      from_col: 1,
      to_row: 2,
      to_col: 4,
      piece: { type: 'Cannon', color: 'red' },
      captured: null,
      note: null,
    },
    {
      from_row: 0,
      from_col: 1,
      to_row: 2,
      to_col: 2,
      piece: { type: 'Horse', color: 'black' },
      captured: redHorse, // Black took Red's Horse
      note: null,
    },
    {
      from_row: 9,
      from_col: 0,
      to_row: 0,
      to_col: 0,
      piece: { type: 'Chariot', color: 'red' },
      captured: blackChariot, // Red took Black's Chariot
      note: null,
    },
    {
      from_row: 2,
      from_col: 4,
      to_row: 7,
      to_col: 4,
      piece: { type: 'Cannon', color: 'red' },
      captured: blackCannon, // Red took Black's Cannon
      note: null,
    },
  ];

  const summary = extractCapturedPieces(history);

  // Red lost 1 Horse (captured by black)
  assert.equal(summary.redCaptured.length, 1);
  assert.equal(summary.redCaptured[0]?.type, 'Horse');
  assert.equal(summary.redCapturedValue, PIECE_VALUES.Horse); // 4

  // Black lost 1 Chariot and 1 Cannon (captured by red)
  assert.equal(summary.blackCaptured.length, 2);
  assert.equal(summary.blackCapturedValue, PIECE_VALUES.Chariot + PIECE_VALUES.Cannon); // 9 + 4.5 = 13.5

  // Red advantage: 13.5 - 4 = +9.5
  assert.equal(summary.advantageForRed, 9.5);

  // Grouped check
  assert.equal(summary.groupedRedCaptured.length, 1);
  assert.equal(summary.groupedRedCaptured[0]?.symbol, PIECE_SYMBOLS.Horse.red);
  assert.equal(summary.groupedRedCaptured[0]?.count, 1);

  assert.equal(summary.groupedBlackCaptured.length, 2);
  // Should be sorted by rank: Chariot (9) first, then Cannon (4.5)
  assert.equal(summary.groupedBlackCaptured[0]?.type, 'Chariot');
  assert.equal(summary.groupedBlackCaptured[1]?.type, 'Cannon');
});

test('determineLayoutOrientation switches between sides and stacked based on aspect ratio and width', () => {
  // Desktop landscape (1440x900, ratio 1.6) -> sides
  assert.equal(determineLayoutOrientation(1440, 900), 'sides');

  // Widescreen laptop (1280x800, ratio 1.6) -> sides
  assert.equal(determineLayoutOrientation(1280, 800), 'sides');

  // Standard tablet landscape (1024x768, ratio 1.33) -> sides
  assert.equal(determineLayoutOrientation(1024, 768), 'sides');

  // Mobile portrait (390x844, ratio 0.46) -> stacked
  assert.equal(determineLayoutOrientation(390, 844), 'stacked');

  // Tablet portrait (768x1024, ratio 0.75) -> stacked
  assert.equal(determineLayoutOrientation(768, 1024), 'stacked');

  // Narrow window (600x900) -> stacked
  assert.equal(determineLayoutOrientation(600, 900), 'stacked');
});

test('calculateOptimalBoardSize strictly equals the smaller edge of the screen (min(W, H))', () => {
  // 2560x1440: H < W -> 1440
  assert.equal(calculateOptimalBoardSize(2560, 1440), 1440);

  // 1920x1080: H < W -> 1080
  assert.equal(calculateOptimalBoardSize(1920, 1080), 1080);

  // 3840x2160 (4K): H < W -> 2160
  assert.equal(calculateOptimalBoardSize(3840, 2160), 2160);

  // 1440x900 (Laptop): H < W -> 900
  assert.equal(calculateOptimalBoardSize(1440, 900), 900);

  // 1024x768 (Tablet landscape): H < W -> 768
  assert.equal(calculateOptimalBoardSize(1024, 768), 768);

  // 768x1024 (Tablet portrait): H > W -> 768
  assert.equal(calculateOptimalBoardSize(768, 1024), 768);

  // 390x844 (Mobile portrait): H > W -> 390
  assert.equal(calculateOptimalBoardSize(390, 844), 390);

  // 412x915 (Android mobile portrait): H > W -> 412
  assert.equal(calculateOptimalBoardSize(412, 915), 412);
});

