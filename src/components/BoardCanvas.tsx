import React, { useRef, useEffect, useCallback } from 'react';
import type { Piece, PieceColor, PieceType, Square } from '../types/game.js';

interface BoardCanvasProps {
  board: Square[];
  playerSide: PieceColor;
  selectedSquare: [number, number] | null;
  legalMoves: [number, number][];
  lastMove: { from_row: number; from_col: number; to_row: number; to_col: number } | null;
  onSquareClick: (row: number, col: number) => void;
  disabled?: boolean;
}

const CELL_SIZE = 50.0;
const PADDING = 25.0;
const BOARD_WIDTH = 450.0;
const BOARD_HEIGHT = 500.0;

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
    x: visCol * CELL_SIZE + PADDING,
    y: visRow * CELL_SIZE + PADDING,
  };
}

function getLogicCoords(
  x: number,
  y: number,
  side: PieceColor
): [number, number] | null {
  const visCol = Math.round((x - PADDING) / CELL_SIZE);
  const visRow = Math.round((y - PADDING) / CELL_SIZE);

  if (visCol < 0 || visCol > 8 || visRow < 0 || visRow > 9) {
    return null;
  }

  // Check click distance to intersection
  const snapX = visCol * CELL_SIZE + PADDING;
  const snapY = visRow * CELL_SIZE + PADDING;
  const dist = Math.hypot(x - snapX, y - snapY);
  if (dist > CELL_SIZE * 0.48) {
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

export const BoardCanvas: React.FC<BoardCanvasProps> = ({
  board,
  playerSide,
  selectedSquare,
  legalMoves,
  lastMove,
  onSquareClick,
  disabled = false,
}) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  const drawBoard = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Clear board background
    ctx.clearRect(0, 0, BOARD_WIDTH, BOARD_HEIGHT);
    ctx.fillStyle = '#edd8b7'; // Rich traditional Xiangqi wooden texture
    ctx.fillRect(0, 0, BOARD_WIDTH, BOARD_HEIGHT);

    // Border frame
    ctx.strokeStyle = '#5a3d28';
    ctx.lineWidth = 3;
    ctx.strokeRect(PADDING - 5, PADDING - 5, 8 * CELL_SIZE + 10, 9 * CELL_SIZE + 10);

    ctx.strokeStyle = '#8b5a2b';
    ctx.lineWidth = 1.5;

    // Draw 10 horizontal lines
    for (let r = 0; r < 10; r++) {
      const y = r * CELL_SIZE + PADDING;
      ctx.beginPath();
      ctx.moveTo(PADDING, y);
      ctx.lineTo(PADDING + 8 * CELL_SIZE, y);
      ctx.stroke();
    }

    // Draw 9 vertical lines (broken at the river between r=4 and r=5)
    for (let c = 0; c < 9; c++) {
      const x = c * CELL_SIZE + PADDING;
      if (c === 0 || c === 8) {
        // Outer boundaries are continuous
        ctx.beginPath();
        ctx.moveTo(x, PADDING);
        ctx.lineTo(x, PADDING + 9 * CELL_SIZE);
        ctx.stroke();
      } else {
        // Top half
        ctx.beginPath();
        ctx.moveTo(x, PADDING);
        ctx.lineTo(x, PADDING + 4 * CELL_SIZE);
        ctx.stroke();

        // Bottom half
        ctx.beginPath();
        ctx.moveTo(x, PADDING + 5 * CELL_SIZE);
        ctx.lineTo(x, PADDING + 9 * CELL_SIZE);
        ctx.stroke();
      }
    }

    // Draw River Text
    ctx.save();
    ctx.font = 'bold 20px "KaiTi", "SimSun", "STKaiti", serif';
    ctx.fillStyle = '#8b5a2b';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    const riverY = PADDING + 4.5 * CELL_SIZE;
    if (playerSide === 'red') {
      ctx.fillText('楚 河', PADDING + 2 * CELL_SIZE, riverY);
      ctx.fillText('漢 界', PADDING + 6 * CELL_SIZE, riverY);
    } else {
      ctx.fillText('漢 界', PADDING + 2 * CELL_SIZE, riverY);
      ctx.fillText('楚 河', PADDING + 6 * CELL_SIZE, riverY);
    }
    ctx.restore();

    // Palaces diagonals
    const drawPalaceDiagonals = (startVisRow: number) => {
      ctx.beginPath();
      ctx.moveTo(PADDING + 3 * CELL_SIZE, PADDING + startVisRow * CELL_SIZE);
      ctx.lineTo(PADDING + 5 * CELL_SIZE, PADDING + (startVisRow + 2) * CELL_SIZE);
      ctx.moveTo(PADDING + 5 * CELL_SIZE, PADDING + startVisRow * CELL_SIZE);
      ctx.lineTo(PADDING + 3 * CELL_SIZE, PADDING + (startVisRow + 2) * CELL_SIZE);
      ctx.stroke();
    };

    drawPalaceDiagonals(0); // Top palace
    drawPalaceDiagonals(7); // Bottom palace

    // Highlight Last Move
    if (lastMove) {
      const fromCoord = getVisualCoords(lastMove.from_row, lastMove.from_col, playerSide);
      const toCoord = getVisualCoords(lastMove.to_row, lastMove.to_col, playerSide);

      ctx.save();
      ctx.strokeStyle = 'rgba(41, 128, 185, 0.7)';
      ctx.lineWidth = 3;
      ctx.setLineDash([4, 4]);

      // Box around from square
      ctx.strokeRect(fromCoord.x - 23, fromCoord.y - 23, 46, 46);
      // Box around to square
      ctx.strokeStyle = 'rgba(39, 174, 96, 0.8)';
      ctx.strokeRect(toCoord.x - 23, toCoord.y - 23, 46, 46);
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

    // Draw Legal Move Dots/Indicators
    legalMoves.forEach(([r, c]) => {
      const { x, y } = getVisualCoords(r, c, playerSide);
      const targetSquare = board.find((s) => s.row === r && s.col === c);
      const isCapture = Boolean(targetSquare && targetSquare.piece);

      ctx.save();
      if (isCapture) {
        // Draw red/gold capture ring
        ctx.strokeStyle = '#e74c3c';
        ctx.lineWidth = 3;
        ctx.beginPath();
        ctx.arc(x, y, 24, 0, Math.PI * 2);
        ctx.stroke();
      } else {
        // Draw green destination dot
        ctx.fillStyle = 'rgba(46, 204, 113, 0.85)';
        ctx.beginPath();
        ctx.arc(x, y, 7, 0, Math.PI * 2);
        ctx.fill();
        ctx.strokeStyle = '#27ae60';
        ctx.lineWidth = 1.5;
        ctx.stroke();
      }
      ctx.restore();
    });
  }, [board, playerSide, selectedSquare, legalMoves, lastMove]);

  useEffect(() => {
    drawBoard();
  }, [drawBoard]);

  const handleCanvasClick = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (disabled) return;
    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;

    const clickX = (e.clientX - rect.left) * scaleX;
    const clickY = (e.clientY - rect.top) * scaleY;

    const coords = getLogicCoords(clickX, clickY, playerSide);
    if (coords) {
      onSquareClick(coords[0], coords[1]);
    }
  };

  return (
    <div style={{ display: 'inline-block', position: 'relative', margin: '10px auto' }}>
      <canvas
        ref={canvasRef}
        width={BOARD_WIDTH}
        height={BOARD_HEIGHT}
        onClick={handleCanvasClick}
        style={{
          borderRadius: '8px',
          boxShadow: '0 8px 24px rgba(0, 0, 0, 0.5)',
          cursor: disabled ? 'not-allowed' : 'pointer',
          maxWidth: '100%',
          height: 'auto',
          display: 'block',
        }}
      />
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
  const radius = 22;

  ctx.save();

  // Shadow / Glow
  if (isSelected) {
    ctx.shadowBlur = 16;
    ctx.shadowColor = '#f39c12';
  } else {
    ctx.shadowBlur = 4;
    ctx.shadowColor = 'rgba(0, 0, 0, 0.4)';
    ctx.shadowOffsetX = 2;
    ctx.shadowOffsetY = 2;
  }

  // Outer piece wooden disk
  ctx.beginPath();
  ctx.arc(x, y, radius, 0, Math.PI * 2);
  ctx.fillStyle = piece.color === 'red' ? '#fdf5e6' : '#f0e6d2';
  ctx.fill();

  ctx.shadowBlur = 0;
  ctx.shadowOffsetX = 0;
  ctx.shadowOffsetY = 0;

  // Outer rim
  ctx.lineWidth = 2.5;
  ctx.strokeStyle = piece.color === 'red' ? '#c0392b' : '#2c3e50';
  ctx.stroke();

  // Inner decorative ring
  ctx.beginPath();
  ctx.arc(x, y, radius - 3.5, 0, Math.PI * 2);
  ctx.lineWidth = 1;
  ctx.strokeStyle = piece.color === 'red' ? '#e74c3c' : '#7f8c8d';
  ctx.stroke();

  // Selected highlight ring
  if (isSelected) {
    ctx.beginPath();
    ctx.arc(x, y, radius + 2, 0, Math.PI * 2);
    ctx.lineWidth = 2.5;
    ctx.strokeStyle = '#f1c40f';
    ctx.stroke();
  }

  // Text character
  const symbol = getPieceSymbol(piece.type, piece.color);
  ctx.font = 'bold 22px "KaiTi", "SimSun", "STKaiti", serif';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillStyle = piece.color === 'red' ? '#c0392b' : '#1a252f';
  ctx.fillText(symbol, x, y + 1);

  ctx.restore();
}

