import type { PieceColor, PieceType } from '../../types/game.js';

export const BOARD_SIZE = 554.0;
export const PADDING = 34.0;
export const GRID_SIZE = BOARD_SIZE - PADDING * 2; // 486.0
export const CELL_WIDTH = GRID_SIZE / 8; // 60.75
export const CELL_HEIGHT = GRID_SIZE / 9; // 54.0
export const BOARD_WIDTH = BOARD_SIZE;
export const BOARD_HEIGHT = BOARD_SIZE;

export const FIDUCIAL_POINTS: [number, number][] = [
  // Cannons
  [2, 1],
  [2, 7],
  [7, 1],
  [7, 7],
  // Soldiers
  [3, 0],
  [3, 2],
  [3, 4],
  [3, 6],
  [3, 8],
  [6, 0],
  [6, 2],
  [6, 4],
  [6, 6],
  [6, 8],
];

export function getPieceSymbol(type: PieceType, color: PieceColor): string {
  switch (type) {
    case 'General':
      return color === 'red' ? '帥' : '將';
    case 'Advisor':
      return color === 'red' ? '仕' : '士';
    case 'Elephant':
      return color === 'red' ? '相' : '象';
    case 'Horse':
      return color === 'red' ? '傌' : '馬';
    case 'Chariot':
      return color === 'red' ? '俥' : '車';
    case 'Cannon':
      return color === 'red' ? '炮' : '砲';
    case 'Soldier':
      return color === 'red' ? '兵' : '卒';
  }
}

export function getVisualCoords(
  row: number,
  col: number,
  side: PieceColor
): { x: number; y: number } {
  let visCol: number;
  let visRow: number;

  if (side === 'black') {
    visCol = 8 - col;
    visRow = row;
  } else {
    visCol = col;
    visRow = 9 - row;
  }

  return {
    x: visCol * CELL_WIDTH + PADDING,
    y: visRow * CELL_HEIGHT + PADDING,
  };
}

export function getLogicCoords(
  x: number,
  y: number,
  side: PieceColor
): [number, number] | null {
  const visCol = Math.round((x - PADDING) / CELL_WIDTH);
  const visRow = Math.round((y - PADDING) / CELL_HEIGHT);

  if (visCol < 0 || visCol > 8 || visRow < 0 || visRow > 9) {
    return null;
  }

  const snapX = visCol * CELL_WIDTH + PADDING;
  const snapY = visRow * CELL_HEIGHT + PADDING;
  const dist = Math.hypot(x - snapX, y - snapY);
  if (dist > Math.min(CELL_WIDTH, CELL_HEIGHT) * 0.48) {
    return null;
  }

  let col: number;
  let row: number;

  if (side === 'black') {
    col = 8 - visCol;
    row = visRow;
  } else {
    col = visCol;
    row = 9 - visRow;
  }

  return [row, col];
}
