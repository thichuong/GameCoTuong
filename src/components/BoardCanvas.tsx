import React, { useRef, useEffect, useCallback, useState } from 'react';
import type { BoardViewMode, Piece, PieceColor, Square } from '../types/game.js';
import {
  BOARD_SIZE,
  BOARD_WIDTH,
  BOARD_HEIGHT,
  getVisualCoords,
} from './board/boardCoords.js';
import {
  getPieceCursorStyle,
  drawPiece,
  drawBoardGridAndRiver,
  drawLastMoveBox,
  drawLegalMoveMarkers,
} from './board/boardRenderer.js';
import {
  type CameraParams,
  VIEW_MODE_CAMERAS,
  isPerspectiveCamera,
  lerpCameraParams,
  unprojectToBoard,
  pickBoardSquare,
} from './board/boardProjection.js';
import {
  type MoveAnimationState,
  type CameraTransitionState,
  CAMERA_TRANSITION_MS,
  MOVE_ANIMATION_MS,
  easeOutCubic,
  easeInOutCubic,
  computeArcLiftZ,
  prefersReducedMotion,
} from './board/boardAnimation.js';
import {
  sortSquaresBackToFront,
  drawTabletopBackground,
  drawBoardSlab3D,
  drawLastMoveBox3D,
  drawLegalMoveMarkers3D,
  drawHoverAndSetupHighlights3D,
  drawPiece3D,
} from './board/boardRenderer3d.js';

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
  viewMode?: BoardViewMode;
  moveCount?: number;
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
  viewMode = '2.5d',
  moveCount = 0,
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

  // Camera & piece flight animation refs (RAF loop only runs during active animations)
  const currentCameraRef = useRef<CameraParams>(VIEW_MODE_CAMERAS[viewMode]);
  const prevViewModeRef = useRef<BoardViewMode>(viewMode);
  const cameraAnimRef = useRef<CameraTransitionState | null>(null);
  const moveAnimRef = useRef<MoveAnimationState | null>(null);
  const prevMoveCountRef = useRef<number>(moveCount);
  const rafIdRef = useRef<number | null>(null);
  const drawBoardRef = useRef<() => void>(() => {});

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

  const stepAnimation = useCallback(() => {
    rafIdRef.current = null;
    drawBoardRef.current();
    if (cameraAnimRef.current !== null || moveAnimRef.current !== null) {
      rafIdRef.current = window.requestAnimationFrame(stepAnimation);
    }
  }, []);

  const ensureAnimationLoop = useCallback(() => {
    if (typeof window === 'undefined') return;
    if (
      rafIdRef.current === null &&
      (cameraAnimRef.current !== null || moveAnimRef.current !== null)
    ) {
      rafIdRef.current = window.requestAnimationFrame(stepAnimation);
    }
  }, [stepAnimation]);

  useEffect(() => {
    return () => {
      if (rafIdRef.current !== null && typeof window !== 'undefined') {
        window.cancelAnimationFrame(rafIdRef.current);
        rafIdRef.current = null;
      }
    };
  }, []);

  // Trigger smooth camera tilt transition when viewMode changes
  useEffect(() => {
    if (prevViewModeRef.current === viewMode) return;
    prevViewModeRef.current = viewMode;
    const target = VIEW_MODE_CAMERAS[viewMode];

    if (prefersReducedMotion()) {
      cameraAnimRef.current = null;
      currentCameraRef.current = target;
      drawBoardRef.current();
      return;
    }

    cameraAnimRef.current = {
      from: { ...currentCameraRef.current },
      to: target,
      startTime: typeof performance !== 'undefined' ? performance.now() : Date.now(),
      durationMs: CAMERA_TRANSITION_MS,
    };
    ensureAnimationLoop();
  }, [viewMode, ensureAnimationLoop]);

  // Trigger parabolic 3D flight animation on forward moves (skip Undo / New Game / Setup)
  useEffect(() => {
    const prevCount = prevMoveCountRef.current;
    prevMoveCountRef.current = moveCount;

    if (isSetupMode || !lastMove || moveCount <= prevCount || prefersReducedMotion()) {
      if (isSetupMode || !lastMove || moveCount < prevCount) {
        moveAnimRef.current = null;
      }
      return;
    }

    const destSq = board.find(
      (sq) => sq.row === lastMove.to_row && sq.col === lastMove.to_col
    );
    if (!destSq?.piece) return;

    moveAnimRef.current = {
      fromRow: lastMove.from_row,
      fromCol: lastMove.from_col,
      toRow: lastMove.to_row,
      toCol: lastMove.to_col,
      piece: destSq.piece,
      startTime: typeof performance !== 'undefined' ? performance.now() : Date.now(),
      durationMs: MOVE_ANIMATION_MS,
    };
    ensureAnimationLoop();
  }, [lastMove, moveCount, isSetupMode, board, ensureAnimationLoop]);

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

    const now = typeof performance !== 'undefined' ? performance.now() : Date.now();
    if (cameraAnimRef.current) {
      const camAnim = cameraAnimRef.current;
      const rawT = camAnim.durationMs > 0 ? (now - camAnim.startTime) / camAnim.durationMs : 1;
      if (rawT >= 1) {
        currentCameraRef.current = camAnim.to;
        cameraAnimRef.current = null;
      } else {
        currentCameraRef.current = lerpCameraParams(
          camAnim.from,
          camAnim.to,
          easeOutCubic(rawT)
        );
      }
    }

    let activeMoveAnim: { anim: MoveAnimationState; t: number } | null = null;
    if (moveAnimRef.current) {
      const anim = moveAnimRef.current;
      const rawT = anim.durationMs > 0 ? (now - anim.startTime) / anim.durationMs : 1;
      if (rawT >= 1) {
        moveAnimRef.current = null;
      } else {
        activeMoveAnim = { anim, t: Math.max(0, rawT) };
      }
    }

    const cam = currentCameraRef.current;
    const dpr = window.devicePixelRatio || 1;
    const pixelSize = Math.round(effectiveSize * dpr);
    if (canvas.width !== pixelSize || canvas.height !== pixelSize) {
      canvas.width = pixelSize;
      canvas.height = pixelSize;
    }

    ctx.save();
    const scale = pixelSize / BOARD_SIZE;
    ctx.scale(scale, scale);

    const hasFloatingPiece =
      Boolean(boardDrag?.hasMoved) ||
      Boolean(externalDraggedPiece && trayDragPos) ||
      Boolean(activeSelectedPiece && mousePos);

    const canHighlightHover =
      Boolean(hoverTarget && hasFloatingPiece) &&
      (isSetupMode ||
        (hoverTarget !== null &&
          legalMoves.some(([r, c]) => r === hoverTarget[0] && c === hoverTarget[1])));

    let selectedEmptySquare: [number, number] | null = null;
    if (isSetupMode && selectedSquare) {
      const sq = board.find((s) => s.row === selectedSquare[0] && s.col === selectedSquare[1]);
      if (!sq || !sq.piece) {
        selectedEmptySquare = selectedSquare;
      }
    }

    let floatingInfo: {
      piece: Piece;
      screenPt: { x: number; y: number };
    } | null = null;
    if (isSetupMode && boardDrag && boardDrag.hasMoved) {
      floatingInfo = {
        piece: boardDrag.piece,
        screenPt: { x: boardDrag.currentX, y: boardDrag.currentY },
      };
    } else if (isSetupMode && externalDraggedPiece && trayDragPos) {
      floatingInfo = {
        piece: externalDraggedPiece,
        screenPt: trayDragPos,
      };
    } else if (activeSelectedPiece && mousePos) {
      floatingInfo = {
        piece: activeSelectedPiece,
        screenPt: mousePos,
      };
    }

    if (!isPerspectiveCamera(cam)) {
      // Pure 2D flat rendering path
      drawBoardGridAndRiver(ctx, playerSide, isSetupMode);

      if (lastMove && !isSetupMode) {
        drawLastMoveBox(ctx, lastMove, playerSide);
      }

      if (hoverTarget && canHighlightHover) {
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

      board.forEach((sq) => {
        if (!sq.piece) return;
        if (
          activeMoveAnim &&
          sq.row === activeMoveAnim.anim.toRow &&
          sq.col === activeMoveAnim.anim.toCol
        ) {
          return;
        }
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

      if (selectedEmptySquare) {
        const { x, y } = getVisualCoords(
          selectedEmptySquare[0],
          selectedEmptySquare[1],
          playerSide
        );
        ctx.save();
        ctx.strokeStyle = '#F59E0B';
        ctx.lineWidth = 2.2;
        ctx.setLineDash([4, 3]);
        ctx.beginPath();
        ctx.arc(x, y, 22, 0, Math.PI * 2);
        ctx.stroke();
        ctx.restore();
      }

      if (!isSetupMode) {
        drawLegalMoveMarkers(ctx, legalMoves, board, playerSide);
      }

      if (activeMoveAnim) {
        const { anim, t } = activeMoveAnim;
        const fromPt = getVisualCoords(anim.fromRow, anim.fromCol, playerSide);
        const toPt = getVisualCoords(anim.toRow, anim.toCol, playerSide);
        const u = easeInOutCubic(t);
        const px = fromPt.x + (toPt.x - fromPt.x) * u;
        const py = fromPt.y + (toPt.y - fromPt.y) * u;
        const popScale = 1 + 0.08 * Math.sin(Math.PI * t);
        ctx.save();
        ctx.translate(px, py);
        ctx.scale(popScale, popScale);
        drawPiece(ctx, 0, 0, anim.piece, false);
        ctx.restore();
      }

      if (floatingInfo) {
        drawPiece(
          ctx,
          floatingInfo.screenPt.x,
          floatingInfo.screenPt.y,
          floatingInfo.piece,
          true
        );
      }
    } else {
      // 2.5D / 3D perspective rendering pipeline
      const captureTargets = new Set<string>();
      if (!isSetupMode) {
        for (const [mr, mc] of legalMoves) {
          const targetSq = board.find((s) => s.row === mr && s.col === mc);
          if (targetSq?.piece) {
            captureTargets.add(`${mr},${mc}`);
          }
        }
      }

      drawTabletopBackground(ctx, cam);
      drawBoardSlab3D(ctx, playerSide, isSetupMode, cam);

      if (lastMove && !isSetupMode) {
        drawLastMoveBox3D(ctx, lastMove, playerSide, cam);
      }

      drawHoverAndSetupHighlights3D(
        ctx,
        {
          hoverTarget,
          canHighlightHover,
          isSetupMode,
          selectedEmptySquare,
          playerSide,
        },
        cam
      );

      if (!isSetupMode) {
        drawLegalMoveMarkers3D(ctx, legalMoves, board, playerSide, cam);
      }

      const sortedSquares = sortSquaresBackToFront(board, playerSide);
      for (const sq of sortedSquares) {
        if (!sq.piece) continue;
        if (
          activeMoveAnim &&
          sq.row === activeMoveAnim.anim.toRow &&
          sq.col === activeMoveAnim.anim.toCol
        ) {
          continue;
        }

        const { x: bx, y: by } = getVisualCoords(sq.row, sq.col, playerSide);
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
        const isCaptureTarget = captureTargets.has(`${sq.row},${sq.col}`);

        if (isBeingDragged || isSelectedWithCursorFollower) {
          drawPiece3D(
            ctx,
            bx,
            by,
            sq.piece,
            { isSelected: true, liftZ: 0, alpha: 0.35 },
            cam
          );
        } else {
          drawPiece3D(
            ctx,
            bx,
            by,
            sq.piece,
            {
              isSelected,
              liftZ: isSelected ? cam.pieceHeight * 0.65 : 0,
              isCaptureTarget,
            },
            cam
          );
        }
      }

      if (activeMoveAnim) {
        const { anim, t } = activeMoveAnim;
        const fromPt = getVisualCoords(anim.fromRow, anim.fromCol, playerSide);
        const toPt = getVisualCoords(anim.toRow, anim.toCol, playerSide);
        const u = easeInOutCubic(t);
        const bx = fromPt.x + (toPt.x - fromPt.x) * u;
        const by = fromPt.y + (toPt.y - fromPt.y) * u;
        const liftZ = computeArcLiftZ(t, 18 + cam.pieceHeight);
        drawPiece3D(ctx, bx, by, anim.piece, { isSelected: false, liftZ }, cam);
      }

      if (floatingInfo) {
        const floatLiftZ = 18;
        const unproj = unprojectToBoard(
          floatingInfo.screenPt.x,
          floatingInfo.screenPt.y,
          floatLiftZ + cam.pieceHeight,
          cam
        );
        const bx = unproj
          ? Math.max(-20, Math.min(BOARD_SIZE + 20, unproj.x))
          : floatingInfo.screenPt.x;
        const by = unproj
          ? Math.max(-20, Math.min(BOARD_SIZE + 20, unproj.y))
          : floatingInfo.screenPt.y;
        drawPiece3D(
          ctx,
          bx,
          by,
          floatingInfo.piece,
          { isSelected: true, liftZ: floatLiftZ },
          cam
        );
      }
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

  drawBoardRef.current = drawBoard;

  useEffect(() => {
    drawBoard();
  }, [drawBoard]);

  const handleCanvasMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (disabled || !isSetupMode || e.button !== 0) return;
    if (selectedSetupPiece) return;

    const pt = getCanvasPoint(e.clientX, e.clientY);
    if (!pt) return;

    const coords = pickBoardSquare(
      pt.x,
      pt.y,
      playerSide,
      currentCameraRef.current,
      board,
      { prioritizePieces: true, selectedSquare }
    );
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
    const isDraggingBoardPiece = Boolean(isSetupMode && boardDrag);
    const target = pickBoardSquare(
      pt.x,
      pt.y,
      playerSide,
      currentCameraRef.current,
      board,
      {
        prioritizePieces: !isDraggingBoardPiece,
        selectedSquare,
        excludeSquare:
          isDraggingBoardPiece && boardDrag
            ? [boardDrag.fromRow, boardDrag.fromCol]
            : null,
      }
    );
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
    const target = pt
      ? pickBoardSquare(pt.x, pt.y, playerSide, currentCameraRef.current, board, {
          prioritizePieces: false,
          excludeSquare: [boardDrag.fromRow, boardDrag.fromCol],
        })
      : null;

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
    const coords = pickBoardSquare(
      pt.x,
      pt.y,
      playerSide,
      currentCameraRef.current,
      board,
      { prioritizePieces: true, selectedSquare }
    );
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

    const coords = pickBoardSquare(
      pt.x,
      pt.y,
      playerSide,
      currentCameraRef.current,
      board,
      { prioritizePieces: true, selectedSquare }
    );
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
    const coords = pickBoardSquare(
      pt.x,
      pt.y,
      playerSide,
      currentCameraRef.current,
      board,
      { prioritizePieces: true }
    );
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

    const coords = pickBoardSquare(
      pt.x,
      pt.y,
      playerSide,
      currentCameraRef.current,
      board,
      { prioritizePieces: true }
    );
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
