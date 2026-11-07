import test from 'node:test';
import assert from 'node:assert/strict';
import {
  PolyfillGameState,
  PolyfillEngine,
  createInitialBoard,
  isInsidePalace,
} from './enginePolyfill.js';

test('createInitialBoard sets up exactly 32 Xiangqi pieces in standard positions', () => {
  const board = createInitialBoard();
  let redCount = 0;
  let blackCount = 0;

  for (let r = 0; r < 10; r++) {
    for (let c = 0; c < 9; c++) {
      const p = board[r]![c];
      if (p) {
        if (p.color === 'red') redCount++;
        if (p.color === 'black') blackCount++;
      }
    }
  }

  assert.equal(redCount, 16);
  assert.equal(blackCount, 16);

  // Red General at (0, 4), Black General at (9, 4)
  assert.equal(board[0]![4]?.type, 'General');
  assert.equal(board[0]![4]?.color, 'red');
  assert.equal(board[9]![4]?.type, 'General');
  assert.equal(board[9]![4]?.color, 'black');
});

test('PolyfillGameState executes legal moves and records captures in history', () => {
  const game = new PolyfillGameState();
  assert.equal(game.turn(), 'red');
  assert.equal(game.is_in_check(), false);

  // Red Cannon 2,1 moves to 2,4 (Central Cannon)
  const res1 = game.make_move(2, 1, 2, 4);
  assert.equal(res1.success, true);
  assert.equal(res1.captured, null);
  assert.equal(game.turn(), 'black');

  // Black moves Horse 9,7 to 7,6
  const res2 = game.make_move(9, 7, 7, 6);
  assert.equal(res2.success, true);
  assert.equal(res2.captured, null);
  assert.equal(game.turn(), 'red');

  // Red Cannon captures Black Soldier at 6,4!
  const res3 = game.make_move(2, 4, 6, 4);
  assert.equal(res3.success, true);
  assert.notEqual(res3.captured, null);
  assert.equal(res3.captured?.type, 'Soldier');
  assert.equal(res3.captured?.color, 'black');

  // Check history
  const history = game.get_history();
  assert.equal(history.length, 3);
  assert.equal(history[2]?.captured?.type, 'Soldier');
  assert.equal(history[2]?.captured?.color, 'black');

  // Test undo
  const undone = game.undo_move();
  assert.equal(undone, true);
  assert.equal(game.turn(), 'red');
  assert.equal(game.get_history().length, 2);
});

test('PolyfillEngine search returns legal move', () => {
  const game = new PolyfillGameState();
  const engine = new PolyfillEngine();

  const searchResult = engine.search(game, 200, 2);
  assert.notEqual(searchResult, null);
  assert.ok(searchResult!.move.from_row >= 0 && searchResult!.move.from_row <= 9);
  assert.ok(searchResult!.move.to_row >= 0 && searchResult!.move.to_row <= 9);
  assert.ok(searchResult!.stats.nodes > 0);
});

test('PolyfillGameState from_fen parses custom position accurately', () => {
  // Simple puzzle: Red King at (0,4), Red Chariot at (4,4), Black King at (9,4)
  const fen = '4k4/9/9/9/9/4R4/9/9/9/4K4 w - - 0 1';
  const game = PolyfillGameState.from_fen(fen);
  assert.equal(game.turn(), 'red');
  const board = game.get_board();
  const rKing = board.find((s) => s.row === 0 && s.col === 4);
  assert.equal(rKing?.piece?.type, 'General');
  assert.equal(rKing?.piece?.color, 'red');

  const bKing = board.find((s) => s.row === 9 && s.col === 4);
  assert.equal(bKing?.piece?.type, 'General');
  assert.equal(bKing?.piece?.color, 'black');

  const rChariot = board.find((s) => s.row === 4 && s.col === 4);
  assert.equal(rChariot?.piece?.type, 'Chariot');
  assert.equal(rChariot?.piece?.color, 'red');
});
