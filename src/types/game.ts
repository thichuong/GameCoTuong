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

export type GameMode = 'HumanVsComputer' | 'ComputerVsComputer' | 'HumanVsHuman';

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
}

export interface SearchResult {
  move: MoveCoords;
  stats: SearchStats;
}

export interface EngineConfig {
  pruning_method: number;
  pruning_multiplier: number;
  mate_score: number;
  eval_mode: number;
  nnue_weight: number;
  [key: string]: number;
}

export const DEFAULT_ENGINE_CONFIG: EngineConfig = {
  pruning_method: 1,
  pruning_multiplier: 1.0,
  mate_score: 30000,
  eval_mode: 2,
  nnue_weight: 100,
};

