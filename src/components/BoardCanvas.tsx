import React, { useRef, useEffect, useCallback, useState } from 'react';
import type { Piece, PieceColor, Square } from '../types/game.js';
import {
  BOARD_SIZE,
  BOARD_WIDTH,
  BOARD_HEIGHT,
  getVisualCoords,
  getLogicCoords,
} from './board/boardCoords.js';
import {
  getPieceCursorStyle,
  drawPiece,
  drawBoardGridAndRiver,
  drawLastMoveBox,
  drawLegalMoveMarkers,
} from './board/boardRenderer.js';

export { getPieceCursorStyle };

export interface BoardCanvasProps {
  board: Square[];
  playerSide: PieceColor;
  selectedSquare: [number, number] | null;
  selectedSetupPiece?: Piece | null;
  legalMoves: [number, number][];
  lastMove: { from_row: number; from_col: number; to_row: number; to_col: number } | null;
  onSquareClick: (row: number, col: number) => void;
  onSquareRightClick?: (row: number, col: number) => void;
  onDropPieceOnSquare?: (row: number, col: number, piece: Piece) => void;
  onMoveSetupPiece?: (fromRow: number, fromCol: number, toRow: number, toCol: number) => void;
  externalDraggedPiece?: Piece | null;
  disabled?: boolean;
  size?: number;
  children?: React.ReactNode;
  isSetupMode?: boolean;
}

export const BoardCanvas: React.FC<BoardCanvasProps> = ({
  board,
  playerSide,
  selectedSquare,
  selectedSetupPiece = null,
  legalMoves,
  lastMove,
  onSquareClick,
  onSquareRightClick,
  onDropPieceOnSquare,
  onMoveSetupPiece,
  externalDraggedPiece = null,
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

  // Setup mode drag state (dragging a piece already on the board or hovering from tray)
  const [boardDrag, setBoardDrag] = useState<{
    fromRow: number;
    fromCol: number;
    piece: Piece;
    startX: number;
    startY: number;
    currentX: number;
    currentY: number;
    hasMoved: boolean;
  } | null>(null);

  const [mousePos, setMousePos] = useState<{ x: number; y: number } | null>(null);
  const [hoverTarget, setHoverTarget] = useState<[number, number] | null>(null);
  const [trayDragPos, setTrayDragPos] = useState<{ x: number; y: number } | null>(null);
  const suppressNextClickRef = useRef(false);

  // Piece on the currently selected board square (if any)
  const selectedBoardPiece = React.useMemo(() => {
    if (!selectedSquare) return null;
    const sq = board.find((s) => s.row === selectedSquare[0] && s.col === selectedSquare[1]);
    return sq?.piece ?? null;
  }, [board, selectedSquare]);

  // Active piece that should follow the cursor (selected from tray or selected on board)
  const activeSelectedPiece: Piece | null = isSetupMode
    ? (selectedSetupPiece ?? selectedBoardPiece)
    : selectedBoardPiece;

  useEffect(() => {
    if (size !== undefined) return;
    const updateSize = () => {
      setInternalSize(Math.max(200, Math.min(window.innerWidth - 32, window.innerHeight - 200)));
    };
    updateSize();
    window.addEventListener('resize', updateSize);
    return () => window.removeEventListener('resize', updateSize);
  }, [size]);

  useEffect(() => {
    if (!isSetupMode) {
      setBoardDrag(null);
      setTrayDragPos(null);
    }
  }, [isSetupMode]);

  const effectiveSize = size ?? internalSize;

  const getCanvasPoint = useCallback(
    (clientX: number, clientY: number): { x: number; y: number } | null => {
      const canvas = canvasRef.current;
      if (!canvas) return null;
      const rect = canvas.getBoundingClientRect();
      const scaleX = BOARD_WIDTH / rect.width;
      const scaleY = BOARD_HEIGHT / rect.height;
      return {
        x: (clientX - rect.left) * scaleX,
        y: (clientY - rect.top) * scaleY,
      };
    },
    []
  );

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

    drawBoardGridAndRiver(ctx, playerSide, isSetupMode);

    // Highlight Last Move
    if (lastMove && !isSetupMode) {
      drawLastMoveBox(ctx, lastMove, playerSide);
    }

    // Draw Hover Target Ring when dragging or holding a selected piece
    const hasFloatingPiece =
      Boolean(boardDrag?.hasMoved) ||
      Boolean(externalDraggedPiece && trayDragPos) ||
      Boolean(activeSelectedPiece && mousePos);

    if (hoverTarget && hasFloatingPiece) {
      const canHighlightTarget =
        isSetupMode ||
        legalMoves.some(([r, c]) => r === hoverTarget[0] && c === hoverTarget[1]);

      if (canHighlightTarget) {
        const { x, y } = getVisualCoords(hoverTarget[0], hoverTarget[1], playerSide);
        ctx.save();
        ctx.fillStyle = 'rgba(245, 158, 11, 0.22)';
        ctx.strokeStyle = '#D97706';
        ctx.lineWidth = 2.8;
        ctx.beginPath();
        ctx.arc(x, y, 26, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();
        ctx.restore();
      }
    }

    // Draw Pieces on Board
    board.forEach((sq) => {
      if (!sq.piece) return;
      const { x, y } = getVisualCoords(sq.row, sq.col, playerSide);
      const isSelected =
        selectedSquare !== null &&
        selectedSquare[0] === sq.row &&
        selectedSquare[1] === sq.col;

      const isBeingDragged =
        isSetupMode &&
        boardDrag !== null &&
        boardDrag.hasMoved &&
        boardDrag.fromRow === sq.row &&
        boardDrag.fromCol === sq.col;

      const isSelectedWithCursorFollower = isSelected && mousePos !== null;

      if (isBeingDragged || isSelectedWithCursorFollower) {
        ctx.save();
        ctx.globalAlpha = 0.4;
        drawPiece(ctx, x, y, sq.piece, true);
        ctx.restore();
      } else {
        drawPiece(ctx, x, y, sq.piece, isSelected);
      }
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
      drawLegalMoveMarkers(ctx, legalMoves, board, playerSide);
    }

    // Draw Floating Piece at Cursor (from board drag, external tray drag, or selected piece)
    if (isSetupMode && boardDrag && boardDrag.hasMoved) {
      drawPiece(ctx, boardDrag.currentX, boardDrag.currentY, boardDrag.piece, true);
    } else if (isSetupMode && externalDraggedPiece && trayDragPos) {
      drawPiece(ctx, trayDragPos.x, trayDragPos.y, externalDraggedPiece, true);
    } else if (activeSelectedPiece && mousePos) {
      drawPiece(ctx, mousePos.x, mousePos.y, activeSelectedPiece, true);
    }

    ctx.restore();
  }, [
    board,
    playerSide,
    selectedSquare,
    legalMoves,
    lastMove,
    effectiveSize,
    isSetupMode,
    hoverTarget,
    boardDrag,
    externalDraggedPiece,
    trayDragPos,
    activeSelectedPiece,
    mousePos,
  ]);

  useEffect(() => {
    drawBoard();
  }, [drawBoard]);

  const handleCanvasMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (disabled || !isSetupMode || e.button !== 0) return;
    if (selectedSetupPiece) return;

    const pt = getCanvasPoint(e.clientX, e.clientY);
    if (!pt) return;

    const coords = getLogicCoords(pt.x, pt.y, playerSide);
    if (!coords) return;

    const [row, col] = coords;
    const sq = board.find((s) => s.row === row && s.col === col);
    if (sq?.piece) {
      setBoardDrag({
        fromRow: row,
        fromCol: col,
        piece: sq.piece,
        startX: pt.x,
        startY: pt.y,
        currentX: pt.x,
        currentY: pt.y,
        hasMoved: false,
      });
      setHoverTarget([row, col]);
    }
  };

  const handleCanvasMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (disabled) return;
    const pt = getCanvasPoint(e.clientX, e.clientY);
    if (!pt) return;

    setMousePos(pt);
    const target = getLogicCoords(pt.x, pt.y, playerSide);
    setHoverTarget(target);

    if (isSetupMode && boardDrag) {
      const dist = Math.hypot(pt.x - boardDrag.startX, pt.y - boardDrag.startY);
      const hasMoved = boardDrag.hasMoved || dist > 5;

      setBoardDrag((prev) =>
        prev
          ? {
              ...prev,
              currentX: pt.x,
              currentY: pt.y,
              hasMoved,
            }
          : null
      );
    }
  };

  const handleCanvasMouseUp = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (disabled || !isSetupMode || !boardDrag || e.button !== 0) return;
    const pt = getCanvasPoint(e.clientX, e.clientY);
    const target = pt ? getLogicCoords(pt.x, pt.y, playerSide) : null;

    if (boardDrag.hasMoved) {
      suppressNextClickRef.current = true;
      if (
        target &&
        onMoveSetupPiece &&
        (target[0] !== boardDrag.fromRow || target[1] !== boardDrag.fromCol)
      ) {
        onMoveSetupPiece(boardDrag.fromRow, boardDrag.fromCol, target[0], target[1]);
      }
    }

    setBoardDrag(null);
  };

  const handleCanvasMouseLeave = () => {
    setMousePos(null);
    setHoverTarget(null);
    if (boardDrag) {
      setBoardDrag(null);
    }
  };

  const handleCanvasClick = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (suppressNextClickRef.current) {
      suppressNextClickRef.current = false;
      return;
    }
    if (disabled) return;
    const pt = getCanvasPoint(e.clientX, e.clientY);
    if (!pt) return;

    setMousePos(pt);
    const coords = getLogicCoords(pt.x, pt.y, playerSide);
    if (coords) {
      onSquareClick(coords[0], coords[1]);
    }
  };

  const handleCanvasContextMenu = (e: React.MouseEvent<HTMLCanvasElement>) => {
    e.preventDefault();
    if (disabled || !isSetupMode || !onSquareRightClick) return;
    setBoardDrag(null);

    const pt = getCanvasPoint(e.clientX, e.clientY);
    if (!pt) return;

    const coords = getLogicCoords(pt.x, pt.y, playerSide);
    if (coords) {
      onSquareRightClick(coords[0], coords[1]);
    }
  };

  const handleCanvasDragOver = (e: React.DragEvent<HTMLCanvasElement>) => {
    if (disabled || !isSetupMode) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = 'copy';

    const pt = getCanvasPoint(e.clientX, e.clientY);
    if (!pt) return;

    setTrayDragPos(pt);
    setMousePos(pt);
    const coords = getLogicCoords(pt.x, pt.y, playerSide);
    setHoverTarget(coords);
  };

  const handleCanvasDragLeave = () => {
    if (!isSetupMode) return;
    setHoverTarget(null);
    setTrayDragPos(null);
  };

  const handleCanvasDrop = (e: React.DragEvent<HTMLCanvasElement>) => {
    if (disabled || !isSetupMode || !onDropPieceOnSquare) return;
    e.preventDefault();

    const pt = getCanvasPoint(e.clientX, e.clientY);
    setHoverTarget(null);
    setTrayDragPos(null);
    if (!pt) return;

    const coords = getLogicCoords(pt.x, pt.y, playerSide);
    if (!coords) return;

    let droppedPiece: Piece | null = externalDraggedPiece;
    const raw =
      e.dataTransfer.getData('application/xiangqi-piece') ||
      e.dataTransfer.getData('text/plain');
    if (raw) {
      try {
        const parsed = JSON.parse(raw) as Piece;
        if (parsed && parsed.type && parsed.color) {
          droppedPiece = parsed;
        }
      } catch {
        // fallback to externalDraggedPiece
      }
    }

    if (droppedPiece) {
      onDropPieceOnSquare(coords[0], coords[1], droppedPiece);
    }
  };

  const hideNativeCursor =
    !disabled &&
    ((isSetupMode && boardDrag?.hasMoved) ||
      Boolean(activeSelectedPiece && mousePos));

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
        onMouseDown={handleCanvasMouseDown}
        onMouseMove={handleCanvasMouseMove}
        onMouseUp={handleCanvasMouseUp}
        onMouseLeave={handleCanvasMouseLeave}
        onClick={handleCanvasClick}
        onContextMenu={handleCanvasContextMenu}
        onDragOver={handleCanvasDragOver}
        onDragLeave={handleCanvasDragLeave}
        onDrop={handleCanvasDrop}
        style={{
          width: `${effectiveSize}px`,
          height: `${effectiveSize}px`,
          maxWidth: '100%',
          aspectRatio: '1 / 1',
        }}
        className={`rounded-xl block w-full h-full aspect-square transition-opacity ${
          disabled
            ? 'cursor-not-allowed'
            : hideNativeCursor
              ? 'cursor-none'
              : 'cursor-pointer'
        }`}
      />
      {children}
    </div>
  );
};
