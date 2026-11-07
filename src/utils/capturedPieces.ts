import type { HistoryRecord, Piece, PieceColor, PieceType } from '../types/game.js';

export interface GroupedPiece {
  type: PieceType;
  color: PieceColor;
  count: number;
  symbol: string;
  nameVi: string;
  baseValue: number;
  totalValue: number;
}

export interface CapturedPiecesSummary {
  /** Pieces of Red that have been captured (Black took them) */
  redCaptured: Piece[];
  /** Pieces of Black that have been captured (Red took them) */
  blackCaptured: Piece[];
  /** Grouped summary of Red pieces captured */
  groupedRedCaptured: GroupedPiece[];
  /** Grouped summary of Black pieces captured */
  groupedBlackCaptured: GroupedPiece[];
  /** Total material value of Red pieces captured */
  redCapturedValue: number;
  /** Total material value of Black pieces captured */
  blackCapturedValue: number;
  /** Material advantage: positive means Red is ahead, negative means Black is ahead */
  advantageForRed: number;
}

export const PIECE_VALUES: Record<PieceType, number> = {
  General: 0,
  Advisor: 2,
  Elephant: 2,
  Horse: 4,
  Cannon: 4.5,
  Chariot: 9,
  Soldier: 1,
};

export const PIECE_NAMES_VI: Record<PieceType, { red: string; black: string }> = {
  General: { red: 'Tướng (Soái)', black: 'Tướng (Tướng)' },
  Advisor: { red: 'Sĩ', black: 'Sĩ' },
  Elephant: { red: 'Tượng (Tướng)', black: 'Tượng (Voi)' },
  Horse: { red: 'Mã', black: 'Mã' },
  Cannon: { red: 'Pháo', black: 'Pháo' },
  Chariot: { red: 'Xe', black: 'Xe' },
  Soldier: { red: 'Binh', black: 'Tốt' },
};

export const PIECE_SYMBOLS: Record<PieceType, { red: string; black: string }> = {
  General: { red: '帥', black: '將' },
  Advisor: { red: '仕', black: '士' },
  Elephant: { red: '相', black: '象' },
  Horse: { red: '傌', black: '馬' },
  Cannon: { red: '炮', black: '砲' },
  Chariot: { red: '俥', black: '車' },
  Soldier: { red: '兵', black: '卒' },
};

// Piece ordering for standard display hierarchy (Xe > Phao > Ma > Tuong > Si > Tot)
const PIECE_ORDER: PieceType[] = [
  'Chariot',
  'Cannon',
  'Horse',
  'Elephant',
  'Advisor',
  'Soldier',
  'General',
];

/**
 * Extracts and categorizes all captured pieces from the game history
 */
export function extractCapturedPieces(history: HistoryRecord[]): CapturedPiecesSummary {
  const redCaptured: Piece[] = [];
  const blackCaptured: Piece[] = [];

  for (const record of history) {
    if (record.captured) {
      if (record.captured.color === 'red') {
        redCaptured.push(record.captured);
      } else if (record.captured.color === 'black') {
        blackCaptured.push(record.captured);
      }
    }
  }

  const groupPieces = (pieces: Piece[], color: PieceColor): GroupedPiece[] => {
    const counts = new Map<PieceType, number>();
    for (const p of pieces) {
      counts.set(p.type, (counts.get(p.type) || 0) + 1);
    }

    const grouped: GroupedPiece[] = [];
    for (const type of PIECE_ORDER) {
      const count = counts.get(type);
      if (count && count > 0) {
        const baseValue = PIECE_VALUES[type];
        grouped.push({
          type,
          color,
          count,
          symbol: PIECE_SYMBOLS[type][color],
          nameVi: PIECE_NAMES_VI[type][color],
          baseValue,
          totalValue: baseValue * count,
        });
      }
    }
    return grouped;
  };

  const groupedRedCaptured = groupPieces(redCaptured, 'red');
  const groupedBlackCaptured = groupPieces(blackCaptured, 'black');

  const redCapturedValue = groupedRedCaptured.reduce((sum, g) => sum + g.totalValue, 0);
  const blackCapturedValue = groupedBlackCaptured.reduce((sum, g) => sum + g.totalValue, 0);

  // When Black pieces are captured, Red gains material advantage
  const advantageForRed = blackCapturedValue - redCapturedValue;

  return {
    redCaptured,
    blackCaptured,
    groupedRedCaptured,
    groupedBlackCaptured,
    redCapturedValue,
    blackCapturedValue,
    advantageForRed,
  };
}

export type LayoutOrientation = 'sides' | 'stacked';

/**
 * Determines layout orientation based on screen dimensions and aspect ratio.
 * In landscape mode (aspect ratio > 1.15 and width >= 768), captured piece trays
 * appear on the left and right sides.
 * In portrait mode or narrow screens, they appear stacked above and below the board.
 */
export function determineLayoutOrientation(
  width: number,
  height: number
): LayoutOrientation {
  if (width <= 0 || height <= 0) return 'stacked';
  const aspectRatio = width / height;
  if (width >= 820 && aspectRatio >= 1.08) {
    return 'sides';
  }
  return 'stacked';
}

/**
 * Calculates square chessboard dimension to equal the smaller edge of the screen:
 * W x H:
 * - If H < W then size is H (e.g. 1920x1080 -> 1080, 2560x1440 -> 1440)
 * - If H > W then size is W (e.g. 390x844 -> 390, 768x1024 -> 768)
 */
export function calculateOptimalBoardSize(
  viewportWidth: number,
  viewportHeight: number,
  _orientation?: LayoutOrientation
): number {
  if (viewportWidth <= 0 && viewportHeight <= 0) return 480;
  if (viewportWidth <= 0) return Math.round(viewportHeight);
  if (viewportHeight <= 0) return Math.round(viewportWidth);
  return Math.round(Math.min(viewportWidth, viewportHeight));
}
