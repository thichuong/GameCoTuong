import React from 'react';
import type {
  Difficulty,
  GameMode,
  GameStatus,
  Piece,
  PieceColor,
  SearchStats,
  Square,
} from '../types/game.js';
import type {
  CapturedPiecesSummary,
  LayoutOrientation,
} from '../utils/capturedPieces.js';
import { BoardCanvas } from './BoardCanvas.js';
import { CapturedPiecesTray } from './CapturedPiecesTray.js';
import { ThinkingIndicator } from './ThinkingIndicator.js';
import { GameOverOverlay } from './GameOverOverlay.js';

export interface BoardPlayAreaProps {
  layoutOrientation: LayoutOrientation;
  boardSize: number;
  board: Square[];
  playerSide: PieceColor;
  turn: PieceColor;
  status: GameStatus;
  gameMode: GameMode;
  difficulty: Difficulty;
  isThinking: boolean;
  lastStats: SearchStats | null;
  isPuzzleSetup: boolean;
  capturedSummary: CapturedPiecesSummary;
  selectedSquare: [number, number] | null;
  selectedSetupPiece: Piece | null;
  draggedTrayPiece: Piece | null;
  legalMoves: [number, number][];
  lastMove: {
    from_row: number;
    from_col: number;
    to_row: number;
    to_col: number;
  } | null;
  moveCount: number;
  onSquareClick: (row: number, col: number) => void;
  onSquareRightClick: (row: number, col: number) => void;
  onDropPieceOnSquare: (row: number, col: number, piece: Piece) => void;
  onMoveSetupPiece: (
    fromRow: number,
    fromCol: number,
    toRow: number,
    toCol: number
  ) => void;
  onSelectSetupPiece: (piece: Piece | null) => void;
  onSetDraggedTrayPiece: (piece: Piece | null) => void;
  onRestartPuzzle: () => void;
  onEditPuzzlePosition: () => void;
  onNewGame: () => void;
}

export const BoardPlayArea: React.FC<BoardPlayAreaProps> = ({
  layoutOrientation,
  boardSize,
  board,
  playerSide,
  turn,
  status,
  gameMode,
  difficulty,
  isThinking,
  lastStats,
  isPuzzleSetup,
  capturedSummary,
  selectedSquare,
  selectedSetupPiece,
  draggedTrayPiece,
  legalMoves,
  lastMove,
  moveCount,
  onSquareClick,
  onSquareRightClick,
  onDropPieceOnSquare,
  onMoveSetupPiece,
  onSelectSetupPiece,
  onSetDraggedTrayPiece,
  onRestartPuzzle,
  onEditPuzzlePosition,
  onNewGame,
}) => {
  const topSide: PieceColor = playerSide === 'red' ? 'black' : 'red';
  const bottomSide: PieceColor = playerSide === 'red' ? 'red' : 'black';

  const getSideData = (s: PieceColor) => {
    if (s === 'red') {
      return {
        side: 'red' as PieceColor,
        isTurn: !isPuzzleSetup && turn === 'red' && status.status === 'Playing',
        isThinking: isThinking && turn === 'red',
        capturedPieces: capturedSummary.groupedBlackCaptured,
        totalCapturedValue: capturedSummary.blackCapturedValue,
        materialAdvantage: capturedSummary.advantageForRed,
      };
    }
    return {
      side: 'black' as PieceColor,
      isTurn: !isPuzzleSetup && turn === 'black' && status.status === 'Playing',
      isThinking: isThinking && turn === 'black',
      capturedPieces: capturedSummary.groupedRedCaptured,
      totalCapturedValue: capturedSummary.redCapturedValue,
      materialAdvantage: -capturedSummary.advantageForRed,
    };
  };

  const topData = getSideData(topSide);
  const bottomData = getSideData(bottomSide);

  const centerBoardNode = (
    <div className="relative flex flex-col items-center justify-center shrink-0">
      <BoardCanvas
        board={board}
        playerSide={playerSide}
        selectedSquare={selectedSquare}
        selectedSetupPiece={selectedSetupPiece}
        legalMoves={legalMoves}
        lastMove={lastMove}
        onSquareClick={onSquareClick}
        onSquareRightClick={onSquareRightClick}
        onDropPieceOnSquare={onDropPieceOnSquare}
        onMoveSetupPiece={onMoveSetupPiece}
        externalDraggedPiece={draggedTrayPiece}
        disabled={isThinking || (status.status !== 'Playing' && !isPuzzleSetup)}
        size={boardSize}
        isSetupMode={isPuzzleSetup}
      >
        <ThinkingIndicator
          isThinking={isThinking}
          turn={turn}
          difficulty={difficulty}
          lastStats={lastStats}
        />
        <GameOverOverlay
          status={status}
          isPuzzleSetup={isPuzzleSetup}
          gameMode={gameMode}
          moveCount={moveCount}
          onRestartPuzzle={onRestartPuzzle}
          onEditPuzzlePosition={onEditPuzzlePosition}
          onNewGame={onNewGame}
        />
      </BoardCanvas>
    </div>
  );

  if (layoutOrientation === 'sides') {
    return (
      <div
        style={{ maxWidth: `${boardSize + 390}px` }}
        className="w-full px-2 sm:px-4 flex flex-row items-center justify-center gap-3 sm:gap-5 my-1 pb-1 sm:pb-2 transition-all"
      >
        {/* Left Tray: Opponent / Top Side */}
        <div style={{ height: `${boardSize}px` }} className="flex">
          <CapturedPiecesTray
            side={topData.side}
            isTurn={topData.isTurn}
            isThinking={topData.isThinking}
            capturedPieces={topData.capturedPieces}
            totalCapturedValue={topData.totalCapturedValue}
            materialAdvantage={topData.materialAdvantage}
            gameMode={gameMode}
            playerSide={playerSide}
            difficulty={difficulty}
            orientation="sides"
            isSetupMode={isPuzzleSetup}
            selectedSetupPiece={selectedSetupPiece}
            onSelectSetupPiece={onSelectSetupPiece}
            onDragStartSetupPiece={onSetDraggedTrayPiece}
            onDragEndSetupPiece={() => onSetDraggedTrayPiece(null)}
          />
        </div>

        {centerBoardNode}

        {/* Right Tray: Player / Bottom Side */}
        <div style={{ height: `${boardSize}px` }} className="flex">
          <CapturedPiecesTray
            side={bottomData.side}
            isTurn={bottomData.isTurn}
            isThinking={bottomData.isThinking}
            capturedPieces={bottomData.capturedPieces}
            totalCapturedValue={bottomData.totalCapturedValue}
            materialAdvantage={bottomData.materialAdvantage}
            gameMode={gameMode}
            playerSide={playerSide}
            difficulty={difficulty}
            orientation="sides"
            isSetupMode={isPuzzleSetup}
            selectedSetupPiece={selectedSetupPiece}
            onSelectSetupPiece={onSelectSetupPiece}
            onDragStartSetupPiece={onSetDraggedTrayPiece}
            onDragEndSetupPiece={() => onSetDraggedTrayPiece(null)}
          />
        </div>
      </div>
    );
  }

  return (
    <div
      style={{ maxWidth: `${boardSize + 24}px` }}
      className="w-full px-2 sm:px-3 flex flex-col items-center justify-center gap-2 my-1 pb-2 sm:pb-3 transition-all"
    >
      {/* Top Tray: Opponent */}
      <div style={{ width: `${boardSize}px`, maxWidth: '100%' }}>
        <CapturedPiecesTray
          side={topData.side}
          isTurn={topData.isTurn}
          isThinking={topData.isThinking}
          capturedPieces={topData.capturedPieces}
          totalCapturedValue={topData.totalCapturedValue}
          materialAdvantage={topData.materialAdvantage}
          gameMode={gameMode}
          playerSide={playerSide}
          difficulty={difficulty}
          orientation="stacked"
          isSetupMode={isPuzzleSetup}
          selectedSetupPiece={selectedSetupPiece}
          onSelectSetupPiece={onSelectSetupPiece}
          onDragStartSetupPiece={onSetDraggedTrayPiece}
          onDragEndSetupPiece={() => onSetDraggedTrayPiece(null)}
        />
      </div>

      {centerBoardNode}

      {/* Bottom Tray: Player */}
      <div style={{ width: `${boardSize}px`, maxWidth: '100%' }}>
        <CapturedPiecesTray
          side={bottomData.side}
          isTurn={bottomData.isTurn}
          isThinking={bottomData.isThinking}
          capturedPieces={bottomData.capturedPieces}
          totalCapturedValue={bottomData.totalCapturedValue}
          materialAdvantage={bottomData.materialAdvantage}
          gameMode={gameMode}
          playerSide={playerSide}
          difficulty={difficulty}
          orientation="stacked"
          isSetupMode={isPuzzleSetup}
          selectedSetupPiece={selectedSetupPiece}
          onSelectSetupPiece={onSelectSetupPiece}
          onDragStartSetupPiece={onSetDraggedTrayPiece}
          onDragEndSetupPiece={() => onSetDraggedTrayPiece(null)}
        />
      </div>
    </div>
  );
};
