// Pure TypeScript/JavaScript polyfill engine for Xiangqi (Cờ Tướng)
// Replaces or falls back for cotuong_core WASM when native binary is not available.

import type {
  EngineConfig,
  GameStatus,
  HistoryRecord,
  MoveCoords,
  Piece,
  PieceColor,
  PieceType,
  SearchResult,
  Square,
} from '../types/game.js';

export interface InternalPiece {
  type: PieceType;
  color: PieceColor;
}

export type BoardGrid = (InternalPiece | null)[][]; // 10 rows x 9 cols

export const INITIAL_FEN = 'rnbakabnr/9/1c5c1/s1s1s1s1s/9/9/S1S1S1S1S/1C5C1/9/RNBAKABNR w - - 0 1';

export function createInitialBoard(): BoardGrid {
  const grid: BoardGrid = Array.from({ length: 10 }, () => Array(9).fill(null));

  // Red pieces (rows 0-3) - base at row 0
  grid[0]![0] = { type: 'Chariot', color: 'red' };
  grid[0]![1] = { type: 'Horse', color: 'red' };
  grid[0]![2] = { type: 'Elephant', color: 'red' };
  grid[0]![3] = { type: 'Advisor', color: 'red' };
  grid[0]![4] = { type: 'General', color: 'red' };
  grid[0]![5] = { type: 'Advisor', color: 'red' };
  grid[0]![6] = { type: 'Elephant', color: 'red' };
  grid[0]![7] = { type: 'Horse', color: 'red' };
  grid[0]![8] = { type: 'Chariot', color: 'red' };

  grid[2]![1] = { type: 'Cannon', color: 'red' };
  grid[2]![7] = { type: 'Cannon', color: 'red' };

  for (let c = 0; c < 9; c += 2) {
    grid[3]![c] = { type: 'Soldier', color: 'red' };
  }

  // Black pieces (rows 6-9) - base at row 9
  grid[9]![0] = { type: 'Chariot', color: 'black' };
  grid[9]![1] = { type: 'Horse', color: 'black' };
  grid[9]![2] = { type: 'Elephant', color: 'black' };
  grid[9]![3] = { type: 'Advisor', color: 'black' };
  grid[9]![4] = { type: 'General', color: 'black' };
  grid[9]![5] = { type: 'Advisor', color: 'black' };
  grid[9]![6] = { type: 'Elephant', color: 'black' };
  grid[9]![7] = { type: 'Horse', color: 'black' };
  grid[9]![8] = { type: 'Chariot', color: 'black' };

  grid[7]![1] = { type: 'Cannon', color: 'black' };
  grid[7]![7] = { type: 'Cannon', color: 'black' };

  for (let c = 0; c < 9; c += 2) {
    grid[6]![c] = { type: 'Soldier', color: 'black' };
  }

  return grid;
}

export function cloneBoard(grid: BoardGrid): BoardGrid {
  return grid.map((row) => row.map((cell) => (cell ? { ...cell } : null)));
}

export function isInsidePalace(row: number, col: number, color: PieceColor): boolean {
  if (col < 3 || col > 5) return false;
  if (color === 'red') {
    return row >= 0 && row <= 2;
  } else {
    return row >= 7 && row <= 9;
  }
}

// Check if generals are facing each other with no intervening pieces (Flying General rule)
export function isFlyingGeneral(grid: BoardGrid): boolean {
  let redGen: [number, number] | null = null;
  let blackGen: [number, number] | null = null;

  for (let r = 0; r < 10; r++) {
    for (let c = 3; c <= 5; c++) {
      const p = grid[r]![c];
      if (p && p.type === 'General') {
        if (p.color === 'red') redGen = [r, c];
        if (p.color === 'black') blackGen = [r, c];
      }
    }
  }

  if (!redGen || !blackGen) return false;
  if (redGen[1] !== blackGen[1]) return false;

  const col = redGen[1];
  const startRow = Math.min(redGen[0], blackGen[0]) + 1;
  const endRow = Math.max(redGen[0], blackGen[0]);

  for (let r = startRow; r < endRow; r++) {
    if (grid[r]![col] !== null) return false;
  }

  return true;
}

export function isSquareUnderAttack(
  grid: BoardGrid,
  targetRow: number,
  targetCol: number,
  byColor: PieceColor
): boolean {
  // Check attacks from byColor
  for (let r = 0; r < 10; r++) {
    for (let c = 0; c < 9; c++) {
      const p = grid[r]![c];
      if (!p || p.color !== byColor) continue;

      if (canPieceAttackSquare(grid, r, c, targetRow, targetCol, p)) {
        return true;
      }
    }
  }
  return false;
}

function canPieceAttackSquare(
  grid: BoardGrid,
  fromR: number,
  fromC: number,
  toR: number,
  toC: number,
  piece: InternalPiece
): boolean {
  const dr = toR - fromR;
  const dc = toC - fromC;
  const absDr = Math.abs(dr);
  const absDc = Math.abs(dc);

  switch (piece.type) {
    case 'General': {
      // General attacks within palace or flying general
      if (absDr + absDc === 1 && isInsidePalace(toR, toC, piece.color)) return true;
      // Flying general attack
      if (fromC === toC) {
        const step = toR > fromR ? 1 : -1;
        let blocked = false;
        for (let r = fromR + step; r !== toR; r += step) {
          if (grid[r]![fromC] !== null) {
            blocked = true;
            break;
          }
        }
        if (!blocked) return true;
      }
      return false;
    }
    case 'Advisor': {
      return absDr === 1 && absDc === 1 && isInsidePalace(toR, toC, piece.color);
    }
    case 'Elephant': {
      if (absDr === 2 && absDc === 2) {
        // Must stay on own side
        if (piece.color === 'red' && toR > 4) return false;
        if (piece.color === 'black' && toR < 5) return false;
        // Check eye
        const eyeR = fromR + dr / 2;
        const eyeC = fromC + dc / 2;
        return grid[eyeR]![eyeC] === null;
      }
      return false;
    }
    case 'Horse': {
      if ((absDr === 2 && absDc === 1) || (absDr === 1 && absDc === 2)) {
        // Check horse leg
        const legR = absDr === 2 ? fromR + Math.sign(dr) : fromR;
        const legC = absDc === 2 ? fromC + Math.sign(dc) : fromC;
        return grid[legR]![legC] === null;
      }
      return false;
    }
    case 'Chariot': {
      if (fromR !== toR && fromC !== toC) return false;
      const stepR = fromR === toR ? 0 : Math.sign(dr);
      const stepC = fromC === toC ? 0 : Math.sign(dc);
      let currR = fromR + stepR;
      let currC = fromC + stepC;
      while (currR !== toR || currC !== toC) {
        if (grid[currR]![currC] !== null) return false;
        currR += stepR;
        currC += stepC;
      }
      return true;
    }
    case 'Cannon': {
      if (fromR !== toR && fromC !== toC) return false;
      const stepR = fromR === toR ? 0 : Math.sign(dr);
      const stepC = fromC === toC ? 0 : Math.sign(dc);
      let count = 0;
      let currR = fromR + stepR;
      let currC = fromC + stepC;
      while (currR !== toR || currC !== toC) {
        if (grid[currR]![currC] !== null) count++;
        currR += stepR;
        currC += stepC;
      }
      // Cannon attacks square only if exactly one screen in between
      return count === 1;
    }
    case 'Soldier': {
      if (piece.color === 'red') {
        // Red moves towards row 9 (dr = +1)
        if (fromR <= 4) {
          // Before river: only forward
          return dr === 1 && dc === 0;
        } else {
          // After river: forward or sideways
          return (dr === 1 && dc === 0) || (dr === 0 && absDc === 1);
        }
      } else {
        // Black moves towards row 0 (dr = -1)
        if (fromR >= 5) {
          // Before river: only forward
          return dr === -1 && dc === 0;
        } else {
          // After river: forward or sideways
          return (dr === -1 && dc === 0) || (dr === 0 && absDc === 1);
        }
      }
    }
  }
}

export function isKingInCheck(grid: BoardGrid, color: PieceColor): boolean {
  let kingRow = -1;
  let kingCol = -1;

  for (let r = 0; r < 10; r++) {
    for (let c = 3; c <= 5; c++) {
      const p = grid[r]![c];
      if (p && p.type === 'General' && p.color === color) {
        kingRow = r;
        kingCol = c;
        break;
      }
    }
    if (kingRow !== -1) break;
  }

  if (kingRow === -1) return true; // General dead = in check

  // Check if flying generals
  if (isFlyingGeneral(grid)) return true;

  const opponentColor: PieceColor = color === 'red' ? 'black' : 'red';
  return isSquareUnderAttack(grid, kingRow, kingCol, opponentColor);
}

export function generatePseudoLegalMovesForPiece(
  grid: BoardGrid,
  r: number,
  c: number,
  piece: InternalPiece
): [number, number][] {
  const moves: [number, number][] = [];

  const addMoveIfValid = (toR: number, toC: number) => {
    if (toR < 0 || toR > 9 || toC < 0 || toC > 8) return;
    const destPiece = grid[toR]![toC];
    if (destPiece && destPiece.color === piece.color) return; // cannot capture own
    moves.push([toR, toC]);
  };

  switch (piece.type) {
    case 'General': {
      const deltas = [
        [-1, 0],
        [1, 0],
        [0, -1],
        [0, 1],
      ];
      for (const [dr, dc] of deltas) {
        const nr = r + dr;
        const nc = c + dc;
        if (isInsidePalace(nr, nc, piece.color)) {
          addMoveIfValid(nr, nc);
        }
      }
      break;
    }
    case 'Advisor': {
      const deltas = [
        [-1, -1],
        [-1, 1],
        [1, -1],
        [1, 1],
      ];
      for (const [dr, dc] of deltas) {
        const nr = r + dr;
        const nc = c + dc;
        if (isInsidePalace(nr, nc, piece.color)) {
          addMoveIfValid(nr, nc);
        }
      }
      break;
    }
    case 'Elephant': {
      const deltas = [
        [-2, -2],
        [-2, 2],
        [2, -2],
        [2, 2],
      ];
      for (const [dr, dc] of deltas) {
        const nr = r + dr;
        const nc = c + dc;
        if (piece.color === 'red' && nr > 4) continue;
        if (piece.color === 'black' && nr < 5) continue;
        if (nr < 0 || nr > 9 || nc < 0 || nc > 8) continue;

        // Eye must be empty
        const eyeR = r + dr / 2;
        const eyeC = c + dc / 2;
        if (grid[eyeR]![eyeC] === null) {
          addMoveIfValid(nr, nc);
        }
      }
      break;
    }
    case 'Horse': {
      const jumps = [
        { dr: -2, dc: -1, legR: -1, legC: 0 },
        { dr: -2, dc: 1, legR: -1, legC: 0 },
        { dr: 2, dc: -1, legR: 1, legC: 0 },
        { dr: 2, dc: 1, legR: 1, legC: 0 },
        { dr: -1, dc: -2, legR: 0, legC: -1 },
        { dr: 1, dc: -2, legR: 0, legC: -1 },
        { dr: -1, dc: 2, legR: 0, legC: 1 },
        { dr: 1, dc: 2, legR: 0, legC: 1 },
      ];
      for (const j of jumps) {
        const legR = r + j.legR;
        const legC = c + j.legC;
        if (legR < 0 || legR > 9 || legC < 0 || legC > 8) continue;
        if (grid[legR]![legC] === null) {
          addMoveIfValid(r + j.dr, c + j.dc);
        }
      }
      break;
    }
    case 'Chariot': {
      const dirs = [
        [-1, 0],
        [1, 0],
        [0, -1],
        [0, 1],
      ];
      for (const [dr, dc] of dirs) {
        let nr = r + dr;
        let nc = c + dc;
        while (nr >= 0 && nr <= 9 && nc >= 0 && nc <= 8) {
          const dest = grid[nr]![nc];
          if (!dest) {
            moves.push([nr, nc]);
          } else {
            if (dest.color !== piece.color) {
              moves.push([nr, nc]);
            }
            break;
          }
          nr += dr;
          nc += dc;
        }
      }
      break;
    }
    case 'Cannon': {
      const dirs = [
        [-1, 0],
        [1, 0],
        [0, -1],
        [0, 1],
      ];
      for (const [dr, dc] of dirs) {
        let nr = r + dr;
        let nc = c + dc;
        let foundPlatform = false;
        while (nr >= 0 && nr <= 9 && nc >= 0 && nc <= 8) {
          const dest = grid[nr]![nc];
          if (!foundPlatform) {
            if (!dest) {
              moves.push([nr, nc]); // quiet slide
            } else {
              foundPlatform = true; // first piece hit is screen
            }
          } else {
            if (dest) {
              if (dest.color !== piece.color) {
                moves.push([nr, nc]); // capture jump
              }
              break;
            }
          }
          nr += dr;
          nc += dc;
        }
      }
      break;
    }
    case 'Soldier': {
      if (piece.color === 'red') {
        // Forward towards row 9 (dr = +1)
        addMoveIfValid(r + 1, c);
        // After crossing river (r >= 5), sideways
        if (r >= 5) {
          addMoveIfValid(r, c - 1);
          addMoveIfValid(r, c + 1);
        }
      } else {
        // Forward towards row 0 (dr = -1)
        addMoveIfValid(r - 1, c);
        // After crossing river (r <= 4), sideways
        if (r <= 4) {
          addMoveIfValid(r, c - 1);
          addMoveIfValid(r, c + 1);
        }
      }
      break;
    }
  }

  return moves;
}

export function getLegalMovesForSquare(
  grid: BoardGrid,
  turn: PieceColor,
  r: number,
  c: number
): [number, number][] {
  const piece = grid[r]![c];
  if (!piece || piece.color !== turn) return [];

  const pseudoMoves = generatePseudoLegalMovesForPiece(grid, r, c, piece);
  const legal: [number, number][] = [];

  for (const [toR, toC] of pseudoMoves) {
    // Try move
    const captured = grid[toR]![toC];
    grid[toR]![toC] = piece;
    grid[r]![c] = null;

    const inCheck = isKingInCheck(grid, turn);

    // Revert move
    grid[r]![c] = piece;
    grid[toR]![toC] = captured;

    if (!inCheck) {
      legal.push([toR, toC]);
    }
  }

  return legal;
}

export function getAllStrictlyLegalMoves(
  grid: BoardGrid,
  turn: PieceColor
): { fromR: number; fromC: number; toR: number; toC: number }[] {
  const allMoves: { fromR: number; fromC: number; toR: number; toC: number }[] = [];

  for (let r = 0; r < 10; r++) {
    for (let c = 0; c < 9; c++) {
      const p = grid[r]![c];
      if (p && p.color === turn) {
        const dests = getLegalMovesForSquare(grid, turn, r, c);
        for (const [toR, toC] of dests) {
          allMoves.push({ fromR: r, fromC: c, toR, toC });
        }
      }
    }
  }

  return allMoves;
}

/**
 * Pure JavaScript WasmGameState implementation
 */
export class PolyfillGameState {
  private grid: BoardGrid;
  private currentTurn: PieceColor = 'red';
  private moveHistory: HistoryRecord[] = [];
  private undoStack: {
    fromR: number;
    fromC: number;
    toR: number;
    toC: number;
    piece: Piece;
    captured: Piece | null;
  }[] = [];

  constructor() {
    this.grid = createInitialBoard();
  }

  static from_json(json_str: string): PolyfillGameState {
    const s = new PolyfillGameState();
    try {
      const data = JSON.parse(json_str);
      if (data && data.grid) {
        s.grid = data.grid;
        s.currentTurn = data.turn ?? 'red';
        s.moveHistory = data.history ?? [];
        s.undoStack = data.undoStack ?? [];
      }
    } catch (e) {
      console.warn('PolyfillGameState from_json error:', e);
    }
    return s;
  }

  static from_fen(fen: string): PolyfillGameState {
    const s = new PolyfillGameState();
    try {
      const parts = fen.trim().split(/\s+/);
      if (parts[0]) {
        const rows = parts[0].split('/');
        if (rows.length === 10) {
          const newGrid: BoardGrid = Array.from({ length: 10 }, () => Array(9).fill(null));
          const charMap: Record<string, PieceType> = {
            k: 'General',
            a: 'Advisor',
            b: 'Elephant',
            e: 'Elephant',
            n: 'Horse',
            h: 'Horse',
            r: 'Chariot',
            c: 'Cannon',
            p: 'Soldier',
            s: 'Soldier',
          };

          for (let rIdx = 0; rIdx < 10; rIdx++) {
            const rowStr = rows[rIdx]!;
            const r = 9 - rIdx;
            let c = 0;
            for (const ch of rowStr) {
              const digit = Number.parseInt(ch, 10);
              if (!Number.isNaN(digit)) {
                c += digit;
              } else {
                const lower = ch.toLowerCase();
                const pType = charMap[lower];
                if (pType && c < 9) {
                  const pColor: PieceColor = ch === ch.toUpperCase() ? 'red' : 'black';
                  newGrid[r]![c] = { type: pType, color: pColor };
                  c++;
                }
              }
            }
          }
          s.grid = newGrid;
          s.currentTurn = parts[1]?.toLowerCase() === 'b' ? 'black' : 'red';
        }
      }
    } catch (e) {
      console.warn('PolyfillGameState from_fen error:', e);
    }
    return s;
  }

  to_fen(): string {
    const pieceCharMap: Record<PieceType, string> = {
      General: 'k',
      Advisor: 'a',
      Elephant: 'b',
      Horse: 'n',
      Chariot: 'r',
      Cannon: 'c',
      Soldier: 'p',
    };
    const rows: string[] = [];
    for (let r = 9; r >= 0; r--) {
      let emptyCount = 0;
      let rowStr = '';
      for (let c = 0; c < 9; c++) {
        const p = this.grid[r]![c];
        if (!p) {
          emptyCount++;
        } else {
          if (emptyCount > 0) {
            rowStr += emptyCount.toString();
            emptyCount = 0;
          }
          const ch = pieceCharMap[p.type] ?? 'p';
          rowStr += p.color === 'red' ? ch.toUpperCase() : ch.toLowerCase();
        }
      }
      if (emptyCount > 0) rowStr += emptyCount.toString();
      rows.push(rowStr);
    }
    const t = this.currentTurn === 'red' ? 'w' : 'b';
    return `${rows.join('/')} ${t} - - 0 1`;
  }

  to_json(): string {
    return JSON.stringify({
      grid: this.grid,
      turn: this.currentTurn,
      history: this.moveHistory,
      undoStack: this.undoStack,
    });
  }

  turn(): PieceColor {
    return this.currentTurn;
  }

  is_in_check(): boolean {
    return isKingInCheck(this.grid, this.currentTurn);
  }

  status(): GameStatus {
    const legalMoves = getAllStrictlyLegalMoves(this.grid, this.currentTurn);
    if (legalMoves.length === 0) {
      const inCheck = this.is_in_check();
      if (inCheck) {
        const winner = this.currentTurn === 'red' ? 'black' : 'red';
        return {
          status: 'Checkmate',
          winner,
          reason: 'Chiếu bí (Checkmate)',
        };
      } else {
        return {
          status: 'Stalemate',
          winner: null,
          reason: 'Hết nước đi hợp lệ (Stalemate)',
        };
      }
    }
    return { status: 'Playing' };
  }

  get_board(): Square[] {
    const squares: Square[] = [];
    for (let r = 0; r < 10; r++) {
      for (let c = 0; c < 9; c++) {
        const p = this.grid[r]![c];
        squares.push({
          row: r,
          col: c,
          piece: p ? { type: p.type, color: p.color } : null,
        });
      }
    }
    return squares;
  }

  count_move_back_and_forth(from_row: number, from_col: number, to_row: number, to_col: number): number {
    const playerMoves = this.moveHistory.filter(r => r.piece.color === this.currentTurn);
    let count = 0;
    let i = playerMoves.length;
    while (i >= 2) {
      const last = playerMoves[i - 1]!;
      const prev = playerMoves[i - 2]!;
      if (
        last.from_row === to_row &&
        last.from_col === to_col &&
        last.to_row === from_row &&
        last.to_col === from_col &&
        prev.from_row === from_row &&
        prev.from_col === from_col &&
        prev.to_row === to_row &&
        prev.to_col === to_col
      ) {
        count++;
        i -= 2;
      } else {
        break;
      }
    }
    return count;
  }

  get_strictly_legal_moves(): { fromR: number; fromC: number; toR: number; toC: number }[] {
    const candidateMoves = getAllStrictlyLegalMoves(this.grid, this.currentTurn);
    const cleanMoves: { fromR: number; fromC: number; toR: number; toC: number }[] = [];
    const repMoves: { fromR: number; fromC: number; toR: number; toC: number }[] = [];

    for (const m of candidateMoves) {
      if (this.count_move_back_and_forth(m.fromR, m.fromC, m.toR, m.toC) >= 3) {
        repMoves.push(m);
      } else {
        cleanMoves.push(m);
      }
    }

    if (cleanMoves.length < 2) {
      return [...cleanMoves, ...repMoves];
    }
    return cleanMoves;
  }

  get_legal_moves(row: number, col: number): [number, number][] {
    const strictly = this.get_strictly_legal_moves();
    return strictly
      .filter((m) => m.fromR === row && m.fromC === col)
      .map((m) => [m.toR, m.toC]);
  }

  is_move_legal(from_row: number, from_col: number, to_row: number, to_col: number): boolean {
    const legal = this.get_legal_moves(from_row, from_col);
    return legal.some(([r, c]) => r === to_row && c === to_col);
  }

  get_strictly_legal_moves_flat(): number[] {
    const moves = this.get_strictly_legal_moves();
    const flat: number[] = [];
    for (const m of moves) {
      flat.push(m.fromR, m.fromC, m.toR, m.toC);
    }
    return flat;
  }

  make_move(
    from_row: number,
    from_col: number,
    to_row: number,
    to_col: number
  ): { success: boolean; captured: Piece | null; in_check: boolean } {
    if (!this.is_move_legal(from_row, from_col, to_row, to_col)) {
      throw new Error(`Illegal move from (${from_row},${from_col}) to (${to_row},${to_col})`);
    }

    const piece = this.grid[from_row]![from_col]!;
    const captured = this.grid[to_row]![to_col];

    const capturedPiece: Piece | null = captured
      ? { type: captured.type, color: captured.color }
      : null;

    // Apply move
    this.grid[to_row]![to_col] = piece;
    this.grid[from_row]![from_col] = null;

    this.undoStack.push({
      fromR: from_row,
      fromC: from_col,
      toR: to_row,
      toC: to_col,
      piece: { ...piece },
      captured: capturedPiece,
    });

    const nextTurn: PieceColor = this.currentTurn === 'red' ? 'black' : 'red';
    this.currentTurn = nextTurn;

    const inCheck = isKingInCheck(this.grid, nextTurn);

    this.moveHistory.push({
      from_row,
      from_col,
      to_row,
      to_col,
      piece: { ...piece },
      captured: capturedPiece,
      note: null,
    });

    return {
      success: true,
      captured: capturedPiece,
      in_check: inCheck,
    };
  }

  undo_move(): boolean {
    const last = this.undoStack.pop();
    if (!last) return false;

    this.grid[last.fromR]![last.fromC] = {
      type: last.piece.type,
      color: last.piece.color,
    };
    this.grid[last.toR]![last.toC] = last.captured
      ? { type: last.captured.type, color: last.captured.color }
      : null;

    this.currentTurn = last.piece.color;
    this.moveHistory.pop();
    return true;
  }

  get_history(): HistoryRecord[] {
    return [...this.moveHistory];
  }

  annotate_last_move(note: string): void {
    if (this.moveHistory.length > 0) {
      this.moveHistory[this.moveHistory.length - 1]!.note = note;
    }
  }

  get_book_move(): MoveCoords | null {
    // Opening book favorites for Xiangqi
    if (this.moveHistory.length === 0) {
      // Red opening: Central Cannon (Pháo 2 bình 5 or Pháo 8 bình 5) or Horse
      const options: MoveCoords[] = [
        { from_row: 2, from_col: 1, to_row: 2, to_col: 4 }, // Pháo 2 bình 5
        { from_row: 2, from_col: 7, to_row: 2, to_col: 4 }, // Pháo 8 bình 5
        { from_row: 3, from_col: 2, to_row: 4, to_col: 2 }, // Tiến Binh 3
        { from_row: 3, from_col: 6, to_row: 4, to_col: 6 }, // Tiến Binh 7
        { from_row: 0, from_col: 1, to_row: 2, to_col: 2 }, // Khởi Mã 2
      ];
      return options[Math.floor(Math.random() * options.length)] ?? null;
    } else if (this.moveHistory.length === 1) {
      // Black response: Bình phong Mã / Thuận Pháo
      const redMove = this.moveHistory[0]!;
      if (redMove.to_row === 2 && redMove.to_col === 4) {
        // Red played central cannon: Black plays Horse 8 to 7,6
        return { from_row: 9, from_col: 7, to_row: 7, to_col: 6 };
      }
    }
    return null;
  }

  history_len(): number {
    return this.moveHistory.length;
  }

  free(): void {
    // GC cleans up
  }

  getInternalGrid(): BoardGrid {
    return this.grid;
  }
}

// Basic piece values for evaluation
const PIECE_EVAL: Record<PieceType, number> = {
  General: 10000,
  Chariot: 950,
  Cannon: 480,
  Horse: 420,
  Elephant: 220,
  Advisor: 220,
  Soldier: 120,
};

function evaluateBoard(grid: BoardGrid): number {
  let score = 0;
  for (let r = 0; r < 10; r++) {
    for (let c = 0; c < 9; c++) {
      const p = grid[r]![c];
      if (!p) continue;
      let val = PIECE_EVAL[p.type];

      // Positional bonuses
      if (p.type === 'Soldier') {
        if (p.color === 'red' && r <= 4) val += 90; // crossed river
        if (p.color === 'black' && r >= 5) val += 90;
      } else if (p.type === 'Horse') {
        // Center horses are active
        if (c >= 2 && c <= 6 && r >= 3 && r <= 6) val += 30;
      } else if (p.type === 'Cannon') {
        if (c === 4) val += 35; // Central cannon
      }

      if (p.color === 'red') {
        score += val;
      } else {
        score -= val;
      }
    }
  }
  return score;
}

/**
 * Pure JavaScript WasmEngine implementation
 */
export class PolyfillEngine {
  private config: EngineConfig = {
    mate_score: 300000,
    tt_size_mb: 512,
    nnue_weight: 100,
    threads: 1,
  };
  private currentThreadId: number = 0;

  constructor(configJson?: string | null) {
    if (configJson) {
      try {
        this.config = { ...this.config, ...JSON.parse(configJson) };
      } catch (e) {
        // ignore
      }
    }
  }

  set_thread_id(thread_id: number): void {
    this.currentThreadId = thread_id;
  }

  thread_id(): number {
    return this.currentThreadId;
  }

  update_config(configJson: string): void {
    try {
      this.config = { ...this.config, ...JSON.parse(configJson) };
    } catch (e) {
      // ignore
    }
  }

  clear_tt(): void {
    // Polyfill transposition table clear
  }

  load_weights_bytes(_bytes: Uint8Array): void {
    // No-op for pure JS polyfill
  }

  is_nnue_active(): boolean {
    return false;
  }

  search_with_partition(
    gameState: PolyfillGameState,
    timeLimitMs: bigint | number,
    maxDepth?: number,
    _allowedMovesFlat?: number[]
  ): SearchResult | null {
    return this.search(gameState, timeLimitMs, maxDepth);
  }

  search(
    gameState: PolyfillGameState,
    timeLimitMs: bigint | number,
    maxDepth?: number
  ): SearchResult | null {
    const startTime = Date.now();
    const timeLimit = Number(timeLimitMs);
    const side = gameState.turn();
    const isMaximizing = side === 'red';

    const legalMoves = getAllStrictlyLegalMoves(
      gameState.getInternalGrid(),
      side
    );

    if (legalMoves.length === 0) return null;

    // Fast heuristic ordering: Captures first
    const grid = gameState.getInternalGrid();
    legalMoves.sort((a, b) => {
      const capA = grid[a.toR]![a.toC];
      const capB = grid[b.toR]![b.toC];
      const valA = capA ? PIECE_EVAL[capA.type] : 0;
      const valB = capB ? PIECE_EVAL[capB.type] : 0;
      return valB - valA;
    });

    const targetDepth = maxDepth ?? 3;
    let nodes = 0;
    let bestMove = legalMoves[0]!;
    let bestScore = isMaximizing ? -Infinity : Infinity;

    for (const mv of legalMoves) {
      const piece = grid[mv.fromR]![mv.fromC]!;
      const captured = grid[mv.toR]![mv.toC];

      grid[mv.toR]![mv.toC] = piece;
      grid[mv.fromR]![mv.fromC] = null;
      nodes++;

      const score = this.minimax(
        grid,
        targetDepth - 1,
        -Infinity,
        Infinity,
        !isMaximizing,
        startTime + timeLimit
      );

      grid[mv.fromR]![mv.fromC] = piece;
      grid[mv.toR]![mv.toC] = captured;

      if (isMaximizing) {
        if (score > bestScore) {
          bestScore = score;
          bestMove = mv;
        }
      } else {
        if (score < bestScore) {
          bestScore = score;
          bestMove = mv;
        }
      }

      if (Date.now() - startTime >= timeLimit * 0.95) break;
    }

    return {
      move: {
        from_row: bestMove.fromR,
        from_col: bestMove.fromC,
        to_row: bestMove.toR,
        to_col: bestMove.toC,
      },
      stats: {
        depth: targetDepth,
        nodes,
        time_ms: Date.now() - startTime,
      },
    };
  }

  private minimax(
    grid: BoardGrid,
    depth: number,
    alpha: number,
    beta: number,
    isMaximizing: boolean,
    deadline: number
  ): number {
    if (depth <= 0 || Date.now() >= deadline) {
      return evaluateBoard(grid);
    }

    const side: PieceColor = isMaximizing ? 'red' : 'black';
    const moves = getAllStrictlyLegalMoves(grid, side);
    if (moves.length === 0) {
      return isKingInCheck(grid, side)
        ? isMaximizing
          ? -100000 - depth
          : 100000 + depth
        : 0;
    }

    if (isMaximizing) {
      let maxEval = -Infinity;
      for (const mv of moves) {
        const piece = grid[mv.fromR]![mv.fromC]!;
        const captured = grid[mv.toR]![mv.toC];

        grid[mv.toR]![mv.toC] = piece;
        grid[mv.fromR]![mv.fromC] = null;

        const evaluation = this.minimax(grid, depth - 1, alpha, beta, false, deadline);

        grid[mv.fromR]![mv.fromC] = piece;
        grid[mv.toR]![mv.toC] = captured;

        maxEval = Math.max(maxEval, evaluation);
        alpha = Math.max(alpha, evaluation);
        if (beta <= alpha) break;
      }
      return maxEval;
    } else {
      let minEval = Infinity;
      for (const mv of moves) {
        const piece = grid[mv.fromR]![mv.fromC]!;
        const captured = grid[mv.toR]![mv.toC];

        grid[mv.toR]![mv.toC] = piece;
        grid[mv.fromR]![mv.fromC] = null;

        const evaluation = this.minimax(grid, depth - 1, alpha, beta, true, deadline);

        grid[mv.fromR]![mv.fromC] = piece;
        grid[mv.toR]![mv.toC] = captured;

        minEval = Math.min(minEval, evaluation);
        beta = Math.min(beta, evaluation);
        if (beta <= alpha) break;
      }
      return minEval;
    }
  }
}
