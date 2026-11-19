import type { BoardViewMode, PieceColor, Square } from '../../types/game.js';
import {
  BOARD_SIZE,
  getVisualCoords,
  getLogicCoords,
} from './boardCoords.js';

export const BOARD_CENTER = BOARD_SIZE / 2; // 277.0
export const WOOD_MIN = 8.0;
export const WOOD_MAX = BOARD_SIZE - 8.0; // 546.0
export const PIECE_RADIUS = 23.5;

export interface CameraParams {
  tiltDeg: number;
  cameraDistance: number;
  zoom: number;
  offsetY: number;
  slabThickness: number;
  pieceHeight: number;
  shadowSpread: number;
}

export interface ProjectedPoint {
  x: number;
  y: number;
  scale: number;
  depth: number;
}

export const VIEW_MODE_CAMERAS: Record<BoardViewMode, CameraParams> = {
  '2d': {
    tiltDeg: 0,
    cameraDistance: 1400,
    zoom: 1.0,
    offsetY: 0,
    slabThickness: 0,
    pieceHeight: 0,
    shadowSpread: 0,
  },
  '2.5d': {
    tiltDeg: 25,
    cameraDistance: 1280,
    zoom: 0.87,
    offsetY: 6,
    slabThickness: 40,
    pieceHeight: 13,
    shadowSpread: 0.75,
  },
  '3d': {
    tiltDeg: 39,
    cameraDistance: 1050,
    zoom: 0.81,
    offsetY: 16,
    slabThickness: 48,
    pieceHeight: 17,
    shadowSpread: 1.0,
  },
};

/**
 * Returns true if the camera requires perspective/3D rendering rather than flat 2D.
 */
export function isPerspectiveCamera(cam: CameraParams): boolean {
  return (
    cam.tiltDeg > 0.05 ||
    Math.abs(cam.zoom - 1) > 0.001 ||
    cam.slabThickness > 0.1 ||
    cam.pieceHeight > 0.1
  );
}

/**
 * Linearly interpolates between two camera configurations for smooth view transitions.
 */
export function lerpCameraParams(
  from: CameraParams,
  to: CameraParams,
  t: number
): CameraParams {
  const u = Math.max(0, Math.min(1, t));
  const lerp = (a: number, b: number) => a + (b - a) * u;
  return {
    tiltDeg: lerp(from.tiltDeg, to.tiltDeg),
    cameraDistance: lerp(from.cameraDistance, to.cameraDistance),
    zoom: lerp(from.zoom, to.zoom),
    offsetY: lerp(from.offsetY, to.offsetY),
    slabThickness: lerp(from.slabThickness, to.slabThickness),
    pieceHeight: lerp(from.pieceHeight, to.pieceHeight),
    shadowSpread: lerp(from.shadowSpread, to.shadowSpread),
  };
}

/**
 * Projects a logical board coordinate (bx, by) at elevation z onto 2D canvas coordinates.
 */
export function projectBoardPoint(
  bx: number,
  by: number,
  z: number,
  cam: CameraParams
): ProjectedPoint {
  const rad = (cam.tiltDeg * Math.PI) / 180;
  const sinT = Math.sin(rad);
  const cosT = Math.cos(rad);
  const lx = bx - BOARD_CENTER;
  const ly = by - BOARD_CENTER;
  const f = cam.cameraDistance * cam.zoom;
  const yCam = ly * cosT - z * sinT;
  const depth = Math.max(10, cam.cameraDistance - ly * sinT - z * cosT);
  const scale = f / depth;

  return {
    x: BOARD_CENTER + lx * scale,
    y: BOARD_CENTER + cam.offsetY + yCam * scale,
    scale,
    depth,
  };
}

/**
 * Exact algebraic inverse of projectBoardPoint onto a horizontal plane at elevation z.
 */
export function unprojectToBoard(
  sx: number,
  sy: number,
  z: number,
  cam: CameraParams
): { x: number; y: number } | null {
  const rad = (cam.tiltDeg * Math.PI) / 180;
  const sinT = Math.sin(rad);
  const cosT = Math.cos(rad);
  const f = cam.cameraDistance * cam.zoom;
  const u = sx - BOARD_CENTER;
  const v = sy - (BOARD_CENTER + cam.offsetY);
  const denom = f * cosT + v * sinT;

  if (Math.abs(denom) < 1e-4) {
    return null;
  }

  const ly = (v * (cam.cameraDistance - z * cosT) + f * z * sinT) / denom;
  const depth = cam.cameraDistance - ly * sinT - z * cosT;
  if (depth <= 10) {
    return null;
  }

  const lx = (u * depth) / f;
  return {
    x: lx + BOARD_CENTER,
    y: ly + BOARD_CENTER,
  };
}

/**
 * Maps a logical row (0..9) to visual top-to-bottom row index (0 = top/far, 9 = bottom/near).
 */
export function getVisualRow(row: number, side: PieceColor): number {
  return side === 'black' ? row : 9 - row;
}

export interface PickBoardSquareOptions {
  prioritizePieces?: boolean;
  selectedSquare?: [number, number] | null;
  excludeSquare?: [number, number] | null;
}

/**
 * Two-phase 3D hit-testing:
 * 1) Checks piece top faces and cylindrical sidewalls front-to-back.
 * 2) Falls back to unprojecting onto the board plane (z = 0) and snapping to the grid.
 */
export function pickBoardSquare(
  sx: number,
  sy: number,
  side: PieceColor,
  cam: CameraParams,
  board: Square[],
  options?: PickBoardSquareOptions
): [number, number] | null {
  if (!isPerspectiveCamera(cam)) {
    return getLogicCoords(sx, sy, side);
  }

  const prioritizePieces = options?.prioritizePieces !== false;
  const selectedSquare = options?.selectedSquare ?? null;
  const excludeSquare = options?.excludeSquare ?? null;

  if (prioritizePieces) {
    const cosT = Math.max(0.25, Math.cos((cam.tiltDeg * Math.PI) / 180));
    const occupiedFrontToBack = board
      .filter((sq) => {
        if (!sq.piece) return false;
        if (
          excludeSquare &&
          excludeSquare[0] === sq.row &&
          excludeSquare[1] === sq.col
        ) {
          return false;
        }
        return true;
      })
      .sort((a, b) => getVisualRow(b.row, side) - getVisualRow(a.row, side));

    for (const sq of occupiedFrontToBack) {
      const isSelected =
        selectedSquare !== null &&
        selectedSquare[0] === sq.row &&
        selectedSquare[1] === sq.col;
      const liftZ = isSelected ? cam.pieceHeight * 0.65 : 0;
      const { x: bx, y: by } = getVisualCoords(sq.row, sq.col, side);
      const basePt = projectBoardPoint(bx, by, liftZ, cam);
      const topPt = projectBoardPoint(bx, by, liftZ + cam.pieceHeight, cam);

      const rxTop = PIECE_RADIUS * topPt.scale;
      const ryTop = Math.max(4, rxTop * cosT);
      const normX = (sx - topPt.x) / rxTop;
      const normY = (sy - topPt.y) / ryTop;
      if (normX * normX + normY * normY <= 1.04) {
        return [sq.row, sq.col];
      }

      const rxBase = PIECE_RADIUS * basePt.scale;
      const ryBase = Math.max(4, rxBase * cosT);
      if (
        Math.abs(sx - basePt.x) <= rxBase &&
        sy >= topPt.y &&
        sy <= basePt.y + ryBase
      ) {
        return [sq.row, sq.col];
      }
    }
  }

  const boardPt = unprojectToBoard(sx, sy, 0, cam);
  if (!boardPt) {
    return null;
  }
  return getLogicCoords(boardPt.x, boardPt.y, side);
}
