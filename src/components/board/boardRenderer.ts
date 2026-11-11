import type { Piece, PieceColor, Square } from '../../types/game.js';
import {
  BOARD_WIDTH,
  BOARD_HEIGHT,
  PADDING,
  CELL_WIDTH,
  CELL_HEIGHT,
  FIDUCIAL_POINTS,
  getPieceSymbol,
  getVisualCoords,
} from './boardCoords.js';

const cursorCache = new Map<string, string>();

/**
 * Generates a 52x52 PNG data URL of a wooden Xiangqi piece for CSS `cursor: url(...)`
 */
export function getPieceCursorStyle(piece: Piece | null | undefined): string | undefined {
  if (!piece || typeof document === 'undefined') return undefined;
  const key = `${piece.color}-${piece.type}`;
  const cached = cursorCache.get(key);
  if (cached) return cached;

  const canvas = document.createElement('canvas');
  canvas.width = 52;
  canvas.height = 52;
  const ctx = canvas.getContext('2d');
  if (!ctx) return undefined;

  drawPiece(ctx, 26, 26, piece, true);
  const dataUrl = canvas.toDataURL('image/png');
  const styleStr = `url("${dataUrl}") 26 26, pointer`;
  cursorCache.set(key, styleStr);
  return styleStr;
}

export function drawFiducialMark(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  visCol: number
): void {
  const offset = 4.5;
  const len = 9;
  ctx.strokeStyle = '#6B4226';
  ctx.lineWidth = 1.4;

  // Left side L-marks (only if not on left edge)
  if (visCol > 0) {
    ctx.beginPath();
    ctx.moveTo(x - offset - len, y - offset);
    ctx.lineTo(x - offset, y - offset);
    ctx.lineTo(x - offset, y - offset - len);
    ctx.stroke();

    ctx.beginPath();
    ctx.moveTo(x - offset - len, y + offset);
    ctx.lineTo(x - offset, y + offset);
    ctx.lineTo(x - offset, y + offset + len);
    ctx.stroke();
  }

  // Right side L-marks (only if not on right edge)
  if (visCol < 8) {
    ctx.beginPath();
    ctx.moveTo(x + offset + len, y - offset);
    ctx.lineTo(x + offset, y - offset);
    ctx.lineTo(x + offset, y - offset - len);
    ctx.stroke();

    ctx.beginPath();
    ctx.moveTo(x + offset + len, y + offset);
    ctx.lineTo(x + offset, y + offset);
    ctx.lineTo(x + offset, y + offset + len);
    ctx.stroke();
  }
}

export function drawPiece(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  piece: Piece,
  isSelected: boolean
): void {
  const radius = 23.5;

  ctx.save();

  // Elevation shadow or selection aura
  if (isSelected) {
    ctx.shadowBlur = 18;
    ctx.shadowColor = 'rgba(245, 158, 11, 0.9)';
    ctx.shadowOffsetX = 0;
    ctx.shadowOffsetY = 2;
  } else {
    ctx.shadowBlur = 6;
    ctx.shadowColor = 'rgba(25, 15, 5, 0.55)';
    ctx.shadowOffsetX = 1.5;
    ctx.shadowOffsetY = 3;
  }

  // Piece wooden ivory disc
  const discGrad = ctx.createRadialGradient(x - 6, y - 6, 3, x, y, radius);
  if (piece.color === 'red') {
    discGrad.addColorStop(0, '#FFFDF9');
    discGrad.addColorStop(0.75, '#FBF0DC');
    discGrad.addColorStop(1, '#EAD2AC');
  } else {
    discGrad.addColorStop(0, '#FAF4E8');
    discGrad.addColorStop(0.75, '#EFE3CC');
    discGrad.addColorStop(1, '#DBC5A0');
  }

  ctx.beginPath();
  ctx.arc(x, y, radius, 0, Math.PI * 2);
  ctx.fillStyle = discGrad;
  ctx.fill();

  // Reset shadow for crisp strokes
  ctx.shadowBlur = 0;
  ctx.shadowOffsetX = 0;
  ctx.shadowOffsetY = 0;

  // Outer carved rim
  ctx.lineWidth = 2.4;
  ctx.strokeStyle = piece.color === 'red' ? '#B91C1C' : '#1E293B';
  ctx.stroke();

  // Inner engraved ring
  ctx.beginPath();
  ctx.arc(x, y, radius - 3.8, 0, Math.PI * 2);
  ctx.lineWidth = 1.1;
  ctx.strokeStyle = piece.color === 'red' ? '#DC2626' : '#475569';
  ctx.stroke();

  // Selection golden halo
  if (isSelected) {
    ctx.beginPath();
    ctx.arc(x, y, radius + 2.2, 0, Math.PI * 2);
    ctx.lineWidth = 2.6;
    ctx.strokeStyle = '#F59E0B';
    ctx.stroke();
  }

  // Traditional Calligraphy Character
  const symbol = getPieceSymbol(piece.type, piece.color);
  ctx.font = 'bold 23px "Noto Serif", "KaiTi", "SimSun", "STKaiti", serif';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillStyle = piece.color === 'red' ? '#B91C1C' : '#0F172A';
  ctx.fillText(symbol, x, y + 1);

  ctx.restore();
}

export function drawBoardGridAndRiver(
  ctx: CanvasRenderingContext2D,
  playerSide: PieceColor,
  isSetupMode: boolean
): void {
  // Rich handcrafted Rosewood / Hinoki board gradient
  const boardGrad = ctx.createLinearGradient(0, 0, BOARD_WIDTH, BOARD_HEIGHT);
  boardGrad.addColorStop(0, '#EED7B5');
  boardGrad.addColorStop(0.5, '#E4C598');
  boardGrad.addColorStop(1, '#D8B480');
  ctx.fillStyle = boardGrad;
  ctx.fillRect(0, 0, BOARD_WIDTH, BOARD_HEIGHT);

  // Outer heavy frame
  ctx.strokeStyle = '#4A2C16';
  ctx.lineWidth = 3.5;
  ctx.strokeRect(
    PADDING - 8,
    PADDING - 8,
    8 * CELL_WIDTH + 16,
    9 * CELL_HEIGHT + 16
  );

  // Inner grid hairline frame
  ctx.strokeStyle = '#6B4226';
  ctx.lineWidth = 1.6;
  ctx.strokeRect(PADDING, PADDING, 8 * CELL_WIDTH, 9 * CELL_HEIGHT);

  // Subtle river band shading
  ctx.fillStyle = 'rgba(107, 66, 38, 0.06)';
  ctx.fillRect(
    PADDING + 1,
    PADDING + 4 * CELL_HEIGHT + 1,
    8 * CELL_WIDTH - 2,
    CELL_HEIGHT - 2
  );

  // Draw 10 horizontal lines
  for (let r = 0; r < 10; r++) {
    const y = r * CELL_HEIGHT + PADDING;
    ctx.beginPath();
    ctx.moveTo(PADDING, y);
    ctx.lineTo(PADDING + 8 * CELL_WIDTH, y);
    ctx.stroke();
  }

  // Draw 9 vertical lines (broken at the river between r=4 and r=5)
  for (let c = 0; c < 9; c++) {
    const x = c * CELL_WIDTH + PADDING;
    if (c === 0 || c === 8) {
      ctx.beginPath();
      ctx.moveTo(x, PADDING);
      ctx.lineTo(x, PADDING + 9 * CELL_HEIGHT);
      ctx.stroke();
    } else {
      ctx.beginPath();
      ctx.moveTo(x, PADDING);
      ctx.lineTo(x, PADDING + 4 * CELL_HEIGHT);
      ctx.stroke();

      ctx.beginPath();
      ctx.moveTo(x, PADDING + 5 * CELL_HEIGHT);
      ctx.lineTo(x, PADDING + 9 * CELL_HEIGHT);
      ctx.stroke();
    }
  }

  // Palaces diagonals
  const drawPalaceDiagonals = (startVisRow: number) => {
    ctx.save();
    ctx.strokeStyle = '#6B4226';
    ctx.lineWidth = 1.4;
    ctx.beginPath();
    ctx.moveTo(PADDING + 3 * CELL_WIDTH, PADDING + startVisRow * CELL_HEIGHT);
    ctx.lineTo(PADDING + 5 * CELL_WIDTH, PADDING + (startVisRow + 2) * CELL_HEIGHT);
    ctx.moveTo(PADDING + 5 * CELL_WIDTH, PADDING + startVisRow * CELL_HEIGHT);
    ctx.lineTo(PADDING + 3 * CELL_WIDTH, PADDING + (startVisRow + 2) * CELL_HEIGHT);
    ctx.stroke();
    ctx.restore();
  };

  drawPalaceDiagonals(0);
  drawPalaceDiagonals(7);

  // Subtle palace zone shading in setup mode to guide piece placement
  if (isSetupMode) {
    ctx.save();
    ctx.fillStyle = 'rgba(217, 119, 6, 0.08)';
    ctx.strokeStyle = 'rgba(217, 119, 6, 0.35)';
    ctx.lineWidth = 1.2;
    ctx.setLineDash([4, 4]);

    // Top palace
    ctx.fillRect(PADDING + 3 * CELL_WIDTH, PADDING, 2 * CELL_WIDTH, 2 * CELL_HEIGHT);
    ctx.strokeRect(PADDING + 3 * CELL_WIDTH, PADDING, 2 * CELL_WIDTH, 2 * CELL_HEIGHT);

    // Bottom palace
    ctx.fillRect(PADDING + 3 * CELL_WIDTH, PADDING + 7 * CELL_HEIGHT, 2 * CELL_WIDTH, 2 * CELL_HEIGHT);
    ctx.strokeRect(PADDING + 3 * CELL_WIDTH, PADDING + 7 * CELL_HEIGHT, 2 * CELL_WIDTH, 2 * CELL_HEIGHT);
    ctx.restore();
  }

  // Fiducial Markers for Cannons and Soldiers
  FIDUCIAL_POINTS.forEach(([r, c]) => {
    const { x, y } = getVisualCoords(r, c, playerSide);
    const visCol = playerSide === 'black' ? 8 - c : c;
    drawFiducialMark(ctx, x, y, visCol);
  });

  // River Calligraphy
  ctx.save();
  ctx.font = 'bold 22px "Noto Serif", "KaiTi", "SimSun", "STKaiti", serif';
  ctx.fillStyle = '#5C381E';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  const riverY = PADDING + 4.5 * CELL_HEIGHT;
  if (playerSide === 'red') {
    ctx.fillText('楚 河', PADDING + 2 * CELL_WIDTH, riverY);
    ctx.fillText('漢 界', PADDING + 6 * CELL_WIDTH, riverY);
  } else {
    ctx.fillText('漢 界', PADDING + 2 * CELL_WIDTH, riverY);
    ctx.fillText('楚 河', PADDING + 6 * CELL_WIDTH, riverY);
  }
  ctx.restore();
}

export function drawLastMoveBox(
  ctx: CanvasRenderingContext2D,
  lastMove: { from_row: number; from_col: number; to_row: number; to_col: number },
  playerSide: PieceColor
): void {
  const fromCoord = getVisualCoords(lastMove.from_row, lastMove.from_col, playerSide);
  const toCoord = getVisualCoords(lastMove.to_row, lastMove.to_col, playerSide);

  ctx.save();
  // Origin square marker
  ctx.strokeStyle = 'rgba(180, 83, 9, 0.75)';
  ctx.lineWidth = 2.5;
  ctx.setLineDash([5, 4]);
  ctx.strokeRect(fromCoord.x - 24, fromCoord.y - 24, 48, 48);

  // Destination square glow box
  ctx.setLineDash([]);
  ctx.strokeStyle = 'rgba(21, 128, 61, 0.9)';
  ctx.lineWidth = 2.8;
  ctx.strokeRect(toCoord.x - 25, toCoord.y - 25, 50, 50);
  ctx.restore();
}

export function drawLegalMoveMarkers(
  ctx: CanvasRenderingContext2D,
  legalMoves: [number, number][],
  board: Square[],
  playerSide: PieceColor
): void {
  legalMoves.forEach(([r, c]) => {
    const { x, y } = getVisualCoords(r, c, playerSide);
    const targetSquare = board.find((s) => s.row === r && s.col === c);
    const isCapture = Boolean(targetSquare && targetSquare.piece);

    ctx.save();
    if (isCapture) {
      // Vermilion capture target ring
      ctx.strokeStyle = '#DC2626';
      ctx.lineWidth = 3.2;
      ctx.beginPath();
      ctx.arc(x, y, 25, 0, Math.PI * 2);
      ctx.stroke();
    } else {
      // Jade destination dot
      ctx.fillStyle = 'rgba(22, 163, 74, 0.9)';
      ctx.beginPath();
      ctx.arc(x, y, 7.5, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = '#DCFCE7';
      ctx.lineWidth = 1.5;
      ctx.stroke();
    }
    ctx.restore();
  });
}
