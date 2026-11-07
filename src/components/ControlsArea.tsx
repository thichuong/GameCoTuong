import React from 'react';
import {
  RotateCcw,
  Undo2,
  Play,
  Pause,
  SlidersHorizontal,
  Download,
  ScrollText,
  Gamepad2,
  Shield,
  Gauge,
} from 'lucide-react';
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
  onToggleLog?: () => void;
  moveCount?: number;
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
  onToggleLog,
  moveCount = 0,
  isThinking,
  canUndo,
}) => {
  return (
    <div className="w-full mx-auto rounded-2xl bg-[#1C1815] border border-stone-800/90 p-3.5 sm:p-4 shadow-xl flex flex-col gap-3">
      {/* Row 1: Game Setup Selectors */}
      <div className="flex flex-wrap items-center justify-between gap-2.5 sm:gap-4">
        {/* Chế độ chơi */}
        <div className="flex items-center gap-2 flex-1 min-w-[200px]">
          <label
            htmlFor="game-mode-select"
            className="inline-flex items-center gap-1.5 text-xs font-medium text-stone-400 whitespace-nowrap shrink-0"
          >
            <Gamepad2 className="w-3.5 h-3.5 text-amber-400" />
            <span>Chế độ</span>
          </label>
          <select
            id="game-mode-select"
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
            className="flex-1 bg-[#26201C] text-[#F5EFE6] text-xs sm:text-sm font-medium rounded-lg px-3 py-2 border border-stone-700/80 hover:border-amber-500/40 focus:border-amber-400 focus:outline-none transition-colors cursor-pointer"
          >
            <option value="HumanVsComputer">Người đấu với Máy</option>
            <option value="ComputerVsComputer">Máy đấu với Máy</option>
            <option value="HumanVsHuman">Hai người chơi</option>
          </select>
        </div>

        {/* Chọn bên */}
        {gameMode === 'HumanVsComputer' && (
          <div className="flex items-center gap-2 flex-1 min-w-[180px]">
            <label
              htmlFor="player-side-select"
              className="inline-flex items-center gap-1.5 text-xs font-medium text-stone-400 whitespace-nowrap shrink-0"
            >
              <Shield className="w-3.5 h-3.5 text-amber-400" />
              <span>Cầm quân</span>
            </label>
            <select
              id="player-side-select"
              value={playerSide}
              onChange={(e) => setPlayerSide(e.target.value as PieceColor)}
              className="flex-1 bg-[#26201C] text-[#F5EFE6] text-xs sm:text-sm font-medium rounded-lg px-3 py-2 border border-stone-700/80 hover:border-amber-500/40 focus:border-amber-400 focus:outline-none transition-colors cursor-pointer"
            >
              <option value="red">Đỏ (Đi tiên)</option>
              <option value="black">Đen (Đi hậu)</option>
            </select>
          </div>
        )}

        {/* Độ khó */}
        {gameMode !== 'HumanVsHuman' && (
          <div className="flex items-center gap-2 flex-1 min-w-[185px]">
            <label
              htmlFor="difficulty-select"
              className="inline-flex items-center gap-1.5 text-xs font-medium text-stone-400 whitespace-nowrap shrink-0"
            >
              <Gauge className="w-3.5 h-3.5 text-amber-400" />
              <span>Độ khó</span>
            </label>
            <select
              id="difficulty-select"
              value={difficulty}
              onChange={(e) => setDifficulty(Number(e.target.value) as Difficulty)}
              className="flex-1 bg-[#26201C] text-[#F5EFE6] text-xs sm:text-sm font-medium rounded-lg px-3 py-2 border border-stone-700/80 hover:border-amber-500/40 focus:border-amber-400 focus:outline-none transition-colors cursor-pointer"
            >
              <option value={1}>Mức 1 · Nhập môn (1s)</option>
              <option value={2}>Mức 2 · Cơ bản (2s)</option>
              <option value={3}>Mức 3 · Trung cấp (5s)</option>
              <option value={4}>Mức 4 · Cao thủ (10s)</option>
              <option value={5}>Mức 5 · Đại sư (20s)</option>
            </select>
          </div>
        )}
      </div>

      {/* Hairline Divider */}
      <div className="h-px bg-stone-800/80" />

      {/* Row 2: Action Toolbar */}
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-wrap items-center gap-2">
          {gameMode === 'ComputerVsComputer' && (
            <button
              type="button"
              onClick={() => setIsPaused(!isPaused)}
              className={`inline-flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-xs font-semibold transition-all cursor-pointer whitespace-nowrap shrink-0 ${
                isPaused
                  ? 'bg-emerald-600 hover:bg-emerald-500 text-white shadow-sm'
                  : 'bg-amber-600 hover:bg-amber-500 text-white shadow-sm'
              }`}
            >
              {isPaused ? (
                <>
                  <Play className="w-3.5 h-3.5 fill-current" />
                  <span>Bắt đầu</span>
                </>
              ) : (
                <>
                  <Pause className="w-3.5 h-3.5 fill-current" />
                  <span>Tạm dừng</span>
                </>
              )}
            </button>
          )}

          <button
            type="button"
            onClick={onNewGame}
            disabled={isThinking}
            className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-xs font-semibold bg-[#B91C1C] hover:bg-[#DC2626] disabled:opacity-45 disabled:cursor-not-allowed text-white transition-all cursor-pointer whitespace-nowrap shrink-0 shadow-sm"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            <span>Ván mới</span>
          </button>

          <button
            type="button"
            onClick={onUndo}
            disabled={!canUndo || isThinking}
            className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-xs font-semibold bg-[#26201C] hover:bg-[#312923] disabled:opacity-40 disabled:cursor-not-allowed text-[#F5EFE6] border border-stone-700/80 transition-all cursor-pointer whitespace-nowrap shrink-0"
          >
            <Undo2 className="w-3.5 h-3.5 text-amber-400" />
            <span>Đi lại</span>
          </button>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          {onToggleLog && (
            <button
              type="button"
              onClick={onToggleLog}
              className="inline-flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-medium bg-[#26201C] hover:bg-[#312923] text-stone-200 border border-stone-700/80 transition-colors cursor-pointer whitespace-nowrap shrink-0"
            >
              <ScrollText className="w-3.5 h-3.5 text-amber-400" />
              <span>Biên bản</span>
              <span className="font-mono-tabular text-amber-400">({moveCount})</span>
            </button>
          )}

          <button
            type="button"
            onClick={onExportCsv}
            className="inline-flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-medium bg-[#26201C] hover:bg-[#312923] text-stone-200 border border-stone-700/80 transition-colors cursor-pointer whitespace-nowrap shrink-0"
          >
            <Download className="w-3.5 h-3.5 text-amber-400" />
            <span>Xuất CSV</span>
          </button>

          <button
            type="button"
            onClick={onOpenConfig}
            className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-xs font-semibold bg-amber-500/20 hover:bg-amber-500/30 text-amber-200 border border-amber-400/40 shadow-xs transition-all cursor-pointer whitespace-nowrap shrink-0"
          >
            <SlidersHorizontal className="w-3.5 h-3.5 text-amber-400" />
            <span>Cấu hình AI (NNUE)</span>
          </button>
        </div>
      </div>
    </div>
  );
};
