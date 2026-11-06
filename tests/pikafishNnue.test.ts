import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  EngineBoard,
  PikafishNnueEngine,
  evaluateNnue,
  halfkpFeatureIdxRed,
  L1_INPUT_SIZE,
  palaceIndex,
} from '../src/engine/pikafishNnue.js';

describe('Pure NNUE HalfKP v3 Architecture & Evaluation', () => {
  it('maps palace squares and HalfKP feature indices within bounds', () => {
    assert.equal(palaceIndex(3), 0);
    assert.equal(palaceIndex(4), 1);
    assert.equal(palaceIndex(5), 2);
    assert.equal(palaceIndex(13), 4);
    assert.equal(palaceIndex(22), 7);

    for (let k = 0; k < 9; k++) {
      const f1 = halfkpFeatureIdxRed(k, false, 4, 0);
      assert.ok(f1 !== null && f1 >= 0 && f1 < L1_INPUT_SIZE);
      const f2 = halfkpFeatureIdxRed(k, true, 0, 85);
      assert.ok(f2 !== null && f2 >= 0 && f2 < L1_INPUT_SIZE);
    }
  });

  it('evaluates initial symmetrical position to exactly 0', () => {
    const board = EngineBoard.newStarting();
    assert.equal(evaluateNnue(board), 0);
  });

  it('evaluates material advantage strongly in favor of the side with an extra Chariot', () => {
    const board = EngineBoard.newStarting();
    // Remove Black's Chariot at row 9, col 0 (sq = 81)
    board.setPiece(81, null);
    const score = evaluateNnue(board);
    assert.ok(
      score > 300,
      `Expected Red +Chariot NNUE score > 300, got ${score}`
    );
  });

  it('penalizes Empty Cannon (0 blockers) and 1-Blocker Cannon Mount on King file via ReLU circuits', () => {
    const board = new EngineBoard();
    board.clear();
    // Red King at (0, 4) = sq 4, Black King at (9, 4) = sq 85
    board.setPiece(4, { pieceType: 0, color: 0 });
    board.setPiece(85, { pieceType: 0, color: 1 });
    // Black Cannon at (5, 4) = sq 49 (same file as Red King, 0 blockers)
    board.setPiece(49, { pieceType: 5, color: 1 });
    const scoreExposed0 = evaluateNnue(board);

    // Move Black Cannon to safe file (5, 3) = sq 48
    board.setPiece(49, null);
    board.setPiece(48, { pieceType: 5, color: 1 });
    const scoreSafe0 = evaluateNnue(board);

    assert.ok(
      scoreSafe0 > scoreExposed0 + 30,
      `Expected safe file score (${scoreSafe0}) > 0-blocker exposed score (${scoreExposed0}) + 30`
    );

    // 1-blocker Cannon mount on King file: add Red Advisor at (2, 4) = sq 22
    board.setPiece(48, null);
    board.setPiece(49, { pieceType: 5, color: 1 });
    board.setPiece(22, { pieceType: 1, color: 0 });
    const scoreExposed1 = evaluateNnue(board);

    board.setPiece(49, null);
    board.setPiece(48, { pieceType: 5, color: 1 });
    const scoreSafe1 = evaluateNnue(board);

    assert.ok(
      scoreSafe1 > scoreExposed1 + 30,
      `Expected safe file score (${scoreSafe1}) > 1-blocker exposed score (${scoreExposed1}) + 30`
    );
  });

  it('maintains bit-exact incremental accumulator parity with full refresh across moves, captures, and King moves', () => {
    const board = EngineBoard.newStarting();
    const engine = new PikafishNnueEngine();
    let turn: 0 | 1 = 0;
    const history: { mv: { fromSq: number; toSq: number; score: number }; cap: ReturnType<typeof board.applyMove> }[] = [];

    for (let step = 0; step < 12; step++) {
      const moves = engine.generatePseudoMoves(board, turn, null, null, 1, false);
      const mv = moves[step % moves.length]!;
      const cap = board.applyMove(mv);
      history.push({ mv, cap });

      const fresh = new EngineBoard();
      fresh.grid = [...board.grid];
      fresh.kingSq = [...board.kingSq];
      fresh.pieceCount = board.pieceCount;
      fresh.refreshNnue();

      assert.deepEqual(board.nnueAcc.red, fresh.nnueAcc.red);
      assert.deepEqual(board.nnueAcc.black, fresh.nnueAcc.black);
      assert.deepEqual(board.nnueAcc.psqtRed, fresh.nnueAcc.psqtRed);
      assert.deepEqual(board.nnueAcc.psqtBlack, fresh.nnueAcc.psqtBlack);

      turn = turn === 0 ? 1 : 0;
    }

    while (history.length > 0) {
      const { mv, cap } = history.pop()!;
      board.undoMove(mv, cap);

      const fresh = new EngineBoard();
      fresh.grid = [...board.grid];
      fresh.kingSq = [...board.kingSq];
      fresh.pieceCount = board.pieceCount;
      fresh.refreshNnue();

      assert.deepEqual(board.nnueAcc.red, fresh.nnueAcc.red);
      assert.deepEqual(board.nnueAcc.black, fresh.nnueAcc.black);
      assert.deepEqual(board.nnueAcc.psqtRed, fresh.nnueAcc.psqtRed);
      assert.deepEqual(board.nnueAcc.psqtBlack, fresh.nnueAcc.psqtBlack);
    }
  });
});

describe('Pikafish & Fairy-Stockfish Pruning & Search Pipeline', () => {
  it('finds immediate mate-in-1 with Chariot', () => {
    // Black King at (9, 4), Red Chariot at (8, 0) cutting off rank 8, Red Chariot at (1, 1) -> (9, 1) mate, Red King at (0, 3)
    const fen = '4k4/R8/9/9/9/9/9/9/1R7/3K5 w';
    const engine = new PikafishNnueEngine();
    const res = engine.searchFen(fen, 1000, 4);
    assert.ok(res !== null);
    assert.equal(res.move.from_row, 1);
    assert.equal(res.move.from_col, 1);
    assert.equal(res.move.to_row, 9);
    assert.equal(res.move.to_col, 1);
  });

  it('captures undefended high-value enemy Chariot', () => {
    const fen = '3ak4/9/9/9/4r4/9/4R4/9/9/4K4 w';
    const engine = new PikafishNnueEngine();
    const res = engine.searchFen(fen, 1000, 4);
    assert.ok(res !== null);
    assert.equal(res.move.from_row, 3);
    assert.equal(res.move.from_col, 4);
    assert.equal(res.move.to_row, 5);
    assert.equal(res.move.to_col, 4);
  });
});
