import React, { useState } from 'react';
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
  Pencil,
  Puzzle,
  Trash2,
  FileCode,
  ShieldAlert,
  Copy,
  Check,
  LayoutGrid,
} from 'lucide-react';
import type {
  Difficulty,
  GameMode,
  PieceColor,
  PuzzleStage,
  PuzzleSubMode,
  Square,
} from '../types/game.js';
import {
  CLASSIC_PUZZLES,
  validateBoardSetup,
  boardToFen,
  fenToBoard,
} from '../utils/fen.js';

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
  puzzleStage?: PuzzleStage;
  puzzleSubMode?: PuzzleSubMode;
  setPuzzleSubMode?: (mode: PuzzleSubMode) => void;
  puzzleFirstTurn?: PieceColor;
  setPuzzleFirstTurn?: (turn: PieceColor) => void;
  board?: Square[];
  onUpdatePuzzleBoard?: (squares: Square[], turn?: PieceColor, recommendedMode?: PuzzleSubMode) => void;
  onClearPuzzleBoard?: () => void;
  onResetStandardBoard?: () => void;
  onStartSolving?: () => void;
  onEditPuzzlePosition?: () => void;
  onRestartPuzzle?: () => void;
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
  puzzleStage = 'playing',
  puzzleSubMode = 'HumanVsComputer',
  setPuzzleSubMode,
  puzzleFirstTurn = 'red',
  setPuzzleFirstTurn,
  board = [],
  onUpdatePuzzleBoard,
  onClearPuzzleBoard,
  onResetStandardBoard,
  onStartSolving,
  onEditPuzzlePosition,
  onRestartPuzzle,
}) => {
  const isPuzzleMode = gameMode === 'Puzzle';
  const isPuzzleSetup = isPuzzleMode && puzzleStage === 'setup';
  const effectiveSubMode = isPuzzleMode ? puzzleSubMode : gameMode;

  const [isFenModalOpen, setIsFenModalOpen] = useState(false);
  const [fenInput, setFenInput] = useState('');
  const [fenError, setFenError] = useState<string | null>(null);
  const [copiedFen, setCopiedFen] = useState(false);

  const validation = React.useMemo(
    () => (isPuzzleSetup ? validateBoardSetup(board) : { valid: true, errors: [] }),
    [isPuzzleSetup, board]
  );

  const currentFen = React.useMemo(
    () => (board.length > 0 ? boardToFen(board, puzzleFirstTurn) : ''),
    [board, puzzleFirstTurn]
  );

  const handleLoadPreset = (puzzleId: string) => {
    const puzzle = CLASSIC_PUZZLES.find((p) => p.id === puzzleId);
    if (!puzzle || !onUpdatePuzzleBoard) return;
    try {
      const { squares, turn } = fenToBoard(puzzle.fen);
      onUpdatePuzzleBoard(squares, turn, puzzle.recommendedMode);
    } catch (e) {
      console.error('Failed to load preset:', e);
    }
  };

  const handleApplyFen = () => {
    if (!onUpdatePuzzleBoard) return;
    try {
      setFenError(null);
      const { squares, turn } = fenToBoard(fenInput);
      onUpdatePuzzleBoard(squares, turn);
      setIsFenModalOpen(false);
    } catch (err: unknown) {
      setFenError(err instanceof Error ? err.message : 'FEN không hợp lệ');
    }
  };

  const handleCopyFen = () => {
    navigator.clipboard.writeText(currentFen).then(() => {
      setCopiedFen(true);
      setTimeout(() => setCopiedFen(false), 2000);
    });
  };

  return (
    <div className="w-full mx-auto rounded-2xl bg-[#1C1815] border border-stone-800/90 p-3.5 sm:p-4 shadow-xl flex flex-col gap-3">
      {/* Row 1: Game Setup Selectors */}
      <div className="flex flex-wrap items-center justify-between gap-2.5 sm:gap-4">
        {/* Chế độ chơi */}
        <div className="flex items-center gap-2 flex-1 min-w-[190px]">
          <label
            htmlFor="game-mode-select"
            className="inline-flex items-center gap-1.5 text-xs font-medium text-stone-400 whitespace-nowrap shrink-0"
          >
            {isPuzzleMode ? (
              <Puzzle className="w-3.5 h-3.5 text-amber-400" />
            ) : (
              <Gamepad2 className="w-3.5 h-3.5 text-amber-400" />
            )}
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
            <option value="Puzzle">🧩 Giải cờ thế (Tự xếp quân)</option>
          </select>
        </div>

        {/* Puzzle Sub-mode selector */}
        {isPuzzleMode && (
          <div className="flex items-center gap-2 flex-1 min-w-[185px]">
            <label
              htmlFor="puzzle-submode-select"
              className="inline-flex items-center gap-1.5 text-xs font-medium text-stone-400 whitespace-nowrap shrink-0"
            >
              <span>Thể thức</span>
            </label>
            {isPuzzleSetup && setPuzzleSubMode ? (
              <select
                id="puzzle-submode-select"
                value={puzzleSubMode}
                onChange={(e) => setPuzzleSubMode(e.target.value as PuzzleSubMode)}
                className="flex-1 bg-[#26201C] text-[#F5EFE6] text-xs sm:text-sm font-medium rounded-lg px-3 py-2 border border-stone-700/80 hover:border-amber-500/40 focus:border-amber-400 focus:outline-none transition-colors cursor-pointer"
              >
                <option value="HumanVsComputer">Người đấu với Máy</option>
                <option value="ComputerVsComputer">Máy đấu với Máy</option>
                <option value="HumanVsHuman">Hai người chơi</option>
              </select>
            ) : (
              <span className="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-amber-500/15 border border-amber-500/30 text-amber-300 text-xs font-semibold">
                {puzzleSubMode === 'ComputerVsComputer' && '🤖 Máy vs Máy'}
                {puzzleSubMode === 'HumanVsComputer' &&
                  `👤 vs 🤖 (Bạn cầm ${playerSide === 'red' ? 'Đỏ' : 'Đen'})`}
                {puzzleSubMode === 'HumanVsHuman' && '👥 Người vs Người'}
              </span>
            )}
          </div>
        )}

        {/* Side selector (HumanVsComputer in standard or Puzzle setup) */}
        {(gameMode === 'HumanVsComputer' ||
          (isPuzzleSetup && puzzleSubMode === 'HumanVsComputer')) && (
          <div className="flex items-center gap-2 flex-1 min-w-[175px]">
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
        {effectiveSubMode !== 'HumanVsHuman' && (
          <div className="flex items-center gap-2 flex-1 min-w-[180px]">
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
          {isPuzzleSetup ? (
            <>
              {/* Start Solving Button */}
              <button
                type="button"
                onClick={onStartSolving}
                disabled={!validation.valid}
                className={`inline-flex items-center gap-1.5 px-4 py-2 rounded-lg text-xs font-semibold transition-all cursor-pointer whitespace-nowrap shrink-0 shadow-sm ${
                  validation.valid
                    ? 'bg-emerald-600 hover:bg-emerald-500 text-white'
                    : 'bg-stone-800 text-stone-500 border border-stone-700/60 cursor-not-allowed opacity-60'
                }`}
              >
                <Play className="w-3.5 h-3.5 fill-current" />
                <span>Bắt đầu giải</span>
              </button>

              {/* First Turn Toggle */}
              {setPuzzleFirstTurn && (
                <button
                  type="button"
                  onClick={() =>
                    setPuzzleFirstTurn(puzzleFirstTurn === 'red' ? 'black' : 'red')
                  }
                  className={`inline-flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-semibold border transition-all cursor-pointer whitespace-nowrap shrink-0 ${
                    puzzleFirstTurn === 'red'
                      ? 'bg-red-950/70 border-red-500/60 text-red-200 hover:bg-red-900/60'
                      : 'bg-stone-800 border-amber-400/60 text-amber-200 hover:bg-stone-700'
                  }`}
                  title="Đổi bên đi nước đầu tiên khi giải thế"
                >
                  <span
                    className={`w-4 h-4 rounded-full flex items-center justify-center text-[10px] font-display-serif font-bold ${
                      puzzleFirstTurn === 'red'
                        ? 'bg-red-600 text-white'
                        : 'bg-stone-900 text-amber-300 border border-amber-400/40'
                    }`}
                  >
                    {puzzleFirstTurn === 'red' ? '帥' : '將'}
                  </span>
                  <span>{puzzleFirstTurn === 'red' ? 'Đỏ đi tiên' : 'Đen đi tiên'}</span>
                </button>
              )}

              {/* Classic Puzzle Presets */}
              <select
                aria-label="Thế cờ mẫu"
                onChange={(e) => {
                  if (e.target.value) {
                    handleLoadPreset(e.target.value);
                    e.target.value = '';
                  }
                }}
                defaultValue=""
                className="bg-[#26201C] text-amber-300 text-xs font-semibold rounded-lg px-2.5 py-2 border border-amber-500/40 hover:border-amber-400 focus:outline-none transition-colors cursor-pointer shrink-0"
              >
                <option value="" disabled>
                  📜 Thế cờ mẫu...
                </option>
                {CLASSIC_PUZZLES.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>

              {/* Clear Board (Keep 2 Generals) */}
              {onClearPuzzleBoard && (
                <button
                  type="button"
                  onClick={onClearPuzzleBoard}
                  className="inline-flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-semibold bg-[#26201C] hover:bg-[#312923] text-stone-200 border border-stone-700/80 transition-all cursor-pointer whitespace-nowrap shrink-0"
                  title="Xóa hết quân, chỉ để lại 2 Tướng"
                >
                  <Trash2 className="w-3.5 h-3.5 text-stone-400" />
                  <span>Xóa trắng</span>
                </button>
              )}

              {/* Standard 32-piece Board */}
              {onResetStandardBoard && (
                <button
                  type="button"
                  onClick={onResetStandardBoard}
                  className="inline-flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-semibold bg-[#26201C] hover:bg-[#312923] text-stone-200 border border-stone-700/80 transition-all cursor-pointer whitespace-nowrap shrink-0"
                  title="Đặt lại đủ 32 quân tiêu chuẩn"
                >
                  <LayoutGrid className="w-3.5 h-3.5 text-amber-400" />
                  <span>Bàn chuẩn</span>
                </button>
              )}

              {/* FEN Modal Button */}
              <button
                type="button"
                onClick={() => {
                  setFenInput(currentFen);
                  setFenError(null);
                  setIsFenModalOpen(true);
                }}
                className="inline-flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-semibold bg-[#26201C] hover:bg-[#312923] text-stone-200 border border-stone-700/80 transition-all cursor-pointer whitespace-nowrap shrink-0"
                title="Nhập hoặc sao chép chuỗi FEN"
              >
                <FileCode className="w-3.5 h-3.5 text-amber-400" />
                <span>FEN</span>
              </button>
            </>
          ) : (
            <>
              {/* If ComputerVsComputer (either standard or in Puzzle playing mode) */}
              {(gameMode === 'ComputerVsComputer' ||
                (isPuzzleMode && puzzleSubMode === 'ComputerVsComputer')) && (
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

              {/* Puzzle Playing Mode Controls */}
              {isPuzzleMode ? (
                <>
                  {onEditPuzzlePosition && (
                    <button
                      type="button"
                      onClick={onEditPuzzlePosition}
                      disabled={isThinking}
                      className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-xs font-semibold bg-amber-500/20 hover:bg-amber-500/30 text-amber-200 border border-amber-400/40 transition-all cursor-pointer whitespace-nowrap shrink-0 shadow-xs"
                    >
                      <Pencil className="w-3.5 h-3.5 text-amber-400" />
                      <span>Xếp lại cờ thế</span>
                    </button>
                  )}

                  {onRestartPuzzle && (
                    <button
                      type="button"
                      onClick={onRestartPuzzle}
                      disabled={isThinking}
                      className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-xs font-semibold bg-[#26201C] hover:bg-[#312923] text-stone-200 border border-stone-700/80 transition-all cursor-pointer whitespace-nowrap shrink-0"
                    >
                      <RotateCcw className="w-3.5 h-3.5 text-amber-400" />
                      <span>Giải lại từ đầu</span>
                    </button>
                  )}
                </>
              ) : (
                <button
                  type="button"
                  onClick={onNewGame}
                  disabled={isThinking}
                  className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-xs font-semibold bg-[#B91C1C] hover:bg-[#DC2626] disabled:opacity-45 disabled:cursor-not-allowed text-white transition-all cursor-pointer whitespace-nowrap shrink-0 shadow-sm"
                >
                  <RotateCcw className="w-3.5 h-3.5" />
                  <span>Ván mới</span>
                </button>
              )}

              <button
                type="button"
                onClick={onUndo}
                disabled={!canUndo || isThinking}
                className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-xs font-semibold bg-[#26201C] hover:bg-[#312923] disabled:opacity-40 disabled:cursor-not-allowed text-[#F5EFE6] border border-stone-700/80 transition-all cursor-pointer whitespace-nowrap shrink-0"
              >
                <Undo2 className="w-3.5 h-3.5 text-amber-400" />
                <span>Đi lại</span>
              </button>
            </>
          )}
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

      {/* Setup Guide / Validation Status Strip in Puzzle Setup Mode */}
      {isPuzzleSetup && (
        <div className="pt-1 border-t border-stone-800/60 flex flex-wrap items-center justify-between gap-2 text-xs">
          {!validation.valid ? (
            <div className="flex items-center gap-1.5 text-red-300 font-medium">
              <ShieldAlert className="w-3.5 h-3.5 text-red-400 shrink-0" />
              <span>{validation.errors.join(' · ')}</span>
            </div>
          ) : (
            <span className="text-stone-400">
              Kéo quân từ khay 2 bên vào bàn cờ · Giữ chuột trái để dời quân trên bàn · Chuột phải để xóa quân
            </span>
          )}
        </div>
      )}

      {/* FEN Import/Export Modal */}
      {isFenModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-xs">
          <div className="w-full max-w-lg rounded-2xl bg-[#1C1815] border border-amber-500/40 shadow-2xl p-5 flex flex-col gap-3">
            <div className="flex items-center justify-between pb-2 border-b border-stone-800">
              <h3 className="font-display-serif text-base font-bold text-amber-200 flex items-center gap-2">
                <FileCode className="w-4 h-4 text-amber-400" />
                <span>Nhập / Xuất FEN Cờ Thế</span>
              </h3>
              <button
                type="button"
                onClick={() => setIsFenModalOpen(false)}
                className="text-stone-400 hover:text-stone-200 text-sm font-bold px-2 py-0.5 rounded cursor-pointer"
              >
                ✕
              </button>
            </div>

            <p className="text-xs text-stone-400">
              Dán chuỗi FEN để nạp nhanh thế cờ hoặc sao chép chuỗi FEN của bàn cờ hiện tại:
            </p>

            <textarea
              value={fenInput}
              onChange={(e) => {
                setFenInput(e.target.value);
                setFenError(null);
              }}
              rows={3}
              placeholder="Ví dụ: 3ak4/4a4/9/9/9/9/9/4N4/4p4/4K4 w"
              className="w-full bg-[#120F0D] text-amber-100 font-mono text-xs rounded-xl p-3 border border-stone-700 focus:border-amber-400 focus:outline-none resize-none"
            />

            {fenError && <p className="text-xs text-red-400">{fenError}</p>}

            <div className="flex items-center justify-between gap-2 pt-2">
              <button
                type="button"
                onClick={handleCopyFen}
                className="inline-flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-semibold bg-[#26201C] hover:bg-stone-800 text-stone-200 border border-stone-700 transition-colors cursor-pointer"
              >
                {copiedFen ? (
                  <>
                    <Check className="w-3.5 h-3.5 text-emerald-400" />
                    <span>Đã sao chép!</span>
                  </>
                ) : (
                  <>
                    <Copy className="w-3.5 h-3.5 text-amber-400" />
                    <span>Sao chép FEN hiện tại</span>
                  </>
                )}
              </button>

              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => setIsFenModalOpen(false)}
                  className="px-3 py-2 rounded-lg text-xs font-medium text-stone-400 hover:text-stone-200 cursor-pointer"
                >
                  Đóng
                </button>
                <button
                  type="button"
                  onClick={handleApplyFen}
                  className="px-4 py-2 rounded-lg text-xs font-semibold bg-amber-600 hover:bg-amber-500 text-white transition-colors cursor-pointer shadow-md"
                >
                  Áp dụng FEN
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
