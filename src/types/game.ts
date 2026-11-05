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
  val_pawn: number;
  val_advisor: number;
  val_elephant: number;
  val_horse: number;
  val_cannon: number;
  val_rook: number;
  val_king: number;
  score_hash_move: number;
  score_capture_base: number;
  score_killer_move: number;
  score_history_max: number;
  pruning_method: number;
  pruning_multiplier: number;
  mate_score: number;
  eval_mode: number;
  nnue_weight: number;
  [key: string]: number;
}

export const DEFAULT_ENGINE_CONFIG: EngineConfig = {
  val_pawn: 30,
  val_advisor: 120,
  val_elephant: 120,
  val_horse: 300,
  val_cannon: 300,
  val_rook: 600,
  val_king: 10000,
  score_hash_move: 3000000,
  score_capture_base: 1000000,
  score_killer_move: 900000,
  score_history_max: 800000,
  pruning_method: 0,
  pruning_multiplier: 1.0,
  mate_score: 30000,
  eval_mode: 0,
  nnue_weight: 50,
};

