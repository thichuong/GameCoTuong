export type PieceType =
  | 'General'
  | 'Advisor'
  | 'Elephant'
  | 'Horse'
  | 'Chariot'
  | 'Cannon'
  | 'Soldier';

export type PieceColor = 'red' | 'black';

export interface Piece {
  type: PieceType;
  color: PieceColor;
}

export interface Square {
  row: number;
  col: number;
  piece: Piece | null;
}

export type GameMode =
  | 'HumanVsComputer'
  | 'ComputerVsComputer'
  | 'HumanVsHuman'
  | 'Puzzle';

export type PuzzleStage = 'setup' | 'playing';
export type PuzzleSubMode = 'HumanVsComputer' | 'ComputerVsComputer' | 'HumanVsHuman';

export type Difficulty = 1 | 2 | 3 | 4 | 5;

export interface GameStatus {
  status: 'Playing' | 'Checkmate' | 'Stalemate';
  winner?: PieceColor | null;
  reason?: string | null;
}

export interface HistoryRecord {
  from_row: number;
  from_col: number;
  to_row: number;
  to_col: number;
  piece: Piece;
  captured: Piece | null;
  note: string | null;
}

export interface MoveCoords {
  from_row: number;
  from_col: number;
  to_row: number;
  to_col: number;
}

export interface SearchStats {
  depth: number;
  nodes: number;
  time_ms: number;
  score?: number;
}

export interface SearchResult {
  move: MoveCoords;
  stats: SearchStats;
}

export interface EngineConfig {
  mate_score: number;
  tt_size_mb: number;
  nnue_weight: number;
  threads: number;
  [key: string]: number | string | undefined;
}

/**
 * Returns default search threads based on user rule: system hardware concurrency divided by 4 (minimum 1)
 */
export function getDefaultThreads(): number {
  const hardware = typeof navigator !== 'undefined' ? navigator.hardwareConcurrency || 4 : 4;
  return Math.max(1, Math.floor(hardware / 4));
}

export const DEFAULT_ENGINE_CONFIG: EngineConfig = {
  mate_score: 300000,
  tt_size_mb: 512,
  nnue_weight: 100,
  threads: getDefaultThreads(),
};

