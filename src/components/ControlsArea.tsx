import React from 'react';
import type { Difficulty, GameMode, PieceColor } from '../types/game.js';

interface ControlsAreaProps {
  gameMode: GameMode;
  setGameMode: (mode: GameMode) => void;
  playerSide: PieceColor;
  setPlayerSide: (side: PieceColor) => void;
  difficulty: Difficulty;
  setDifficulty: (diff: Difficulty) => void;
  isPaused: boolean;
  setIsPaused: (paused: boolean) => void;
  onNewGame: () => void;
  onUndo: () => void;
  onExportCsv: () => void;
  onOpenConfig: () => void;
  isThinking: boolean;
  canUndo: boolean;
}

export const ControlsArea: React.FC<ControlsAreaProps> = ({
  gameMode,
  setGameMode,
  playerSide,
  setPlayerSide,
  difficulty,
  setDifficulty,
  isPaused,
  setIsPaused,
  onNewGame,
  onUndo,
  onExportCsv,
  onOpenConfig,
  isThinking,
  canUndo,
}) => {
  return (
    <div className="controls-area">
      <div className="controls-config">
        {/* Chế độ chơi */}
        <div className="control-group">
          <label className="control-label" htmlFor="game-mode-select">Chế độ</label>
          <select
            id="game-mode-select"
            className="control-select"
            value={gameMode}
            onChange={(e) => {
              const mode = e.target.value as GameMode;
              setGameMode(mode);
              if (mode === 'ComputerVsComputer') {
                setIsPaused(true);
              } else {
                setIsPaused(false);
              }
            }}
          >
            <option value="HumanVsComputer">Người vs Máy</option>
            <option value="ComputerVsComputer">Máy vs Máy</option>
            <option value="HumanVsHuman">Người vs Người</option>
          </select>
        </div>

        {/* Chọn bên */}
        {gameMode === 'HumanVsComputer' && (
          <div className="control-group">
            <label className="control-label" htmlFor="player-side-select">Chọn bên</label>
            <select
              id="player-side-select"
              className="control-select"
              value={playerSide}
              onChange={(e) => setPlayerSide(e.target.value as PieceColor)}
            >
              <option value="red">Đỏ (Đi trước)</option>
              <option value="black">Đen (Đi sau)</option>
            </select>
          </div>
        )}

        {/* Độ khó */}
        {gameMode !== 'HumanVsHuman' && (
          <div className="control-group">
            <label className="control-label" htmlFor="difficulty-select">Độ khó</label>
            <select
              id="difficulty-select"
              className="control-select"
              value={difficulty}
              onChange={(e) => setDifficulty(Number(e.target.value) as Difficulty)}
            >
              <option value={1}>Mức 1 (1s)</option>
              <option value={2}>Mức 2 (2s)</option>
              <option value={3}>Mức 3 (5s)</option>
              <option value={4}>Mức 4 (10s)</option>
              <option value={5}>Mức 5 (20s)</option>
            </select>
          </div>
        )}
      </div>

      <div className="controls-actions">
        {gameMode === 'ComputerVsComputer' && (
          <button
            type="button"
            className={`control-btn ${isPaused ? 'btn-success' : 'btn-danger'}`}
            onClick={() => setIsPaused(!isPaused)}
          >
            {isPaused ? '▶ Bắt đầu' : '⏸ Tạm dừng'}
          </button>
        )}

        <button
          type="button"
          className="control-btn btn-primary"
          onClick={onNewGame}
          disabled={isThinking}
        >
          🔄 Chơi mới
        </button>

        <button
          type="button"
          className="control-btn btn-warning"
          onClick={onUndo}
          disabled={!canUndo || isThinking}
        >
          ↩ Đi lại
        </button>

        <button
          type="button"
          className="control-btn btn-secondary"
          onClick={onExportCsv}
        >
          📥 Xuất CSV
        </button>

        <button
          type="button"
          className="control-btn btn-info"
          onClick={onOpenConfig}
        >
          ⚙ Cấu hình AI
        </button>
      </div>
    </div>
  );
};

