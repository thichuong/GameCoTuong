import React, { useState, useMemo } from 'react';
import { getPieceCursorStyle } from './components/BoardCanvas.js';
import { TopHeader } from './components/TopHeader.js';
import { ControlsArea } from './components/ControlsArea.js';
import { BoardPlayArea } from './components/BoardPlayArea.js';
import { LogPanel } from './components/LogPanel.js';
import { ConfigModal } from './components/ConfigModal.js';
import { useResponsiveLayout } from './hooks/useResponsiveLayout.js';
import { useXiangqiGame } from './hooks/useXiangqiGame.js';
import { exportCsv } from './utils/export.js';
import './App.css';

export const App: React.FC = () => {
  const [isLogOpen, setIsLogOpen] = useState(false);
  const [isConfigOpen, setIsConfigOpen] = useState(false);

  const {
    layoutOverride,
    setLayoutOverride,
    layoutOrientation,
    boardSize,
  } = useResponsiveLayout();

  const {
    gameMode,
    handleGameModeChange,
    playerSide,
    setPlayerSide,
    difficulty,
    setDifficulty,
    isPaused,
    setIsPaused,
    isThinking,
    lastStats,
    puzzleStage,
    puzzleSubMode,
    setPuzzleSubMode,
    puzzleFirstTurn,
    setPuzzleFirstTurn,
    selectedSetupPiece,
    draggedTrayPiece,
    setDraggedTrayPiece,
    board,
    turn,
    status,
    isInCheck,
    selectedSquare,
    legalMoves,
    lastMove,
    history,
    redConfig,
    setRedConfig,
    blackConfig,
    setBlackConfig,
    capturedSummary,
    isPuzzleSetup,
    activeCursorPiece,
    handleDropPieceOnSquare,
    handleMoveSetupPiece,
    handleSelectSetupPiece,
    handleSetupSquareRightClick,
    handleUpdatePuzzleBoard,
    handleClearPuzzleBoard,
    handleResetStandardBoard,
    handleStartSolving,
    handleRestartPuzzle,
    handleEditPuzzlePosition,
    handleSquareClick,
    handleNewGame,
    handleUndo,
    handleEngineConfigChange,
  } = useXiangqiGame();

  const activePieceCursorStyle = useMemo(
    () => getPieceCursorStyle(activeCursorPiece),
    [activeCursorPiece]
  );

  return (
    <div
      style={
        activePieceCursorStyle ? { cursor: activePieceCursorStyle } : undefined
      }
      className="min-h-screen bg-[#14110F] text-[#F5EFE6] flex flex-col justify-between py-3 sm:py-5"
    >
      <TopHeader
        layoutOrientation={layoutOrientation}
        boardSize={boardSize}
        layoutOverride={layoutOverride}
        setLayoutOverride={setLayoutOverride}
        isInCheck={isInCheck}
        status={status}
        isPuzzleSetup={isPuzzleSetup}
        activeTurn={isPuzzleSetup ? puzzleFirstTurn : turn}
      />

      {/* Main Content Area */}
      <main className="flex-1 flex flex-col items-center justify-center gap-3 sm:gap-4 w-full mx-auto">
        <div
          style={{
            maxWidth:
              layoutOrientation === 'sides'
                ? `${Math.max(900, boardSize + 390)}px`
                : `${Math.max(340, boardSize + 24)}px`,
          }}
          className="w-full px-3 sm:px-6 transition-[max-width] duration-150"
        >
          <ControlsArea
            gameMode={gameMode}
            setGameMode={handleGameModeChange}
            playerSide={playerSide}
            setPlayerSide={setPlayerSide}
            difficulty={difficulty}
            setDifficulty={setDifficulty}
            isPaused={isPaused}
            setIsPaused={setIsPaused}
            onNewGame={handleNewGame}
            onUndo={handleUndo}
            onExportCsv={() => exportCsv(history)}
            onOpenConfig={() => setIsConfigOpen(true)}
            onToggleLog={() => setIsLogOpen((prev) => !prev)}
            moveCount={history.length}
            isThinking={isThinking}
            canUndo={history.length > 0}
            puzzleStage={puzzleStage}
            puzzleSubMode={puzzleSubMode}
            setPuzzleSubMode={setPuzzleSubMode}
            puzzleFirstTurn={puzzleFirstTurn}
            setPuzzleFirstTurn={setPuzzleFirstTurn}
            board={board}
            onUpdatePuzzleBoard={handleUpdatePuzzleBoard}
            onClearPuzzleBoard={handleClearPuzzleBoard}
            onResetStandardBoard={handleResetStandardBoard}
            onStartSolving={handleStartSolving}
            onEditPuzzlePosition={handleEditPuzzlePosition}
            onRestartPuzzle={handleRestartPuzzle}
          />
        </div>

        <BoardPlayArea
          layoutOrientation={layoutOrientation}
          boardSize={boardSize}
          board={board}
          playerSide={playerSide}
          turn={turn}
          status={status}
          gameMode={gameMode}
          difficulty={difficulty}
          isThinking={isThinking}
          lastStats={lastStats}
          isPuzzleSetup={isPuzzleSetup}
          capturedSummary={capturedSummary}
          selectedSquare={selectedSquare}
          selectedSetupPiece={selectedSetupPiece}
          draggedTrayPiece={draggedTrayPiece}
          legalMoves={legalMoves}
          lastMove={lastMove}
          moveCount={history.length}
          onSquareClick={handleSquareClick}
          onSquareRightClick={handleSetupSquareRightClick}
          onDropPieceOnSquare={handleDropPieceOnSquare}
          onMoveSetupPiece={handleMoveSetupPiece}
          onSelectSetupPiece={handleSelectSetupPiece}
          onSetDraggedTrayPiece={setDraggedTrayPiece}
          onRestartPuzzle={handleRestartPuzzle}
          onEditPuzzlePosition={handleEditPuzzlePosition}
          onNewGame={handleNewGame}
        />

        {/* Collapsible Move Log Dock at the bottom */}
        <LogPanel
          history={history}
          isOpen={isLogOpen}
          onToggle={setIsLogOpen}
          onExportCsv={() => exportCsv(history)}
        />
      </main>

      <ConfigModal
        isOpen={isConfigOpen}
        onClose={() => setIsConfigOpen(false)}
        redConfig={redConfig}
        setRedConfig={setRedConfig}
        blackConfig={blackConfig}
        setBlackConfig={setBlackConfig}
        onConfigChange={handleEngineConfigChange}
      />
    </div>
  );
};

export default App;
