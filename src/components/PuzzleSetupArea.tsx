import React, { useState } from 'react';
import {
  Play,
  Trash2,
  RotateCcw,
  ArrowRightLeft,
  FileCode,
  ShieldAlert,
  CheckCircle2,
  Sparkles,
} from 'lucide-react';
import type {
  Difficulty,
  Piece,
  PieceColor,
  PuzzleSubMode,
  Square,
} from '../types/game.js';
import {
  CLASSIC_PUZZLES,
  validateBoardSetup,
  boardToFen,
  fenToBoard,
  createEmptyBoard,
} from '../utils/fen.js';
import { FenModal } from './controls/FenModal.js';
import { PuzzlePiecePalette } from './puzzle/PuzzlePiecePalette.js';
import { PuzzleMatchSettings } from './puzzle/PuzzleMatchSettings.js';

export interface PuzzleSetupAreaProps {
  board: Square[];
  setBoard: React.Dispatch<React.SetStateAction<Square[]>>;
  firstTurn: PieceColor;
  setFirstTurn: (turn: PieceColor) => void;
  subMode: PuzzleSubMode;
  setSubMode: (mode: PuzzleSubMode) => void;
  playerSide: PieceColor;
  setPlayerSide: (side: PieceColor) => void;
  difficulty: Difficulty;
  setDifficulty: (diff: Difficulty) => void;
  selectedTool: Piece | 'erase' | null;
  setSelectedTool: (tool: Piece | 'erase' | null) => void;
  onStartSolving: () => void;
  onExitPuzzleMode: () => void;
}

export const PuzzleSetupArea: React.FC<PuzzleSetupAreaProps> = ({
  board,
  setBoard,
  firstTurn,
  setFirstTurn,
  subMode,
  setSubMode,
  playerSide,
  setPlayerSide,
  difficulty,
  setDifficulty,
  selectedTool,
  setSelectedTool,
  onStartSolving,
  onExitPuzzleMode,
}) => {
  const [isFenModalOpen, setIsFenModalOpen] = useState(false);

  // Compute piece counts on the current board
  const pieceCounts = React.useMemo(() => {
    const counts: Record<string, number> = {};
    for (const sq of board) {
      if (sq.piece) {
        const key = `${sq.piece.color}-${sq.piece.type}`;
        counts[key] = (counts[key] ?? 0) + 1;
      }
    }
    return counts;
  }, [board]);

  // Real-time board validation
  const validation = React.useMemo(() => validateBoardSetup(board), [board]);

  // Load a classic preset puzzle
  const handleLoadPreset = (puzzleId: string) => {
    const puzzle = CLASSIC_PUZZLES.find((p) => p.id === puzzleId);
    if (!puzzle) return;
    try {
      const { squares, turn } = fenToBoard(puzzle.fen);
      setBoard(squares);
      setFirstTurn(turn);
      if (puzzle.recommendedMode) {
        setSubMode(puzzle.recommendedMode);
      }
      setSelectedTool(null);
    } catch (e) {
      console.error('Failed to load preset:', e);
    }
  };

  // Clear all pieces
  const handleClearBoard = () => {
    setBoard(createEmptyBoard());
    setSelectedTool(null);
  };

  // Reset to standard Xiangqi 32 pieces
  const handleResetStandard = () => {
    const standardFen =
      'rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w';
    const { squares } = fenToBoard(standardFen);
    setBoard(squares);
    setSelectedTool(null);
  };

  // Flip Red & Black pieces
  const handleFlipSides = () => {
    setBoard((prev) =>
      prev.map((sq) => {
        const oppositeRow = 9 - sq.row;
        const oppositeCol = 8 - sq.col;
        const counterpart = prev.find(
          (s) => s.row === oppositeRow && s.col === oppositeCol
        );
        if (!counterpart?.piece) return { ...sq, piece: null };
        return {
          ...sq,
          piece: {
            type: counterpart.piece.type,
            color: counterpart.piece.color === 'red' ? 'black' : 'red',
          },
        };
      })
    );
  };

  // Apply imported FEN
  const handleApplyFen = (fenInput: string) => {
    const { squares, turn } = fenToBoard(fenInput);
    setBoard(squares);
    setFirstTurn(turn);
  };

  // Export current FEN
  const currentFen = React.useMemo(
    () => boardToFen(board, firstTurn),
    [board, firstTurn]
  );

  return (
    <div className="w-full mx-auto rounded-2xl bg-[#1C1815] border border-amber-600/30 p-3.5 sm:p-4 shadow-2xl flex flex-col gap-3.5 animate-fadeIn">
      {/* Header Banner */}
      <div className="flex flex-wrap items-center justify-between gap-2.5 pb-2.5 border-b border-stone-800">
        <div className="flex items-center gap-2.5">
          <div className="w-7 h-7 rounded-lg bg-amber-500/20 border border-amber-500/40 flex items-center justify-center text-amber-400">
            <Sparkles className="w-4 h-4" />
          </div>
          <div>
            <h2 className="text-sm sm:text-base font-display-serif font-bold text-amber-100 flex items-center gap-2">
              <span>Chế độ: Giải Cờ Thế</span>
              <span className="text-[11px] font-sans font-normal px-2 py-0.5 rounded-full bg-amber-500/15 border border-amber-500/30 text-amber-300">
                Tự sắp xếp quân
              </span>
            </h2>
            <p className="text-xs text-stone-400 hidden sm:block">
              Chọn quân bên dưới rồi nhấp lên bàn cờ để đặt hoặc click chuột phải để xóa quân
            </p>
          </div>
        </div>

        {/* Quick Presets & Utility Tools */}
        <div className="flex flex-wrap items-center gap-2">
          {/* Preset Selector */}
          <div className="relative">
            <select
              aria-label="Thế cờ kinh điển"
              onChange={(e) => {
                if (e.target.value) {
                  handleLoadPreset(e.target.value);
                  e.target.value = '';
                }
              }}
              defaultValue=""
              className="bg-[#26201C] text-amber-300 text-xs font-medium rounded-lg px-2.5 py-1.5 border border-amber-500/40 hover:border-amber-400 focus:outline-none transition-colors cursor-pointer"
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
          </div>

          <button
            type="button"
            onClick={handleClearBoard}
            className="inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs font-medium bg-[#26201C] hover:bg-stone-800 text-stone-300 border border-stone-700/80 transition-colors cursor-pointer"
            title="Xóa hết quân trên bàn cờ"
          >
            <Trash2 className="w-3.5 h-3.5 text-stone-400" />
            <span className="hidden xs:inline">Xóa trắng</span>
          </button>

          <button
            type="button"
            onClick={handleResetStandard}
            className="inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs font-medium bg-[#26201C] hover:bg-stone-800 text-stone-300 border border-stone-700/80 transition-colors cursor-pointer"
            title="Khôi phục bàn cờ chuẩn 32 quân"
          >
            <RotateCcw className="w-3.5 h-3.5 text-amber-400" />
            <span className="hidden xs:inline">Bàn chuẩn</span>
          </button>

          <button
            type="button"
            onClick={handleFlipSides}
            className="inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs font-medium bg-[#26201C] hover:bg-stone-800 text-stone-300 border border-stone-700/80 transition-colors cursor-pointer"
            title="Đảo chiều quân Đỏ và Đen"
          >
            <ArrowRightLeft className="w-3.5 h-3.5 text-amber-400" />
            <span className="hidden sm:inline">Đảo bên</span>
          </button>

          <button
            type="button"
            onClick={() => setIsFenModalOpen(true)}
            className="inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs font-medium bg-[#26201C] hover:bg-stone-800 text-stone-300 border border-stone-700/80 transition-colors cursor-pointer"
            title="Nhập hoặc sao chép chuỗi FEN"
          >
            <FileCode className="w-3.5 h-3.5 text-amber-400" />
            <span>FEN</span>
          </button>

          <button
            type="button"
            onClick={onExitPuzzleMode}
            className="inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs font-medium text-stone-400 hover:text-stone-200 hover:bg-stone-800 transition-colors cursor-pointer"
          >
            <span>Thoát</span>
          </button>
        </div>
      </div>

      {/* Piece Palette */}
      <PuzzlePiecePalette
        pieceCounts={pieceCounts}
        selectedTool={selectedTool}
        setSelectedTool={setSelectedTool}
      />

      {/* Match Setup Controls */}
      <PuzzleMatchSettings
        firstTurn={firstTurn}
        setFirstTurn={setFirstTurn}
        subMode={subMode}
        setSubMode={setSubMode}
        playerSide={playerSide}
        setPlayerSide={setPlayerSide}
        difficulty={difficulty}
        setDifficulty={setDifficulty}
      />

      {/* Validation Status & Start Button */}
      <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3 pt-1">
        {/* Status display */}
        <div className="flex-1">
          {validation.valid ? (
            <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-lg bg-emerald-950/60 border border-emerald-600/50 text-emerald-300 text-xs font-semibold">
              <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
              <span>Thế cờ hợp lệ. Sẵn sàng bắt đầu giải thế!</span>
            </div>
          ) : (
            <div className="flex items-start gap-2 px-3 py-2 rounded-lg bg-red-950/40 border border-red-700/50 text-red-200 text-xs">
              <ShieldAlert className="w-4 h-4 text-red-400 shrink-0 mt-0.5" />
              <div>
                <span className="font-semibold text-red-300">Cần điều chỉnh: </span>
                <span className="text-red-200/90">{validation.errors.join(' · ')}</span>
              </div>
            </div>
          )}
        </div>

        {/* Big Start Button */}
        <button
          type="button"
          onClick={onStartSolving}
          disabled={!validation.valid}
          className={`inline-flex items-center justify-center gap-2 px-6 py-2.5 rounded-xl font-display-serif text-sm font-bold shadow-lg transition-all cursor-pointer shrink-0 ${
            validation.valid
              ? 'bg-gradient-to-r from-amber-600 via-amber-500 to-amber-600 hover:from-amber-500 hover:to-amber-500 text-[#14110F] shadow-amber-500/20 hover:scale-[1.02] ring-2 ring-amber-400/40'
              : 'bg-stone-800 text-stone-500 border border-stone-700/60 cursor-not-allowed opacity-60'
          }`}
        >
          <Play className="w-4 h-4 fill-current" />
          <span>BẮT ĐẦU GIẢI THẾ</span>
        </button>
      </div>

      {/* FEN Import/Export Modal */}
      <FenModal
        isOpen={isFenModalOpen}
        initialFen={currentFen}
        currentFen={currentFen}
        description="Bạn có thể dán chuỗi FEN từ các trang cờ tướng hoặc sách cờ thế để nạp nhanh vị trí quân:"
        onClose={() => setIsFenModalOpen(false)}
        onApplyFen={handleApplyFen}
      />
    </div>
  );
};
