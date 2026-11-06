import type {
  EngineConfig,
  MoveCoords,
  PieceColor,
  PieceType,
  SearchResult,
} from '../types/game.js';
import { DEFAULT_ENGINE_CONFIG } from '../types/game.js';

export const L1_INPUT_SIZE = 10_530; // 9 palace squares * 13 HalfKP piece types * 90 squares
export const L1_OUTPUT_SIZE = 192;
export const PSQT_BUCKETS = 8;
export const L2_INPUT_SIZE = 384;
export const L2_OUTPUT_SIZE = 32;
export const L3_INPUT_SIZE = 32;
export const L3_OUTPUT_SIZE = 32;

const MATE_THRESHOLD = 15_000;
const INF_SCORE = 500_000;

const VAL_KING = 10_000;
const VAL_ROOK = 600;
const VAL_CANNON = 300;
const VAL_HORSE = 300;
const VAL_ELEPHANT = 120;
const VAL_ADVISOR = 120;
const VAL_PAWN = 30;

const SCORE_HASH_MOVE = 3_000_000;
const SCORE_CAPTURE_BASE = 1_000_000;
const SCORE_KILLER_MOVE = 900_000;
const SCORE_HISTORY_MAX = 800_000;

export type ColorIdx = 0 | 1; // 0 = Red, 1 = Black
export type PieceTypeIdx = 0 | 1 | 2 | 3 | 4 | 5 | 6; // General=0, Advisor=1, Elephant=2, Horse=3, Chariot=4, Cannon=5, Soldier=6

export interface EnginePiece {
  pieceType: PieceTypeIdx;
  color: ColorIdx;
}

export interface EngineMove {
  fromSq: number;
  toSq: number;
  score: number;
}

const PST_PAWN: number[][] = [
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, -10, 0, -10, 0, 0, 0],
  [-2, 0, -2, 0, 6, 0, -2, 0, -2],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [10, 10, 10, 10, 10, 10, 10, 10, 10],
  [30, 40, 40, 50, 60, 50, 40, 40, 30],
  [40, 50, 60, 70, 80, 70, 60, 50, 40],
  [50, 60, 70, 80, 80, 80, 70, 60, 50],
  [60, 70, 80, 90, 100, 90, 80, 70, 60],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
];

const PST_ADVISOR: number[][] = [
  [0, 0, 0, 10, 0, 10, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
];

const PST_ELEPHANT: number[][] = [
  [0, 0, 10, 0, 0, 0, 10, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 10, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
];

const PST_HORSE: number[][] = [
  [-4, -8, -4, -10, -10, -10, -4, -8, -4],
  [-4, 8, 4, 0, 0, 0, 4, 8, -4],
  [4, 4, 12, 15, 15, 15, 12, 4, -4],
  [5, 8, 15, 20, 20, 20, 15, 8, 5],
  [10, 20, 20, 25, 25, 25, 20, 20, 10],
  [10, 25, 20, 30, 30, 30, 20, 25, 10],
  [10, 20, 25, 35, 35, 35, 25, 20, 10],
  [10, 20, 20, 10, 10, 10, 20, 20, 10],
  [5, 10, 10, 10, 10, 10, 10, 10, 5],
  [5, 5, 5, 5, 5, 5, 5, 5, 5],
];

const PST_ROOK: number[][] = [
  [10, 10, 10, 10, 10, 10, 10, 10, 10],
  [20, 30, 20, 30, 20, 30, 20, 30, 20],
  [10, 20, 20, 20, 20, 20, 20, 20, 10],
  [10, 20, 20, 20, 20, 20, 20, 20, 10],
  [10, 30, 20, 20, 20, 20, 20, 30, 10],
  [10, 40, 30, 30, 30, 30, 30, 40, 10],
  [20, 40, 40, 40, 40, 40, 40, 40, 20],
  [20, 40, 40, 40, 45, 40, 40, 40, 20],
  [20, 40, 40, 40, 50, 40, 40, 40, 20],
  [10, 20, 20, 20, 20, 20, 20, 20, 10],
];

const PST_CANNON: number[][] = [
  [0, 0, 2, 4, 4, 4, 2, 0, 0],
  [0, 10, 0, 0, 0, 0, 0, 10, 0],
  [0, 0, 10, 0, 20, 0, 10, 0, 0],
  [0, 0, 10, 0, 0, 0, 10, 0, 0],
  [0, 10, 10, 10, 10, 10, 10, 10, 0],
  [0, 5, 5, 5, 5, 5, 5, 5, 0],
  [0, 5, 5, 5, 5, 5, 5, 5, 0],
  [10, 10, 10, 10, 10, 10, 10, 10, 10],
  [20, 20, 20, 20, 20, 20, 20, 20, 20],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
];

const PST_KING: number[][] = [
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 10, 0, 0, 0, 0],
  [0, 0, 0, 0, -10, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
];

function getPstValue(pt: PieceTypeIdx, r: number, c: number): number {
  switch (pt) {
    case 0:
      return PST_KING[r]?.[c] ?? 0;
    case 1:
      return PST_ADVISOR[r]?.[c] ?? 0;
    case 2:
      return PST_ELEPHANT[r]?.[c] ?? 0;
    case 3:
      return PST_HORSE[r]?.[c] ?? 0;
    case 4:
      return PST_ROOK[r]?.[c] ?? 0;
    case 5:
      return PST_CANNON[r]?.[c] ?? 0;
    case 6:
      return PST_PAWN[r]?.[c] ?? 0;
  }
}

export function palaceIndex(sq: number): number {
  switch (sq) {
    case 3:
      return 0;
    case 4:
      return 1;
    case 5:
      return 2;
    case 12:
      return 3;
    case 13:
      return 4;
    case 14:
      return 5;
    case 21:
      return 6;
    case 22:
      return 7;
    case 23:
      return 8;
    default:
      return 1;
  }
}

export function halfkpPieceTypeIdx(
  isEnemy: boolean,
  pieceType: PieceTypeIdx
): number | null {
  if (isEnemy) {
    return 6 + pieceType;
  }
  if (pieceType === 0) {
    return null; // Friendly General is the conditioning king
  }
  return pieceType - 1; // Advisor=0..Soldier=5
}

export function halfkpFeatureIdxRed(
  kPalaceIdx: number,
  isBlackPiece: boolean,
  pieceType: PieceTypeIdx,
  sq: number
): number | null {
  const pTypeIdx = halfkpPieceTypeIdx(isBlackPiece, pieceType);
  if (pTypeIdx === null) return null;
  const sqClamped = Math.min(89, Math.max(0, sq));
  return kPalaceIdx * 1170 + pTypeIdx * 90 + sqClamped;
}

export function halfkpFeatureIdxBlack(
  kPalaceIdx: number,
  isRedPiece: boolean,
  pieceType: PieceTypeIdx,
  sq: number
): number | null {
  const pTypeIdx = halfkpPieceTypeIdx(isRedPiece, pieceType);
  if (pTypeIdx === null) return null;
  const flippedSq = 89 - Math.min(89, Math.max(0, sq));
  return kPalaceIdx * 1170 + pTypeIdx * 90 + flippedSq;
}

export interface NnueWeightsV3 {
  w1: Int16Array; // Flat [L1_INPUT_SIZE * L1_OUTPUT_SIZE]
  b1: Int16Array; // [L1_OUTPUT_SIZE]
  wPsqt: Int16Array; // Flat [L1_INPUT_SIZE * PSQT_BUCKETS]
  w2Col: Int8Array; // Pre-transposed column-major [L2_INPUT_SIZE * L2_OUTPUT_SIZE]
  b2: Int32Array; // [L2_OUTPUT_SIZE]
  w3Col: Int8Array; // Pre-transposed column-major [L3_INPUT_SIZE * L3_OUTPUT_SIZE]
  b3: Int32Array; // [L3_OUTPUT_SIZE]
  w4: Int16Array; // [L3_OUTPUT_SIZE]
  b4: number;
}

let cachedWeights: NnueWeightsV3 | null = null;

function q16(val: number, scale = 256.0): number {
  return Math.max(-32767, Math.min(32767, Math.trunc(val * scale)));
}

function q8(val: number): number {
  return Math.max(-127, Math.min(127, Math.trunc(val * 64.0)));
}

export function getNnueWeights(): NnueWeightsV3 {
  if (cachedWeights) return cachedWeights;

  const w1 = new Int16Array(L1_INPUT_SIZE * L1_OUTPUT_SIZE);
  const b1 = new Int16Array(L1_OUTPUT_SIZE);
  const wPsqt = new Int16Array(L1_INPUT_SIZE * PSQT_BUCKETS);
  const w2Col = new Int8Array(L2_INPUT_SIZE * L2_OUTPUT_SIZE);
  const b2 = new Int32Array(L2_OUTPUT_SIZE);
  const w3Col = new Int8Array(L3_INPUT_SIZE * L3_OUTPUT_SIZE);
  const b3 = new Int32Array(L3_OUTPUT_SIZE);
  const w4 = new Int16Array(L3_OUTPUT_SIZE);
  const b4 = 0;

  b1[8] = q16(-0.55);
  b1[9] = q16(-0.55);
  b1[10] = q16(-0.55);
  b1[11] = q16(-0.75);
  b1[16] = q16(0.0);
  b1[17] = q16(-1.25);
  b1[18] = q16(-0.7);

  const friendlyTypes: PieceTypeIdx[] = [1, 2, 3, 4, 5, 6];
  const enemyTypes: PieceTypeIdx[] = [0, 1, 2, 3, 4, 5, 6];

  for (let k = 0; k < 9; k++) {
    const kr = Math.floor(k / 3);
    const kc = 3 + (k % 3);

    // 1. Friendly HalfKP features
    for (let ptypeIdx = 0; ptypeIdx < friendlyTypes.length; ptypeIdx++) {
      const pt = friendlyTypes[ptypeIdx]!;
      for (let sq = 0; sq < 90; sq++) {
        const r = Math.floor(sq / 9);
        const c = sq % 9;
        const feat = k * 1170 + ptypeIdx * 90 + sq;
        const pst = getPstValue(pt, r, c);

        const psqtBase = feat * PSQT_BUCKETS;
        for (let bucket = 0; bucket < PSQT_BUCKETS; bucket++) {
          const phase = bucket / (PSQT_BUCKETS - 1);
          let baseCp = 0;
          if (pt === 1 || pt === 2) baseCp = 120 + 10 * (1 - phase);
          else if (pt === 3) baseCp = 265 + 30 * (1 - phase);
          else if (pt === 4) baseCp = 600;
          else if (pt === 5) baseCp = 270 + 30 * phase;
          else if (pt === 6) baseCp = 30 + 25 * (1 - phase);

          let devBonus = 0;
          if (pt === 4 && r === 0 && (c === 1 || c === 7)) devBonus = 16;
          else if (pt === 4 && r === 0 && (c === 0 || c === 8)) devBonus = -6;
          else if (pt === 3 && r === 2 && (c === 2 || c === 6)) devBonus = 12;
          else if (pt === 1 && r === 1 && c === 4) devBonus = 12;

          const kingHomeBonus =
            kr === 0 && kc === 4 ? 3 * phase : kr >= 1 ? -4 * phase : 0;
          const shieldBonus = c === kc && r > kr && r <= 3 ? 8 : 0;
          wPsqt[psqtBase + bucket] = q16(
            baseCp + pst + devBonus + kingHomeBonus + shieldBonus,
            16.0
          );
        }

        const w1Base = feat * L1_OUTPUT_SIZE;
        if (pt === 1) {
          w1[w1Base + 8] = q16(0.55);
        } else if (pt === 2) {
          w1[w1Base + 9] = q16(r === 2 && c === 4 ? 0.65 : 0.5);
        } else if (pt === 4) {
          w1[w1Base + 10] = q16(0.55);
          if (r >= 5) w1[w1Base + 12] = q16(0.35);
        } else if (pt === 3) {
          if (r >= 3) w1[w1Base + 11] = q16(0.45);
          if (r >= 6 && c >= 2 && c <= 6) w1[w1Base + 12] = q16(0.4);
        } else if (pt === 5) {
          w1[w1Base + 11] = q16(0.45);
          if (c === 4) w1[w1Base + 13] = q16(0.35);
        } else if (pt === 6) {
          if (r >= 5) w1[w1Base + 14] = q16(0.3);
        }

        if (c === kc && r > kr && r <= 4) {
          w1[w1Base + 17] = q16(0.625);
          w1[w1Base + 18] = q16(0.7);
        }

        for (let idx = 24; idx < L1_OUTPUT_SIZE; idx++) {
          const h = (Math.imul(feat, 2654435761) ^ Math.imul(idx, 97)) & 0xff;
          w1[w1Base + idx] = q16(((h - 128) / 128) * 0.04);
        }
      }
    }

    // 2. Enemy HalfKP features
    for (let eIdx = 0; eIdx < enemyTypes.length; eIdx++) {
      const pt = enemyTypes[eIdx]!;
      const ptypeIdx = 6 + eIdx;
      for (let sq = 0; sq < 90; sq++) {
        const r = Math.floor(sq / 9);
        const c = sq % 9;
        const feat = k * 1170 + ptypeIdx * 90 + sq;
        const w1Base = feat * L1_OUTPUT_SIZE;

        if (pt === 5 && (r === kr || c === kc) && (r !== kr || c !== kc)) {
          w1[w1Base + 16] = q16(0.78);
          if (c === kc && r > kr) {
            w1[w1Base + 17] = q16(0.78);
          }
        } else if (
          pt === 4 &&
          (r === kr || c === kc) &&
          (r !== kr || c !== kc)
        ) {
          w1[w1Base + 16] = q16(0.65);
          if (c === kc && r > kr) {
            w1[w1Base + 18] = q16(0.7);
          }
        }

        if (r <= 3 && c >= 2 && c <= 6) {
          if (pt === 4 || pt === 3 || pt === 5) {
            w1[w1Base + 19] = q16(0.4);
          } else if (pt === 6) {
            w1[w1Base + 19] = q16(0.3);
          }
        }

        if (c === kc && r > kr && r <= 4 && pt !== 5) {
          w1[w1Base + 17] = q16(0.625);
          w1[w1Base + 18] = q16(0.7);
        }

        for (let idx = 24; idx < L1_OUTPUT_SIZE; idx++) {
          const h = (Math.imul(feat, 2654435761) ^ Math.imul(idx, 131)) & 0xff;
          w1[w1Base + idx] = q16(((h - 128) / 128) * 0.04);
        }
      }
    }
  }

  const setW2 = (row: number, col: number, val: number) => {
    w2Col[col * L2_OUTPUT_SIZE + row] = q8(val);
  };

  setW2(0, 8, 0.55);
  setW2(0, 9, 0.55);
  setW2(0, 10, 0.65);
  setW2(0, 11, 0.5);
  setW2(0, 12, 0.6);
  setW2(0, 13, 0.45);
  setW2(0, 14, 0.5);

  setW2(1, L1_OUTPUT_SIZE + 16, 1.45);
  setW2(1, L1_OUTPUT_SIZE + 17, -1.45);
  setW2(1, L1_OUTPUT_SIZE + 18, -1.1);
  setW2(1, L1_OUTPUT_SIZE + 19, 0.85);

  setW2(2, 16, 1.45);
  setW2(2, 17, -1.45);
  setW2(2, 18, -1.1);
  setW2(2, 19, 0.85);

  for (let j = 3; j < L2_OUTPUT_SIZE; j++) {
    for (let i = 0; i < L2_INPUT_SIZE; i++) {
      const h = (Math.imul(j, 313) ^ Math.imul(i, 1000003)) & 0xff;
      setW2(j, i, ((h - 128) / 128) * 0.08);
    }
  }

  for (let k = 0; k < L3_OUTPUT_SIZE; k++) {
    for (let j = 0; j < L3_INPUT_SIZE; j++) {
      const val =
        k === j
          ? 1.0
          : (((Math.imul(k, 73) ^ Math.imul(j, 199)) & 0xff) - 128) /
            128 *
            0.04;
      w3Col[j * L3_OUTPUT_SIZE + k] = q8(val);
    }
  }

  w4[0] = q16(65.0, 16.0);
  w4[1] = q16(135.0, 16.0);
  w4[2] = q16(-135.0, 16.0);
  for (let k = 3; k < L3_OUTPUT_SIZE; k++) {
    const sign = k % 2 === 0 ? 1.0 : -1.0;
    w4[k] = q16(sign * 8.0, 16.0);
  }

  cachedWeights = { w1, b1, wPsqt, w2Col, b2, w3Col, b3, w4, b4 };
  return cachedWeights;
}

export class NnueAccumulatorV3 {
  red = new Int16Array(L1_OUTPUT_SIZE);
  black = new Int16Array(L1_OUTPUT_SIZE);
  psqtRed = new Int32Array(PSQT_BUCKETS);
  psqtBlack = new Int32Array(PSQT_BUCKETS);

  clone(): NnueAccumulatorV3 {
    const copy = new NnueAccumulatorV3();
    copy.red.set(this.red);
    copy.black.set(this.black);
    copy.psqtRed.set(this.psqtRed);
    copy.psqtBlack.set(this.psqtBlack);
    return copy;
  }

  refreshRed(
    grid: (EnginePiece | null)[],
    kRedPalaceIdx: number,
    weights: NnueWeightsV3
  ): void {
    this.red.set(weights.b1);
    this.psqtRed.fill(0);
    for (let sq = 0; sq < 90; sq++) {
      const piece = grid[sq];
      if (piece) {
        this.addPieceRed(piece, sq, kRedPalaceIdx, weights);
      }
    }
  }

  refreshBlack(
    grid: (EnginePiece | null)[],
    kBlackPalaceIdx: number,
    weights: NnueWeightsV3
  ): void {
    this.black.set(weights.b1);
    this.psqtBlack.fill(0);
    for (let sq = 0; sq < 90; sq++) {
      const piece = grid[sq];
      if (piece) {
        this.addPieceBlack(piece, sq, kBlackPalaceIdx, weights);
      }
    }
  }

  addPieceRed(
    piece: EnginePiece,
    sq: number,
    kRed: number,
    weights: NnueWeightsV3
  ): void {
    const feat = halfkpFeatureIdxRed(
      kRed,
      piece.color === 1,
      piece.pieceType,
      sq
    );
    if (feat === null) return;
    const w1Base = feat * L1_OUTPUT_SIZE;
    for (let i = 0; i < L1_OUTPUT_SIZE; i++) {
      this.red[i] = this.red[i]! + weights.w1[w1Base + i]!;
    }
    const psqtBase = feat * PSQT_BUCKETS;
    for (let b = 0; b < PSQT_BUCKETS; b++) {
      this.psqtRed[b] = this.psqtRed[b]! + weights.wPsqt[psqtBase + b]!;
    }
  }

  removePieceRed(
    piece: EnginePiece,
    sq: number,
    kRed: number,
    weights: NnueWeightsV3
  ): void {
    const feat = halfkpFeatureIdxRed(
      kRed,
      piece.color === 1,
      piece.pieceType,
      sq
    );
    if (feat === null) return;
    const w1Base = feat * L1_OUTPUT_SIZE;
    for (let i = 0; i < L1_OUTPUT_SIZE; i++) {
      this.red[i] = this.red[i]! - weights.w1[w1Base + i]!;
    }
    const psqtBase = feat * PSQT_BUCKETS;
    for (let b = 0; b < PSQT_BUCKETS; b++) {
      this.psqtRed[b] = this.psqtRed[b]! - weights.wPsqt[psqtBase + b]!;
    }
  }

  addPieceBlack(
    piece: EnginePiece,
    sq: number,
    kBlack: number,
    weights: NnueWeightsV3
  ): void {
    const feat = halfkpFeatureIdxBlack(
      kBlack,
      piece.color === 0,
      piece.pieceType,
      sq
    );
    if (feat === null) return;
    const w1Base = feat * L1_OUTPUT_SIZE;
    for (let i = 0; i < L1_OUTPUT_SIZE; i++) {
      this.black[i] = this.black[i]! + weights.w1[w1Base + i]!;
    }
    const psqtBase = feat * PSQT_BUCKETS;
    for (let b = 0; b < PSQT_BUCKETS; b++) {
      this.psqtBlack[b] = this.psqtBlack[b]! + weights.wPsqt[psqtBase + b]!;
    }
  }

  removePieceBlack(
    piece: EnginePiece,
    sq: number,
    kBlack: number,
    weights: NnueWeightsV3
  ): void {
    const feat = halfkpFeatureIdxBlack(
      kBlack,
      piece.color === 0,
      piece.pieceType,
      sq
    );
    if (feat === null) return;
    const w1Base = feat * L1_OUTPUT_SIZE;
    for (let i = 0; i < L1_OUTPUT_SIZE; i++) {
      this.black[i] = this.black[i]! - weights.w1[w1Base + i]!;
    }
    const psqtBase = feat * PSQT_BUCKETS;
    for (let b = 0; b < PSQT_BUCKETS; b++) {
      this.psqtBlack[b] = this.psqtBlack[b]! - weights.wPsqt[psqtBase + b]!;
    }
  }

  addPiece(
    piece: EnginePiece,
    sq: number,
    kRed: number,
    kBlack: number,
    weights: NnueWeightsV3
  ): void {
    this.addPieceRed(piece, sq, kRed, weights);
    this.addPieceBlack(piece, sq, kBlack, weights);
  }

  removePiece(
    piece: EnginePiece,
    sq: number,
    kRed: number,
    kBlack: number,
    weights: NnueWeightsV3
  ): void {
    this.removePieceRed(piece, sq, kRed, weights);
    this.removePieceBlack(piece, sq, kBlack, weights);
  }
}

const l2AccScratch = new Int32Array(L2_OUTPUT_SIZE);
const l3AccScratch = new Int32Array(L3_OUTPUT_SIZE);

export function evaluateForward(
  activeAcc: Int16Array,
  oppAcc: Int16Array,
  weights: NnueWeightsV3
): number {
  l2AccScratch.set(weights.b2);

  // Sparse non-zero SCReLU forward pass for active perspective
  for (let i = 0; i < L1_OUTPUT_SIZE; i++) {
    const val = activeAcc[i]!;
    if (val > 0) {
      const clamped = val > 256 ? 256 : val;
      const act = (clamped * clamped) >> 8;
      if (act !== 0) {
        const colBase = i * L2_OUTPUT_SIZE;
        for (let j = 0; j < L2_OUTPUT_SIZE; j++) {
          l2AccScratch[j] =
            l2AccScratch[j]! + act * weights.w2Col[colBase + j]!;
        }
      }
    }
  }

  // Sparse non-zero SCReLU forward pass for opponent perspective
  for (let i = 0; i < L1_OUTPUT_SIZE; i++) {
    const val = oppAcc[i]!;
    if (val > 0) {
      const clamped = val > 256 ? 256 : val;
      const act = (clamped * clamped) >> 8;
      if (act !== 0) {
        const colBase = (L1_OUTPUT_SIZE + i) * L2_OUTPUT_SIZE;
        for (let j = 0; j < L2_OUTPUT_SIZE; j++) {
          l2AccScratch[j] =
            l2AccScratch[j]! + act * weights.w2Col[colBase + j]!;
        }
      }
    }
  }

  // Layer 2 -> Layer 3 with sparse zero-skipping
  l3AccScratch.set(weights.b3);
  for (let j = 0; j < L2_OUTPUT_SIZE; j++) {
    const raw = l2AccScratch[j]! >> 6;
    if (raw > 0) {
      const l2Act = raw > 127 ? 127 : raw;
      const colBase = j * L3_OUTPUT_SIZE;
      for (let k = 0; k < L3_OUTPUT_SIZE; k++) {
        l3AccScratch[k] =
          l3AccScratch[k]! + l2Act * weights.w3Col[colBase + k]!;
      }
    }
  }

  // Layer 3 -> Output
  let finalSum = weights.b4;
  for (let k = 0; k < L3_OUTPUT_SIZE; k++) {
    const raw = l3AccScratch[k]! >> 6;
    if (raw > 0) {
      const l3Act = raw > 127 ? 127 : raw;
      finalSum += l3Act * weights.w4[k]!;
    }
  }

  return Math.trunc(finalSum / 16);
}

// Zobrist hashing keys
const ZOBRIST_PIECES: BigUint64Array = (() => {
  const arr = new BigUint64Array(2 * 7 * 90);
  let state = 0x9e3779b97f4a7c15n;
  for (let i = 0; i < arr.length; i++) {
    state ^= state >> 12n;
    state ^= (state << 25n) & 0xffffffffffffffffn;
    state ^= state >> 27n;
    arr[i] = (state * 0x2545f4914f6cdd1dn) & 0xffffffffffffffffn;
  }
  return arr;
})();
const ZOBRIST_SIDE = 0xf1357aea2e62a9c5n;

function pieceKey(color: ColorIdx, pt: PieceTypeIdx, sq: number): bigint {
  return ZOBRIST_PIECES[(color * 7 + pt) * 90 + sq]!;
}

export class EngineBoard {
  grid: (EnginePiece | null)[] = new Array(90).fill(null);
  kingSq: [number, number] = [4, 85];
  pieceCount = 0;
  zobristHash = 0n;
  nnueAcc = new NnueAccumulatorV3();

  static fromFen(fen: string): { board: EngineBoard; turn: ColorIdx } {
    const b = new EngineBoard();
    b.clear();
    const parts = fen.trim().split(/\s+/);
    const rows = (parts[0] ?? '').split('/');
    for (let rIdx = 0; rIdx < 10 && rIdx < rows.length; rIdx++) {
      const r = 9 - rIdx;
      let c = 0;
      for (const ch of rows[rIdx]!) {
        if (ch >= '1' && ch <= '9') {
          c += Number(ch);
        } else {
          const color: ColorIdx = ch === ch.toUpperCase() ? 0 : 1;
          const lower = ch.toLowerCase();
          const pt: PieceTypeIdx =
            lower === 'k'
              ? 0
              : lower === 'a'
                ? 1
                : lower === 'b' || lower === 'e'
                  ? 2
                  : lower === 'n' || lower === 'h'
                    ? 3
                    : lower === 'r'
                      ? 4
                      : lower === 'c'
                        ? 5
                        : 6;
          b.rawAddPiece(r * 9 + c, { pieceType: pt, color });
          c++;
        }
      }
    }
    const turn: ColorIdx = parts[1] === 'b' ? 1 : 0;
    if (turn === 1) {
      b.zobristHash ^= ZOBRIST_SIDE;
    }
    b.refreshNnue();
    return { board: b, turn };
  }

  static newStarting(): EngineBoard {
    return EngineBoard.fromFen(
      'rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w'
    ).board;
  }

  clear(): void {
    this.grid.fill(null);
    this.kingSq = [4, 85];
    this.pieceCount = 0;
    this.zobristHash = 0n;
    this.nnueAcc.refreshRed(this.grid, 1, getNnueWeights());
    this.nnueAcc.refreshBlack(this.grid, 1, getNnueWeights());
  }

  rawAddPiece(sq: number, piece: EnginePiece): void {
    this.grid[sq] = piece;
    this.pieceCount++;
    if (piece.pieceType === 0) {
      this.kingSq[piece.color] = sq;
    }
    this.zobristHash ^= pieceKey(piece.color, piece.pieceType, sq);
  }

  rawRemovePiece(sq: number, piece: EnginePiece): void {
    this.grid[sq] = null;
    this.pieceCount--;
    this.zobristHash ^= pieceKey(piece.color, piece.pieceType, sq);
  }

  setPiece(sq: number, piece: EnginePiece | null): void {
    const existing = this.grid[sq];
    if (existing) {
      this.rawRemovePiece(sq, existing);
    }
    if (piece) {
      this.rawAddPiece(sq, piece);
    }
    this.refreshNnue();
  }

  getRedKingPalaceIdx(): number {
    return palaceIndex(this.kingSq[0]);
  }

  getBlackKingPalaceIdx(): number {
    return palaceIndex(89 - this.kingSq[1]);
  }

  refreshNnue(): void {
    const weights = getNnueWeights();
    this.nnueAcc.refreshRed(this.grid, this.getRedKingPalaceIdx(), weights);
    this.nnueAcc.refreshBlack(this.grid, this.getBlackKingPalaceIdx(), weights);
  }

  hasMajorPieces(color: ColorIdx): boolean {
    for (let sq = 0; sq < 90; sq++) {
      const p = this.grid[sq];
      if (
        p &&
        p.color === color &&
        (p.pieceType === 3 || p.pieceType === 4 || p.pieceType === 5)
      ) {
        return true;
      }
    }
    return false;
  }

  applyNullMove(): void {
    this.zobristHash ^= ZOBRIST_SIDE;
  }

  applyMove(mv: EngineMove): EnginePiece | null {
    const piece = this.grid[mv.fromSq]!;
    const captured = this.grid[mv.toSq];
    const weights = getNnueWeights();
    const kRed = this.getRedKingPalaceIdx();
    const kBlack = this.getBlackKingPalaceIdx();

    this.rawRemovePiece(mv.fromSq, piece);
    if (captured) {
      this.rawRemovePiece(mv.toSq, captured);
    }
    this.rawAddPiece(mv.toSq, piece);
    this.zobristHash ^= ZOBRIST_SIDE;

    if (piece.pieceType === 0) {
      if (piece.color === 0) {
        const newKRed = palaceIndex(mv.toSq);
        this.nnueAcc.refreshRed(this.grid, newKRed, weights);
        if (captured) {
          this.nnueAcc.removePieceBlack(captured, mv.toSq, kBlack, weights);
        }
        this.nnueAcc.removePieceBlack(piece, mv.fromSq, kBlack, weights);
        this.nnueAcc.addPieceBlack(piece, mv.toSq, kBlack, weights);
      } else {
        const newKBlack = palaceIndex(89 - mv.toSq);
        this.nnueAcc.refreshBlack(this.grid, newKBlack, weights);
        if (captured) {
          this.nnueAcc.removePieceRed(captured, mv.toSq, kRed, weights);
        }
        this.nnueAcc.removePieceRed(piece, mv.fromSq, kRed, weights);
        this.nnueAcc.addPieceRed(piece, mv.toSq, kRed, weights);
      }
    } else {
      if (captured) {
        this.nnueAcc.removePiece(captured, mv.toSq, kRed, kBlack, weights);
      }
      this.nnueAcc.removePiece(piece, mv.fromSq, kRed, kBlack, weights);
      this.nnueAcc.addPiece(piece, mv.toSq, kRed, kBlack, weights);
    }

    return captured;
  }

  undoMove(mv: EngineMove, captured: EnginePiece | null): void {
    const piece = this.grid[mv.toSq]!;
    const weights = getNnueWeights();
    const kRed = this.getRedKingPalaceIdx();
    const kBlack = this.getBlackKingPalaceIdx();

    this.zobristHash ^= ZOBRIST_SIDE;
    this.rawRemovePiece(mv.toSq, piece);
    this.rawAddPiece(mv.fromSq, piece);
    if (captured) {
      this.rawAddPiece(mv.toSq, captured);
    }

    if (piece.pieceType === 0) {
      if (piece.color === 0) {
        const newKRed = palaceIndex(mv.fromSq);
        this.nnueAcc.refreshRed(this.grid, newKRed, weights);
        this.nnueAcc.removePieceBlack(piece, mv.toSq, kBlack, weights);
        this.nnueAcc.addPieceBlack(piece, mv.fromSq, kBlack, weights);
        if (captured) {
          this.nnueAcc.addPieceBlack(captured, mv.toSq, kBlack, weights);
        }
      } else {
        const newKBlack = palaceIndex(89 - mv.fromSq);
        this.nnueAcc.refreshBlack(this.grid, newKBlack, weights);
        this.nnueAcc.removePieceRed(piece, mv.toSq, kRed, weights);
        this.nnueAcc.addPieceRed(piece, mv.fromSq, kRed, weights);
        if (captured) {
          this.nnueAcc.addPieceRed(captured, mv.toSq, kRed, weights);
        }
      }
    } else {
      this.nnueAcc.removePiece(piece, mv.toSq, kRed, kBlack, weights);
      this.nnueAcc.addPiece(piece, mv.fromSq, kRed, kBlack, weights);
      if (captured) {
        this.nnueAcc.addPiece(captured, mv.toSq, kRed, kBlack, weights);
      }
    }
  }
}

/**
 * Pure NNUE Evaluation (HalfKP v3: 8-Bucket Game-Phase PSQT + Deep SCReLU Network).
 * Zero handcrafted evaluation terms.
 */
export function evaluateNnue(
  board: EngineBoard,
  weights: NnueWeightsV3 = getNnueWeights()
): number {
  const bucket = Math.min(
    PSQT_BUCKETS - 1,
    Math.floor((Math.max(0, board.pieceCount - 2) * PSQT_BUCKETS) / 30)
  );
  const psqtScore = Math.trunc(
    (board.nnueAcc.psqtRed[bucket]! - board.nnueAcc.psqtBlack[bucket]!) / 16
  );

  const redScore = evaluateForward(
    board.nnueAcc.red,
    board.nnueAcc.black,
    weights
  );
  const blackScore = evaluateForward(
    board.nnueAcc.black,
    board.nnueAcc.red,
    weights
  );
  const positionalScore = Math.trunc((redScore - blackScore) / 2);

  return psqtScore + positionalScore;
}

// Rules & Attack detection
export function isFlyingGeneral(board: EngineBoard): boolean {
  const rSq = board.kingSq[0];
  const bSq = board.kingSq[1];
  if (rSq % 9 !== bSq % 9) return false;
  for (let sq = rSq + 9; sq < bSq; sq += 9) {
    if (board.grid[sq] !== null) return false;
  }
  return true;
}

const ORTHO_DIRS: [number, number][] = [
  [-1, 0],
  [1, 0],
  [0, -1],
  [0, 1],
];
const HORSE_STEPS: [number, number, number, number][] = [
  [-2, -1, -1, 0],
  [-2, 1, -1, 0],
  [2, -1, 1, 0],
  [2, 1, 1, 0],
  [-1, -2, 0, -1],
  [1, -2, 0, -1],
  [-1, 2, 0, 1],
  [1, 2, 0, 1],
];
const ADVISOR_STEPS: [number, number][] = [
  [-1, -1],
  [-1, 1],
  [1, -1],
  [1, 1],
];
const ELEPHANT_STEPS: [number, number, number, number][] = [
  [-2, -2, -1, -1],
  [-2, 2, -1, 1],
  [2, -2, 1, -1],
  [2, 2, 1, 1],
];

export function isInCheck(board: EngineBoard, color: ColorIdx): boolean {
  const kSq = board.kingSq[color];
  const kr = Math.floor(kSq / 9);
  const kc = kSq % 9;
  const enemy: ColorIdx = color === 0 ? 1 : 0;

  // 1. Orthogonal Chariot / Cannon / Flying General
  for (const [dr, dc] of ORTHO_DIRS) {
    let r = kr + dr;
    let c = kc + dc;
    let blockers = 0;
    while (r >= 0 && r < 10 && c >= 0 && c < 9) {
      const p = board.grid[r * 9 + c];
      if (p !== null) {
        if (blockers === 0) {
          if (p.color === enemy && (p.pieceType === 4 || p.pieceType === 0)) {
            return true;
          }
          blockers = 1;
        } else {
          if (p.color === enemy && p.pieceType === 5) {
            return true;
          }
          break;
        }
      }
      r += dr;
      c += dc;
    }
  }

  // 2. Enemy Horse attacks (reverse leg check from enemy horse square to King)
  for (const [dr, dc] of [
    [-2, -1],
    [-2, 1],
    [2, -1],
    [2, 1],
    [-1, -2],
    [1, -2],
    [-1, 2],
    [1, 2],
  ] as [number, number][]) {
    const hr = kr + dr;
    const hc = kc + dc;
    if (hr >= 0 && hr < 10 && hc >= 0 && hc < 9) {
      const p = board.grid[hr * 9 + hc];
      if (p !== null && p.color === enemy && p.pieceType === 3) {
        const legR = hr - Math.trunc(dr / 2);
        const legC = hc - Math.trunc(dc / 2);
        if (board.grid[legR * 9 + legC] === null) {
          return true;
        }
      }
    }
  }

  // 3. Enemy Soldier attacks
  const forwardDr = color === 0 ? 1 : -1;
  const sr = kr + forwardDr;
  if (sr >= 0 && sr < 10) {
    const p = board.grid[sr * 9 + kc];
    if (p !== null && p.color === enemy && p.pieceType === 6) return true;
  }
  for (const dc of [-1, 1]) {
    const sc = kc + dc;
    if (sc >= 0 && sc < 9) {
      const p = board.grid[kr * 9 + sc];
      if (p !== null && p.color === enemy && p.pieceType === 6) return true;
    }
  }

  return false;
}

enum TTFlag {
  Exact = 0,
  LowerBound = 1,
  UpperBound = 2,
}

interface TTEntry {
  hash: bigint;
  bestMove: EngineMove | null;
  score: number;
  depth: number;
  flag: TTFlag;
}

const TT_SIZE = 1 << 16;
const TT_MASK = BigInt(TT_SIZE - 1);

export class PikafishNnueEngine {
  config: EngineConfig;
  private tt: (TTEntry | null)[] = new Array(TT_SIZE).fill(null);
  private killerMoves: [EngineMove | null, EngineMove | null][] = Array.from(
    { length: 64 },
    () => [null, null]
  );
  private countermoveTable: (EngineMove | null)[] = new Array(90 * 90).fill(
    null
  );
  private historyTable = new Int32Array(90 * 90);
  private staticEvalStack = new Int32Array(128);
  private moveStack: (EngineMove | null)[] = new Array(128).fill(null);
  private historyStack: bigint[] = [];
  private lmrTable: Uint8Array = new Uint8Array(64 * 64);
  private nodesSearched = 0;
  private startTime = 0;
  private timeLimitMs: number | null = null;

  constructor(config?: EngineConfig | null) {
    this.config = config
      ? { ...DEFAULT_ENGINE_CONFIG, ...config }
      : { ...DEFAULT_ENGINE_CONFIG };
    this.initLmrTable();
  }

  updateConfig(config: EngineConfig): void {
    this.config = { ...this.config, ...config };
  }

  private initLmrTable(): void {
    for (let d = 0; d < 64; d++) {
      for (let m = 0; m < 64; m++) {
        if (d >= 2 && m >= 2) {
          const r = 0.75 + (Math.log(d) * Math.log(m)) / 2.15;
          this.lmrTable[d * 64 + m] = Math.min(d - 1, Math.trunc(r));
        } else {
          this.lmrTable[d * 64 + m] = 0;
        }
      }
    }
  }

  private getPieceValue(pt: PieceTypeIdx): number {
    switch (pt) {
      case 0:
        return VAL_KING;
      case 1:
        return VAL_ADVISOR;
      case 2:
        return VAL_ELEPHANT;
      case 3:
        return VAL_HORSE;
      case 4:
        return VAL_ROOK;
      case 5:
        return VAL_CANNON;
      case 6:
        return VAL_PAWN;
    }
  }

  private evalForTurn(board: EngineBoard, turn: ColorIdx): number {
    const raw = Math.trunc(
      (evaluateNnue(board) * (this.config.nnue_weight ?? 100)) / 100
    );
    return turn === 0 ? raw : -raw;
  }

  private checkTime(): boolean {
    if (this.timeLimitMs !== null && (this.nodesSearched & 511) === 0) {
      if (performance.now() - this.startTime > this.timeLimitMs) {
        return true;
      }
    }
    return false;
  }

  private isRepetition(hash: bigint): boolean {
    let count = 0;
    for (let i = this.historyStack.length - 1; i >= 0; i--) {
      if (this.historyStack[i] === hash) {
        count++;
        if (count >= 2) return true;
      }
    }
    return false;
  }

  private getCountermove(ply: number): EngineMove | null {
    if (ply <= 0) return null;
    const prev = this.moveStack[ply - 1];
    if (!prev) return null;
    return this.countermoveTable[prev.fromSq * 90 + prev.toSq] ?? null;
  }

  private updateQuietHistories(
    depth: number,
    ply: number,
    bestMv: EngineMove,
    quietMovesTried: EngineMove[]
  ): void {
    const killers = this.killerMoves[depth];
    if (killers) {
      if (
        !killers[0] ||
        killers[0].fromSq !== bestMv.fromSq ||
        killers[0].toSq !== bestMv.toSq
      ) {
        killers[1] = killers[0];
        killers[0] = bestMv;
      }
    }

    if (ply > 0) {
      const prev = this.moveStack[ply - 1];
      if (prev) {
        this.countermoveTable[prev.fromSq * 90 + prev.toSq] = bestMv;
      }
    }

    const bonus = Math.min(1600, depth * depth * 16 + 32 * depth);
    this.applyHistoryGravity(bestMv, bonus);
    for (const qMv of quietMovesTried) {
      if (qMv.fromSq !== bestMv.fromSq || qMv.toSq !== bestMv.toSq) {
        this.applyHistoryGravity(qMv, -bonus);
      }
    }
  }

  private applyHistoryGravity(mv: EngineMove, bonus: number): void {
    const idx = mv.fromSq * 90 + mv.toSq;
    const cur = this.historyTable[idx]!;
    this.historyTable[idx] =
      cur + bonus - Math.trunc((cur * Math.abs(bonus)) / 16384);
  }

  generatePseudoMoves(
    board: EngineBoard,
    turn: ColorIdx,
    ttMove: EngineMove | null,
    countermove: EngineMove | null,
    depth: number,
    onlyCaptures: boolean
  ): EngineMove[] {
    const moves: EngineMove[] = [];
    const killers = this.killerMoves[Math.min(63, depth)] ?? [null, null];

    const pushMove = (fromSq: number, toSq: number) => {
      const target = board.grid[toSq];
      if (target !== null) {
        if (target.color === turn) return;
      } else if (onlyCaptures) {
        return;
      }

      let score: number;
      if (ttMove && ttMove.fromSq === fromSq && ttMove.toSq === toSq) {
        score = SCORE_HASH_MOVE;
      } else if (target !== null) {
        const victimVal = this.getPieceValue(target.pieceType);
        const attackerVal = this.getPieceValue(board.grid[fromSq]!.pieceType);
        score = SCORE_CAPTURE_BASE + victimVal * 10 - attackerVal;
      } else if (
        killers[0] &&
        killers[0].fromSq === fromSq &&
        killers[0].toSq === toSq
      ) {
        score = SCORE_KILLER_MOVE;
      } else if (
        killers[1] &&
        killers[1].fromSq === fromSq &&
        killers[1].toSq === toSq
      ) {
        score = SCORE_KILLER_MOVE - 10_000;
      } else if (
        countermove &&
        countermove.fromSq === fromSq &&
        countermove.toSq === toSq
      ) {
        score = SCORE_KILLER_MOVE - 20_000;
      } else {
        score = Math.min(
          SCORE_HISTORY_MAX,
          this.historyTable[fromSq * 90 + toSq]!
        );
      }

      moves.push({ fromSq, toSq, score });
    };

    for (let sq = 0; sq < 90; sq++) {
      const p = board.grid[sq];
      if (!p || p.color !== turn) continue;
      const r = Math.floor(sq / 9);
      const c = sq % 9;

      switch (p.pieceType) {
        case 0: {
          // General
          const minR = turn === 0 ? 0 : 7;
          const maxR = turn === 0 ? 2 : 9;
          for (const [dr, dc] of ORTHO_DIRS) {
            const tr = r + dr;
            const tc = c + dc;
            if (tr >= minR && tr <= maxR && tc >= 3 && tc <= 5) {
              pushMove(sq, tr * 9 + tc);
            }
          }
          break;
        }
        case 1: {
          // Advisor
          const minR = turn === 0 ? 0 : 7;
          const maxR = turn === 0 ? 2 : 9;
          for (const [dr, dc] of ADVISOR_STEPS) {
            const tr = r + dr;
            const tc = c + dc;
            if (tr >= minR && tr <= maxR && tc >= 3 && tc <= 5) {
              pushMove(sq, tr * 9 + tc);
            }
          }
          break;
        }
        case 2: {
          // Elephant
          const minR = turn === 0 ? 0 : 5;
          const maxR = turn === 0 ? 4 : 9;
          for (const [dr, dc, er, ec] of ELEPHANT_STEPS) {
            const tr = r + dr;
            const tc = c + dc;
            if (
              tr >= minR &&
              tr <= maxR &&
              tc >= 0 &&
              tc < 9 &&
              board.grid[(r + er) * 9 + (c + ec)] === null
            ) {
              pushMove(sq, tr * 9 + tc);
            }
          }
          break;
        }
        case 3: {
          // Horse
          for (const [dr, dc, lr, lc] of HORSE_STEPS) {
            const tr = r + dr;
            const tc = c + dc;
            if (
              tr >= 0 &&
              tr < 10 &&
              tc >= 0 &&
              tc < 9 &&
              board.grid[(r + lr) * 9 + (c + lc)] === null
            ) {
              pushMove(sq, tr * 9 + tc);
            }
          }
          break;
        }
        case 4: {
          // Chariot
          for (const [dr, dc] of ORTHO_DIRS) {
            let tr = r + dr;
            let tc = c + dc;
            while (tr >= 0 && tr < 10 && tc >= 0 && tc < 9) {
              const targetSq = tr * 9 + tc;
              pushMove(sq, targetSq);
              if (board.grid[targetSq] !== null) break;
              tr += dr;
              tc += dc;
            }
          }
          break;
        }
        case 5: {
          // Cannon
          for (const [dr, dc] of ORTHO_DIRS) {
            let tr = r + dr;
            let tc = c + dc;
            let jumped = false;
            while (tr >= 0 && tr < 10 && tc >= 0 && tc < 9) {
              const targetSq = tr * 9 + tc;
              const occ = board.grid[targetSq];
              if (!jumped) {
                if (occ === null) {
                  if (!onlyCaptures) pushMove(sq, targetSq);
                } else {
                  jumped = true;
                }
              } else if (occ !== null) {
                if (occ.color !== turn) pushMove(sq, targetSq);
                break;
              }
              tr += dr;
              tc += dc;
            }
          }
          break;
        }
        case 6: {
          // Soldier
          const fwd = turn === 0 ? r + 1 : r - 1;
          if (fwd >= 0 && fwd < 10) {
            pushMove(sq, fwd * 9 + c);
          }
          const crossedRiver = turn === 0 ? r >= 5 : r <= 4;
          if (crossedRiver) {
            if (c > 0) pushMove(sq, r * 9 + (c - 1));
            if (c < 8) pushMove(sq, r * 9 + (c + 1));
          }
          break;
        }
      }
    }

    moves.sort((a, b) => b.score - a.score);
    return moves;
  }

  private quiescence(
    board: EngineBoard,
    alpha: number,
    beta: number,
    turn: ColorIdx
  ): number {
    this.nodesSearched++;

    const ttIdx = Number(board.zobristHash & TT_MASK);
    const entry = this.tt[ttIdx];
    if (entry && entry.hash === board.zobristHash) {
      if (entry.flag === TTFlag.Exact) return entry.score;
      if (entry.flag === TTFlag.LowerBound && entry.score >= beta)
        return entry.score;
      if (entry.flag === TTFlag.UpperBound && entry.score <= alpha)
        return entry.score;
    }

    const standPat = this.evalForTurn(board, turn);
    if (standPat >= beta) return beta;
    if (standPat > alpha) alpha = standPat;

    const captures = this.generatePseudoMoves(board, turn, null, null, 0, true);
    const opp: ColorIdx = turn === 0 ? 1 : 0;

    for (const mv of captures) {
      const capPiece = board.grid[mv.toSq];
      if (capPiece) {
        const capVal = this.getPieceValue(capPiece.pieceType);
        if (standPat + capVal + 180 < alpha) continue;
      }

      const captured = board.applyMove(mv);
      if (isInCheck(board, turn) || isFlyingGeneral(board)) {
        board.undoMove(mv, captured);
        continue;
      }

      const score = -this.quiescence(board, -beta, -alpha, opp);
      board.undoMove(mv, captured);

      if (score >= beta) return beta;
      if (score > alpha) alpha = score;
    }

    return alpha;
  }

  private alphaBeta(
    board: EngineBoard,
    alpha: number,
    beta: number,
    depth: number,
    turn: ColorIdx,
    ply: number,
    excludedMove: EngineMove | null
  ): number | null {
    this.nodesSearched++;
    if (this.checkTime()) return null;

    const isPv = beta - alpha > 1;
    const opp: ColorIdx = turn === 0 ? 1 : 0;

    if (ply > 0) {
      const mat = this.config.mate_score - ply;
      if (-mat > alpha) alpha = -mat;
      if (mat < beta) beta = mat;
      if (alpha >= beta) return alpha;
    }

    const hash = board.zobristHash;
    if (ply > 0 && this.isRepetition(hash)) {
      return 0;
    }

    this.historyStack.push(hash);

    const ttIdx = Number(hash & TT_MASK);
    const ttEntry =
      excludedMove === null && this.tt[ttIdx]?.hash === hash
        ? this.tt[ttIdx]!
        : null;

    if (
      ttEntry &&
      ttEntry.depth >= depth &&
      (!isPv || ttEntry.flag === TTFlag.Exact)
    ) {
      if (ttEntry.flag === TTFlag.Exact) {
        this.historyStack.pop();
        return ttEntry.score;
      }
      if (ttEntry.flag === TTFlag.LowerBound && ttEntry.score >= beta) {
        this.historyStack.pop();
        return ttEntry.score;
      }
      if (ttEntry.flag === TTFlag.UpperBound && ttEntry.score <= alpha) {
        this.historyStack.pop();
        return ttEntry.score;
      }
    }

    if (depth <= 0) {
      const q = this.quiescence(board, alpha, beta, turn);
      this.historyStack.pop();
      return q;
    }

    const inCheck = isInCheck(board, turn);
    let staticEval = -INF_SCORE;
    let evalScore = -INF_SCORE;
    let improving = false;

    if (!inCheck) {
      staticEval = this.evalForTurn(board, turn);
      this.staticEvalStack[ply] = staticEval;
      evalScore = staticEval;
      if (ttEntry && Math.abs(ttEntry.score) < MATE_THRESHOLD) {
        if (
          ttEntry.flag === TTFlag.Exact ||
          (ttEntry.flag === TTFlag.LowerBound && ttEntry.score > staticEval) ||
          (ttEntry.flag === TTFlag.UpperBound && ttEntry.score < staticEval)
        ) {
          evalScore = ttEntry.score;
        }
      }
      improving =
        ply >= 2 &&
        this.staticEvalStack[ply - 2]! > -INF_SCORE &&
        staticEval > this.staticEvalStack[ply - 2]!;
    } else {
      this.staticEvalStack[ply] = -INF_SCORE;
    }

    // Pikafish / Fairy-Stockfish Non-PV Pre-Move-Loop Pruning
    if (!isPv && !inCheck && excludedMove === null) {
      // 1. Razoring
      if (depth <= 2 && Math.abs(alpha) < MATE_THRESHOLD) {
        if (evalScore + 210 * depth <= alpha) {
          const qScore = this.quiescence(board, alpha, beta, turn);
          if (qScore <= alpha) {
            this.historyStack.pop();
            return qScore;
          }
        }
      }

      // 2. Reverse Futility Pruning (RFP)
      if (depth <= 6 && Math.abs(beta) < MATE_THRESHOLD) {
        const rfpMargin = 85 * depth - (improving ? 40 : 0);
        if (evalScore - rfpMargin >= beta) {
          this.historyStack.pop();
          return Math.trunc((evalScore + beta) / 2);
        }
      }

      // 3. Null Move Pruning (NMP)
      if (
        depth >= 3 &&
        evalScore >= beta &&
        Math.abs(beta) < MATE_THRESHOLD &&
        board.hasMajorPieces(turn)
      ) {
        const evalBonus = Math.min(
          3,
          Math.max(0, Math.trunc((evalScore - beta) / 180))
        );
        const r = Math.min(depth - 1, 3 + Math.trunc(depth / 4) + evalBonus);
        board.applyNullMove();
        this.moveStack[ply] = null;
        const nmpRes = this.alphaBeta(
          board,
          -beta,
          -beta + 1,
          depth - r,
          opp,
          ply + 1,
          null
        );
        board.applyNullMove();
        if (nmpRes === null) {
          this.historyStack.pop();
          return null;
        }
        const nmpScore = -nmpRes;
        if (nmpScore >= beta) {
          this.historyStack.pop();
          return nmpScore >= MATE_THRESHOLD ? beta : nmpScore;
        }
      }

      // 4. ProbCut
      if (depth >= 5 && Math.abs(beta) < MATE_THRESHOLD) {
        const probBeta = beta + 200;
        const probRes = this.alphaBeta(
          board,
          -probBeta,
          -probBeta + 1,
          depth - 4,
          opp,
          ply + 1,
          null
        );
        if (probRes === null) {
          this.historyStack.pop();
          return null;
        }
        if (-probRes >= probBeta) {
          this.historyStack.pop();
          return beta;
        }
      }
    }

    const ttMove = ttEntry?.bestMove ?? null;

    // 5. Internal Iterative Reductions (IIR)
    if (depth >= 4 && ttMove === null && excludedMove === null) {
      depth -= 1;
    }

    // 6. Singular Extension
    let singularExt = 0;
    if (
      depth >= 6 &&
      excludedMove === null &&
      ttMove !== null &&
      ttEntry &&
      ttEntry.depth >= depth - 3 &&
      Math.abs(ttEntry.score) < MATE_THRESHOLD &&
      (ttEntry.flag === TTFlag.Exact || ttEntry.flag === TTFlag.LowerBound)
    ) {
      const sBeta = ttEntry.score - (2 * depth + 50);
      const sRes = this.alphaBeta(
        board,
        sBeta - 1,
        sBeta,
        Math.trunc((depth - 1) / 2),
        turn,
        ply,
        ttMove
      );
      if (sRes !== null && sRes < sBeta) {
        singularExt = 1;
      }
    }

    const countermove = this.getCountermove(ply);
    const moves = this.generatePseudoMoves(
      board,
      turn,
      ttMove,
      countermove,
      depth,
      false
    );

    const lmpLimit =
      !inCheck && depth <= 6
        ? improving
          ? 4 + depth * depth + depth * 2
          : Math.trunc(((4 + depth * depth) * 2) / 3)
        : moves.length;

    let bestScore = -INF_SCORE;
    let bestMoveThisNode: EngineMove | null = null;
    let ttFlag = TTFlag.UpperBound;
    let legalCount = 0;
    const quietTried: EngineMove[] = [];

    for (let i = 0; i < moves.length; i++) {
      const mv = moves[i]!;
      if (
        excludedMove &&
        excludedMove.fromSq === mv.fromSq &&
        excludedMove.toSq === mv.toSq
      ) {
        continue;
      }

      const isCapture = board.grid[mv.toSq] !== null;

      // Quiet Move Pruning (LMP + Futility Pruning)
      if (!isPv && !inCheck && !isCapture && bestScore > -MATE_THRESHOLD) {
        if (legalCount >= lmpLimit) continue;
        if (depth <= 4 && legalCount > 0) {
          const hist = this.historyTable[mv.fromSq * 90 + mv.toSq]!;
          const margin = 100 + 85 * depth + Math.trunc(hist / 128);
          if (staticEval + margin <= alpha) continue;
        }
      }

      const captured = board.applyMove(mv);
      if (isInCheck(board, turn) || isFlyingGeneral(board)) {
        board.undoMove(mv, captured);
        continue;
      }

      legalCount++;
      if (!isCapture && quietTried.length < 64) {
        quietTried.push(mv);
      }
      this.moveStack[ply] = mv;

      // Late Move Reductions (LMR)
      let reduction = 0;
      if (depth >= 3 && legalCount >= 3 && !inCheck && !isCapture) {
        const dIdx = Math.min(63, depth);
        const mIdx = Math.min(63, legalCount);
        let r = this.lmrTable[dIdx * 64 + mIdx]!;
        if (!isPv) r += 1;
        if (!improving) r += 1;
        if (mv.score >= SCORE_KILLER_MOVE - 20_000) r -= 1;
        const histAdj = Math.max(
          -2,
          Math.min(
            2,
            Math.trunc(this.historyTable[mv.fromSq * 90 + mv.toSq]! / 4096)
          )
        );
        r -= histAdj;
        reduction = Math.max(0, Math.min(depth - 2, r));
      }

      const isTtMove =
        ttMove !== null &&
        ttMove.fromSq === mv.fromSq &&
        ttMove.toSq === mv.toSq;
      const ext = (inCheck ? 1 : 0) + (isTtMove ? singularExt : 0);
      const fullDepth = depth - 1 + ext;

      let childRes: number | null;
      if (legalCount === 1) {
        childRes = this.alphaBeta(
          board,
          -beta,
          -alpha,
          fullDepth,
          opp,
          ply + 1,
          null
        );
      } else {
        const reducedDepth = Math.max(1, fullDepth - reduction);
        childRes = this.alphaBeta(
          board,
          -alpha - 1,
          -alpha,
          reducedDepth,
          opp,
          ply + 1,
          null
        );
        if (childRes !== null && -childRes > alpha && reduction > 0) {
          childRes = this.alphaBeta(
            board,
            -alpha - 1,
            -alpha,
            fullDepth,
            opp,
            ply + 1,
            null
          );
        }
        if (childRes !== null && -childRes > alpha && -childRes < beta) {
          childRes = this.alphaBeta(
            board,
            -beta,
            -alpha,
            fullDepth,
            opp,
            ply + 1,
            null
          );
        }
      }

      board.undoMove(mv, captured);

      if (childRes === null) {
        this.historyStack.pop();
        return null;
      }

      const score = -childRes;
      if (score > bestScore) {
        bestScore = score;
        bestMoveThisNode = mv;
      }
      if (score > alpha) {
        alpha = score;
        ttFlag = TTFlag.Exact;
      }
      if (alpha >= beta) {
        if (!isCapture) {
          this.updateQuietHistories(depth, ply, mv, quietTried);
        }
        ttFlag = TTFlag.LowerBound;
        break;
      }
    }

    if (legalCount === 0) {
      this.historyStack.pop();
      return -(this.config.mate_score - ply);
    }

    if (excludedMove === null) {
      this.tt[ttIdx] = {
        hash,
        bestMove: bestMoveThisNode,
        score: bestScore,
        depth,
        flag: ttFlag,
      };
    }

    this.historyStack.pop();
    return bestScore;
  }

  searchFen(
    fen: string,
    timeLimitMs: number,
    maxDepth?: number | null
  ): SearchResult | null {
    const { board, turn } = EngineBoard.fromFen(fen);
    return this.searchBoard(board, turn, timeLimitMs, maxDepth);
  }

  searchBoard(
    board: EngineBoard,
    turn: ColorIdx,
    timeLimitMs: number,
    maxDepth?: number | null
  ): SearchResult | null {
    this.nodesSearched = 0;
    this.startTime = performance.now();
    this.timeLimitMs = timeLimitMs > 0 ? timeLimitMs : null;
    const softLimit = this.timeLimitMs !== null ? timeLimitMs * 0.6 : null;
    const targetDepth = maxDepth ?? 64;
    const opp: ColorIdx = turn === 0 ? 1 : 0;

    for (let i = 0; i < this.historyTable.length; i++) {
      this.historyTable[i] = Math.trunc(this.historyTable[i]! / 2);
    }

    const rootMoves = this.generatePseudoMoves(
      board,
      turn,
      null,
      null,
      1,
      false
    ).filter((mv) => {
      const cap = board.applyMove(mv);
      const legal = !isInCheck(board, turn) && !isFlyingGeneral(board);
      board.undoMove(mv, cap);
      return legal;
    });

    if (rootMoves.length === 0) return null;

    let bestMove: EngineMove = rootMoves[0]!;
    let finalDepth = 1;
    let prevScore: number | null = null;

    for (let d = 1; d <= targetDepth; d++) {
      if (softLimit !== null && performance.now() - this.startTime > softLimit) {
        break;
      }

      let alpha = -INF_SCORE;
      let beta = INF_SCORE;
      let delta = 35;

      if (prevScore !== null && d >= 4 && Math.abs(prevScore) < MATE_THRESHOLD) {
        alpha = prevScore - delta;
        beta = prevScore + delta;
      }

      let timedOut = false;

      while (true) {
        const alphaOrig = alpha;
        const betaOrig = beta;
        let iterBestScore = -INF_SCORE;
        let iterBestMove: EngineMove | null = null;

        // Sort bestMove first
        rootMoves.sort((a, b) => {
          const aIsBest =
            a.fromSq === bestMove.fromSq && a.toSq === bestMove.toSq ? 1 : 0;
          const bIsBest =
            b.fromSq === bestMove.fromSq && b.toSq === bestMove.toSq ? 1 : 0;
          if (aIsBest !== bIsBest) return bIsBest - aIsBest;
          return b.score - a.score;
        });

        for (let i = 0; i < rootMoves.length; i++) {
          const mv = rootMoves[i]!;
          const cap = board.applyMove(mv);
          this.moveStack[0] = mv;
          this.historyStack = [board.zobristHash];

          let res: number | null;
          if (i === 0) {
            res = this.alphaBeta(board, -beta, -alpha, d - 1, opp, 1, null);
          } else {
            res = this.alphaBeta(
              board,
              -alpha - 1,
              -alpha,
              d - 1,
              opp,
              1,
              null
            );
            if (res !== null && -res > alpha && -res < beta) {
              res = this.alphaBeta(board, -beta, -alpha, d - 1, opp, 1, null);
            }
          }

          board.undoMove(mv, cap);

          if (res === null) {
            timedOut = true;
            break;
          }

          const score = -res;
          if (score > iterBestScore) {
            iterBestScore = score;
            iterBestMove = mv;
          }
          if (score > alpha) {
            alpha = score;
          }
        }

        if (timedOut) break;

        if (iterBestScore <= alphaOrig) {
          if (alphaOrig <= -INF_SCORE) break;
          alpha = Math.max(-INF_SCORE, alphaOrig - delta);
          delta += Math.trunc(delta / 2);
          continue;
        }
        if (iterBestScore >= betaOrig) {
          if (betaOrig >= INF_SCORE) break;
          beta = Math.min(INF_SCORE, betaOrig + delta);
          delta += Math.trunc(delta / 2);
          continue;
        }

        if (iterBestMove) {
          bestMove = iterBestMove;
          finalDepth = d;
          prevScore = iterBestScore;
        }
        break;
      }

      if (timedOut || this.checkTime()) break;
    }

    const elapsedMs = Math.max(1, Math.round(performance.now() - this.startTime));
    const moveCoords: MoveCoords = {
      from_row: Math.floor(bestMove.fromSq / 9),
      from_col: bestMove.fromSq % 9,
      to_row: Math.floor(bestMove.toSq / 9),
      to_col: bestMove.toSq % 9,
    };

    return {
      move: moveCoords,
      stats: {
        depth: finalDepth,
        nodes: this.nodesSearched,
        time_ms: elapsedMs,
      },
    };
  }
}

export function pieceTypeFromString(t: PieceType): PieceTypeIdx {
  switch (t) {
    case 'General':
      return 0;
    case 'Advisor':
      return 1;
    case 'Elephant':
      return 2;
    case 'Horse':
      return 3;
    case 'Chariot':
      return 4;
    case 'Cannon':
      return 5;
    case 'Soldier':
      return 6;
  }
}

export function pieceTypeToString(pt: PieceTypeIdx): PieceType {
  switch (pt) {
    case 0:
      return 'General';
    case 1:
      return 'Advisor';
    case 2:
      return 'Elephant';
    case 3:
      return 'Horse';
    case 4:
      return 'Chariot';
    case 5:
      return 'Cannon';
    case 6:
      return 'Soldier';
  }
}

export function colorFromString(c: PieceColor): ColorIdx {
  return c === 'red' ? 0 : 1;
}

export function colorToString(c: ColorIdx): PieceColor {
  return c === 0 ? 'red' : 'black';
}

export function boardToFen(board: EngineBoard, turn: ColorIdx): string {
  const rows: string[] = [];
  for (let r = 9; r >= 0; r--) {
    let rowStr = '';
    let empty = 0;
    for (let c = 0; c < 9; c++) {
      const p = board.grid[r * 9 + c];
      if (!p) {
        empty++;
      } else {
        if (empty > 0) {
          rowStr += String(empty);
          empty = 0;
        }
        const code =
          p.pieceType === 0
            ? 'k'
            : p.pieceType === 1
              ? 'a'
              : p.pieceType === 2
                ? 'b'
                : p.pieceType === 3
                  ? 'n'
                  : p.pieceType === 4
                    ? 'r'
                    : p.pieceType === 5
                      ? 'c'
                      : 'p';
        rowStr += p.color === 0 ? code.toUpperCase() : code;
      }
    }
    if (empty > 0) rowStr += String(empty);
    rows.push(rowStr);
  }
  return `${rows.join('/')} ${turn === 0 ? 'w' : 'b'}`;
}

interface InternalMoveRecord {
  from_row: number;
  from_col: number;
  to_row: number;
  to_col: number;
  piece: { type: PieceType; color: PieceColor };
  captured: { type: PieceType; color: PieceColor } | null;
  note: string | null;
  rawCaptured: EnginePiece | null;
}

export class WasmGameState {
  board: EngineBoard;
  turnColor: ColorIdx;
  records: InternalMoveRecord[] = [];
  private helperEngine = new PikafishNnueEngine();

  constructor() {
    this.board = EngineBoard.newStarting();
    this.turnColor = 0;
  }

  free(): void {
    // No-op for GC compatibility
  }

  static from_fen(fen: string): WasmGameState {
    const { board, turn } = EngineBoard.fromFen(fen);
    const state = new WasmGameState();
    state.board = board;
    state.turnColor = turn;
    return state;
  }

  to_fen(): string {
    return boardToFen(this.board, this.turnColor);
  }

  turn(): PieceColor {
    return colorToString(this.turnColor);
  }

  is_in_check(): boolean {
    return isInCheck(this.board, this.turnColor);
  }

  private getAllLegalMovesForTurn(turn: ColorIdx): EngineMove[] {
    const pseudo = this.helperEngine.generatePseudoMoves(
      this.board,
      turn,
      null,
      null,
      0,
      false
    );
    const legal: EngineMove[] = [];
    for (const mv of pseudo) {
      const cap = this.board.applyMove(mv);
      const ok = !isInCheck(this.board, turn) && !isFlyingGeneral(this.board);
      this.board.undoMove(mv, cap);
      if (ok) legal.push(mv);
    }
    return legal;
  }

  status(): {
    status: 'Playing' | 'Checkmate' | 'Stalemate';
    winner: PieceColor | null;
    reason: string | null;
  } {
    const legal = this.getAllLegalMovesForTurn(this.turnColor);
    if (legal.length > 0) {
      return { status: 'Playing', winner: null, reason: null };
    }
    if (isInCheck(this.board, this.turnColor)) {
      const winner: PieceColor = this.turnColor === 0 ? 'black' : 'red';
      return {
        status: 'Checkmate',
        winner,
        reason: 'Chiếu bí (Checkmate)',
      };
    }
    return {
      status: 'Stalemate',
      winner: null,
      reason: 'Hết nước đi (Stalemate)',
    };
  }

  get_board(): {
    row: number;
    col: number;
    piece: { type: PieceType; color: PieceColor } | null;
  }[] {
    const squares: {
      row: number;
      col: number;
      piece: { type: PieceType; color: PieceColor } | null;
    }[] = [];
    for (let row = 0; row < 10; row++) {
      for (let col = 0; col < 9; col++) {
        const p = this.board.grid[row * 9 + col];
        squares.push({
          row,
          col,
          piece: p
            ? {
                type: pieceTypeToString(p.pieceType),
                color: colorToString(p.color),
              }
            : null,
        });
      }
    }
    return squares;
  }

  get_legal_moves(row: number, col: number): [number, number][] {
    const fromSq = row * 9 + col;
    const legal = this.getAllLegalMovesForTurn(this.turnColor);
    return legal
      .filter((m) => m.fromSq === fromSq)
      .map((m) => [Math.floor(m.toSq / 9), m.toSq % 9]);
  }

  make_move(
    from_row: number,
    from_col: number,
    to_row: number,
    to_col: number
  ): {
    success: boolean;
    captured: { type: PieceType; color: PieceColor } | null;
    in_check: boolean;
  } {
    const fromSq = from_row * 9 + from_col;
    const toSq = to_row * 9 + to_col;
    const movingPiece = this.board.grid[fromSq];
    if (!movingPiece || movingPiece.color !== this.turnColor) {
      throw new Error('Illegal move: invalid source piece');
    }

    const legal = this.getAllLegalMovesForTurn(this.turnColor);
    if (!legal.some((m) => m.fromSq === fromSq && m.toSq === toSq)) {
      throw new Error('Illegal move');
    }

    const rawCaptured = this.board.applyMove({ fromSq, toSq, score: 0 });
    this.turnColor = this.turnColor === 0 ? 1 : 0;

    const capturedDto = rawCaptured
      ? {
          type: pieceTypeToString(rawCaptured.pieceType),
          color: colorToString(rawCaptured.color),
        }
      : null;

    this.records.push({
      from_row,
      from_col,
      to_row,
      to_col,
      piece: {
        type: pieceTypeToString(movingPiece.pieceType),
        color: colorToString(movingPiece.color),
      },
      captured: capturedDto,
      note: null,
      rawCaptured,
    });

    return {
      success: true,
      captured: capturedDto,
      in_check: isInCheck(this.board, this.turnColor),
    };
  }

  undo_move(): boolean {
    const last = this.records.pop();
    if (!last) return false;
    const fromSq = last.from_row * 9 + last.from_col;
    const toSq = last.to_row * 9 + last.to_col;
    this.board.undoMove({ fromSq, toSq, score: 0 }, last.rawCaptured);
    this.turnColor = this.turnColor === 0 ? 1 : 0;
    return true;
  }

  get_history(): {
    from_row: number;
    from_col: number;
    to_row: number;
    to_col: number;
    piece: { type: PieceType; color: PieceColor };
    captured: { type: PieceType; color: PieceColor } | null;
    note: string | null;
  }[] {
    return this.records.map((r) => ({
      from_row: r.from_row,
      from_col: r.from_col,
      to_row: r.to_row,
      to_col: r.to_col,
      piece: r.piece,
      captured: r.captured,
      note: r.note,
    }));
  }

  annotate_last_move(note: string): void {
    const last = this.records[this.records.length - 1];
    if (last) {
      last.note = note;
    }
  }

  get_book_move(): MoveCoords | null {
    const fen = this.to_fen();
    if (
      fen === 'rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w'
    ) {
      const candidates: MoveCoords[] = [
        { from_row: 2, from_col: 1, to_row: 2, to_col: 4 },
        { from_row: 2, from_col: 7, to_row: 2, to_col: 4 },
        { from_row: 0, from_col: 2, to_row: 2, to_col: 4 },
        { from_row: 0, from_col: 6, to_row: 2, to_col: 4 },
        { from_row: 3, from_col: 2, to_row: 4, to_col: 2 },
        { from_row: 3, from_col: 6, to_row: 4, to_col: 6 },
      ];
      return candidates[Math.floor(Math.random() * candidates.length)] ?? null;
    }
    if (
      fen === 'rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C2C4/9/RNBAKABNR b'
    ) {
      const candidates: MoveCoords[] = [
        { from_row: 9, from_col: 7, to_row: 7, to_col: 6 },
        { from_row: 9, from_col: 1, to_row: 7, to_col: 2 },
        { from_row: 7, from_col: 7, to_row: 7, to_col: 4 },
      ];
      return candidates[Math.floor(Math.random() * candidates.length)] ?? null;
    }
    if (
      fen === 'rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/4C2C1/9/RNBAKABNR b'
    ) {
      const candidates: MoveCoords[] = [
        { from_row: 9, from_col: 7, to_row: 7, to_col: 6 },
        { from_row: 9, from_col: 1, to_row: 7, to_col: 2 },
        { from_row: 7, from_col: 1, to_row: 7, to_col: 4 },
      ];
      return candidates[Math.floor(Math.random() * candidates.length)] ?? null;
    }
    return null;
  }

  history_len(): number {
    return this.records.length;
  }

  to_json(): string {
    return JSON.stringify({
      fen: this.to_fen(),
      records: this.records,
    });
  }

  static from_json(jsonStr: string): WasmGameState {
    const parsed = JSON.parse(jsonStr) as {
      fen?: string;
      records?: InternalMoveRecord[];
    };
    const state = parsed.fen
      ? WasmGameState.from_fen(parsed.fen)
      : new WasmGameState();
    if (Array.isArray(parsed.records)) {
      state.records = parsed.records;
    }
    return state;
  }
}

export class WasmEngine {
  private inner: PikafishNnueEngine;

  constructor(configJson?: string | null) {
    const parsed = configJson ? (JSON.parse(configJson) as EngineConfig) : null;
    this.inner = new PikafishNnueEngine(parsed);
  }

  free(): void {
    // No-op
  }

  update_config(configJson: string): void {
    const parsed = JSON.parse(configJson) as EngineConfig;
    this.inner.updateConfig(parsed);
  }

  search(
    gameState: WasmGameState,
    timeLimitMs: bigint | number,
    maxDepth?: number | null
  ): SearchResult | null {
    return this.inner.searchFen(
      gameState.to_fen(),
      Number(timeLimitMs),
      maxDepth ?? null
    );
  }
}

