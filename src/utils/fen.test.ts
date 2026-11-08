import test from 'node:test';
import assert from 'node:assert/strict';
import {
  boardToFen,
  fenToBoard,
  validateBoardSetup,
  createEmptyBoard,
  createTwoGeneralsBoard,
  CLASSIC_PUZZLES,
} from './fen.js';
import type { Square } from '../types/game.js';

test('createEmptyBoard creates exactly 90 empty squares', () => {
  const board = createEmptyBoard();
  assert.equal(board.length, 90);
  assert.ok(board.every((sq) => sq.piece === null));
});

test('createTwoGeneralsBoard creates 90 squares with Red and Black Generals in palaces', () => {
  const board = createTwoGeneralsBoard();
  assert.equal(board.length, 90);
  const nonEmpty = board.filter((sq) => sq.piece !== null);
  assert.equal(nonEmpty.length, 2);
  const redGen = board.find((sq) => sq.row === 0 && sq.col === 4);
  const blackGen = board.find((sq) => sq.row === 9 && sq.col === 4);
  assert.deepEqual(redGen?.piece, { type: 'General', color: 'red' });
  assert.deepEqual(blackGen?.piece, { type: 'General', color: 'black' });
});

test('fenToBoard and boardToFen roundtrip correctly', () => {
  const testFen = 'rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w - - 0 1';
  const { squares, turn } = fenToBoard(testFen);

  assert.equal(squares.length, 90);
  assert.equal(turn, 'red');

  // Verify Red General at (0, 4)
  const redGen = squares.find((s) => s.row === 0 && s.col === 4);
  assert.equal(redGen?.piece?.type, 'General');
  assert.equal(redGen?.piece?.color, 'red');

  // Verify Black General at (9, 4)
  const blackGen = squares.find((s) => s.row === 9 && s.col === 4);
  assert.equal(blackGen?.piece?.type, 'General');
  assert.equal(blackGen?.piece?.color, 'black');

  // Roundtrip back to FEN
  const generatedFen = boardToFen(squares, turn);
  const reParsed = fenToBoard(generatedFen);
  assert.equal(reParsed.turn, 'red');
  assert.equal(reParsed.squares.length, 90);
});

test('validateBoardSetup detects missing generals', () => {
  const empty = createEmptyBoard();
  const res = validateBoardSetup(empty);
  assert.equal(res.valid, false);
  assert.ok(res.errors.some((e) => e.includes('Thiếu Tướng Đỏ')));
  assert.ok(res.errors.some((e) => e.includes('Thiếu Tướng Đen')));
});

test('validateBoardSetup detects flying generals without screen', () => {
  const board: Square[] = createEmptyBoard();
  // Place Red General at (0, 4) and Black General at (9, 4) with nothing between
  const rGen = board.find((s) => s.row === 0 && s.col === 4)!;
  rGen.piece = { type: 'General', color: 'red' };
  const bGen = board.find((s) => s.row === 9 && s.col === 4)!;
  bGen.piece = { type: 'General', color: 'black' };

  const res = validateBoardSetup(board);
  assert.equal(res.valid, false);
  assert.ok(res.errors.some((e) => e.includes('Lộ mặt tướng')));

  // Now place a screen piece (e.g. Red Advisor at (1, 4))
  const screen = board.find((s) => s.row === 1 && s.col === 4)!;
  screen.piece = { type: 'Advisor', color: 'red' };
  const resValid = validateBoardSetup(board);
  assert.equal(resValid.valid, true);
});

test('All classic puzzles parse into valid boards', () => {
  assert.ok(CLASSIC_PUZZLES.length >= 5);
  for (const puzzle of CLASSIC_PUZZLES) {
    const { squares, turn } = fenToBoard(puzzle.fen);
    assert.equal(turn, puzzle.turn);
    const validation = validateBoardSetup(squares);
    assert.equal(validation.valid, true, `Puzzle ${puzzle.name} should be valid`);
  }
});
