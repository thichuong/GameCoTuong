import React, { useRef, useEffect, useCallback, useState } from 'react';
import type { Piece, PieceColor, PieceType, Square } from '../types/game.js';

interface BoardCanvasProps {
  board: Square[];
  playerSide: PieceColor;
  selectedSquare: [number, number] | null;
  legalMoves: [number, number][];
  lastMove: { from_row: number; from_col: number; to_row: number; to_col: number } | null;
  onSquareClick: (row: number, col: number) => void;
  onSquareRightClick?: (row: number, col: number) => void;
  disabled?: boolean;
  size?: number;
  children?: React.ReactNode;
  isSetupMode?: boolean;
}

const BOARD_SIZE = 554.0;
const PADDING = 34.0;
const GRID_SIZE = BOARD_SIZE - PADDING * 2; // 486.0
const CELL_WIDTH = GRID_SIZE / 8; // 60.75
const CELL_HEIGHT = GRID_SIZE / 9; // 54.0
const BOARD_WIDTH = BOARD_SIZE;
const BOARD_HEIGHT = BOARD_SIZE;

const FIDUCIAL_POINTS: [number, number][] = [
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

function getPieceSymbol(type: PieceType, color: PieceColor): string {
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

function getVisualCoords(
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

function getLogicCoords(
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

function drawFiducialMark(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  visCol: number
) {
  const offset = 4.5;
  const len = 9;
  ctx.strokeStyle = '#6B4226';
  ctx.lineWidth = 1.4;

  // Left side L-marks (only if not on left edge)
  if (visCol > 0) {
    // Top-left
    ctx.beginPath();
    ctx.moveTo(x - offset - len, y - offset);
    ctx.lineTo(x - offset, y - offset);
    ctx.lineTo(x - offset, y - offset - len);
    ctx.stroke();

    // Bottom-left
    ctx.beginPath();
    ctx.moveTo(x - offset - len, y + offset);
    ctx.lineTo(x - offset, y + offset);
    ctx.lineTo(x - offset, y + offset + len);
    ctx.stroke();
  }

  // Right side L-marks (only if not on right edge)
  if (visCol < 8) {
    // Top-right
    ctx.beginPath();
    ctx.moveTo(x + offset + len, y - offset);
    ctx.lineTo(x + offset, y - offset);
    ctx.lineTo(x + offset, y - offset - len);
    ctx.stroke();

    // Bottom-right
    ctx.beginPath();
    ctx.moveTo(x + offset + len, y + offset);
    ctx.lineTo(x + offset, y + offset);
    ctx.lineTo(x + offset, y + offset + len);
    ctx.stroke();
  }
}

export const BoardCanvas: React.FC<BoardCanvasProps> = ({
  board,
  playerSide,
  selectedSquare,
  legalMoves,
  lastMove,
  onSquareClick,
  onSquareRightClick,
  disabled = false,
  size,
  children,
  isSetupMode = false,
}) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [internalSize, setInternalSize] = useState<number>(() =>
    typeof window !== 'undefined'
      ? Math.max(200, Math.min(window.innerWidth - 32, window.innerHeight - 200))
      : BOARD_SIZE
  );

  useEffect(() => {
    if (size !== undefined) return;
    const updateSize = () => {
      setInternalSize(Math.max(200, Math.min(window.innerWidth - 32, window.innerHeight - 200)));
    };
    updateSize();
    window.addEventListener('resize', updateSize);
    return () => window.removeEventListener('resize', updateSize);
  }, [size]);

  const effectiveSize = size ?? internalSize;

  const drawBoard = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const pixelSize = Math.round(effectiveSize * dpr);
    if (canvas.width !== pixelSize || canvas.height !== pixelSize) {
      canvas.width = pixelSize;
      canvas.height = pixelSize;
    }

    ctx.save();
    const scale = pixelSize / BOARD_SIZE;
    ctx.scale(scale, scale);

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

    // Highlight Last Move
    if (lastMove && !isSetupMode) {
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

    // Draw Pieces
    board.forEach((sq) => {
      if (!sq.piece) return;
      const { x, y } = getVisualCoords(sq.row, sq.col, playerSide);
      const isSelected =
        selectedSquare !== null &&
        selectedSquare[0] === sq.row &&
        selectedSquare[1] === sq.col;

      drawPiece(ctx, x, y, sq.piece, isSelected);
    });

    // Draw Setup Mode Selected Square ring (if empty square selected)
    if (isSetupMode && selectedSquare) {
      const [selR, selC] = selectedSquare;
      const sq = board.find((s) => s.row === selR && s.col === selC);
      if (!sq || !sq.piece) {
        const { x, y } = getVisualCoords(selR, selC, playerSide);
        ctx.save();
        ctx.strokeStyle = '#F59E0B';
        ctx.lineWidth = 2.2;
        ctx.setLineDash([4, 3]);
        ctx.beginPath();
        ctx.arc(x, y, 22, 0, Math.PI * 2);
        ctx.stroke();
        ctx.restore();
      }
    }

    // Draw Legal Move Indicators (only in playing mode)
    if (!isSetupMode) {
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

    ctx.restore();
  }, [board, playerSide, selectedSquare, legalMoves, lastMove, effectiveSize, isSetupMode]);

  useEffect(() => {
    drawBoard();
  }, [drawBoard]);

  const handleCanvasClick = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (disabled) return;
    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const scaleX = BOARD_WIDTH / rect.width;
    const scaleY = BOARD_HEIGHT / rect.height;

    const clickX = (e.clientX - rect.left) * scaleX;
    const clickY = (e.clientY - rect.top) * scaleY;

    const coords = getLogicCoords(clickX, clickY, playerSide);
    if (coords) {
      onSquareClick(coords[0], coords[1]);
    }
  };

  const handleCanvasContextMenu = (e: React.MouseEvent<HTMLCanvasElement>) => {
    e.preventDefault();
    if (disabled || !isSetupMode || !onSquareRightClick) return;
    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const scaleX = BOARD_WIDTH / rect.width;
    const scaleY = BOARD_HEIGHT / rect.height;

    const clickX = (e.clientX - rect.left) * scaleX;
    const clickY = (e.clientY - rect.top) * scaleY;

    const coords = getLogicCoords(clickX, clickY, playerSide);
    if (coords) {
      onSquareRightClick(coords[0], coords[1]);
    }
  };

  return (
    <div
      style={{
        width: `${effectiveSize}px`,
        height: `${effectiveSize}px`,
        maxWidth: '100%',
      }}
      className="relative inline-flex items-center justify-center aspect-square rounded-2xl bg-gradient-to-b from-[#2B1D14] to-[#1D130D] border border-[#4A3222] shadow-[0_20px_50px_rgba(0,0,0,0.75)] box-border overflow-hidden shrink-0"
    >
      <canvas
        ref={canvasRef}
        onClick={handleCanvasClick}
        onContextMenu={handleCanvasContextMenu}
        style={{
          width: `${effectiveSize}px`,
          height: `${effectiveSize}px`,
          maxWidth: '100%',
          aspectRatio: '1 / 1',
        }}
        className={`rounded-xl block w-full h-full aspect-square transition-opacity ${
          disabled ? 'cursor-not-allowed' : 'cursor-pointer'
        }`}
      />
      {children}
    </div>
  );
};

function drawPiece(
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
