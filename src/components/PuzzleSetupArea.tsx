import React, { useState } from 'react';
import {
  Play,
  Trash2,
  RotateCcw,
  ArrowRightLeft,
  FileCode,
  ShieldAlert,
  CheckCircle2,
  User,
  Bot,
  Users,
  Sparkles,
  Eraser,
  Copy,
  Check,
} from 'lucide-react';
import type {
  Difficulty,
  Piece,
  PieceColor,
  PieceType,
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

interface PuzzleSetupAreaProps {
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

const RED_PIECES: { type: PieceType; label: string; symbol: string; max: number }[] = [
  { type: 'General', label: 'Tướng', symbol: '帥', max: 1 },
  { type: 'Advisor', label: 'Sĩ', symbol: '仕', max: 2 },
  { type: 'Elephant', label: 'Tượng', symbol: '相', max: 2 },
  { type: 'Horse', label: 'Mã', symbol: '傌', max: 2 },
  { type: 'Chariot', label: 'Xe', symbol: '俥', max: 2 },
  { type: 'Cannon', label: 'Pháo', symbol: '炮', max: 2 },
  { type: 'Soldier', label: 'Binh', symbol: '兵', max: 5 },
];

const BLACK_PIECES: { type: PieceType; label: string; symbol: string; max: number }[] = [
  { type: 'General', label: 'Tướng', symbol: '將', max: 1 },
  { type: 'Advisor', label: 'Sĩ', symbol: '士', max: 2 },
  { type: 'Elephant', label: 'Tượng', symbol: '象', max: 2 },
  { type: 'Horse', label: 'Mã', symbol: '馬', max: 2 },
  { type: 'Chariot', label: 'Xe', symbol: '車', max: 2 },
  { type: 'Cannon', label: 'Pháo', symbol: '砲', max: 2 },
  { type: 'Soldier', label: 'Tốt', symbol: '卒', max: 5 },
];

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
  const [fenInput, setFenInput] = useState('');
  const [fenError, setFenError] = useState<string | null>(null);
  const [isFenModalOpen, setIsFenModalOpen] = useState(false);
  const [copiedFen, setCopiedFen] = useState(false);

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
    const standardFen = 'rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w';
    const { squares } = fenToBoard(standardFen);
    setBoard(squares);
    setSelectedTool(null);
  };

  // Flip Red & Black pieces
  const handleFlipSides = () => {
    setBoard((prev) =>
      prev.map((sq) => {
        // Find opposite row (9 - row) and col (8 - col)
        const oppositeRow = 9 - sq.row;
        const oppositeCol = 8 - sq.col;
        const counterpart = prev.find((s) => s.row === oppositeRow && s.col === oppositeCol);
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
  const handleApplyFen = () => {
    try {
      setFenError(null);
      const { squares, turn } = fenToBoard(fenInput);
      setBoard(squares);
      setFirstTurn(turn);
      setIsFenModalOpen(false);
    } catch (err: unknown) {
      setFenError(err instanceof Error ? err.message : 'FEN không hợp lệ');
    }
  };

  // Export current FEN
  const currentFen = React.useMemo(() => boardToFen(board, firstTurn), [board, firstTurn]);

  const handleCopyFen = () => {
    navigator.clipboard.writeText(currentFen).then(() => {
      setCopiedFen(true);
      setTimeout(() => setCopiedFen(false), 2000);
    });
  };

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
            onClick={() => {
              setFenInput(currentFen);
              setIsFenModalOpen(true);
            }}
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

      {/* Piece Palette: Red pieces, Black pieces, Eraser */}
      <div className="bg-[#14110F] border border-stone-800 rounded-xl p-2.5 sm:p-3 flex flex-col gap-2.5">
        <div className="flex items-center justify-between">
          <span className="text-[11px] font-semibold text-stone-400 uppercase tracking-wider">
            Kho quân cờ (Nhấp để chọn đặt lên bàn cờ):
          </span>
          <button
            type="button"
            onClick={() => setSelectedTool(selectedTool === 'erase' ? null : 'erase')}
            className={`inline-flex items-center gap-1.5 px-3 py-1 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
              selectedTool === 'erase'
                ? 'bg-red-600 text-white shadow-md ring-2 ring-red-400/50'
                : 'bg-stone-800 text-stone-300 hover:bg-stone-700 border border-stone-700'
            }`}
          >
            <Eraser className="w-3.5 h-3.5" />
            <span>Xóa quân (Tẩy)</span>
          </button>
        </div>

        {/* Red Pieces Row */}
        <div className="flex flex-wrap items-center gap-1.5 sm:gap-2">
          <span className="text-xs font-bold text-red-400 min-w-[55px] flex items-center gap-1">
            <span className="w-2.5 h-2.5 rounded-full bg-red-600 inline-block" />
            <span>Quân Đỏ:</span>
          </span>
          {RED_PIECES.map((item) => {
            const count = pieceCounts[`red-${item.type}`] ?? 0;
            const isSelected =
              selectedTool !== 'erase' &&
              selectedTool?.type === item.type &&
              selectedTool?.color === 'red';

            return (
              <button
                key={`red-${item.type}`}
                type="button"
                onClick={() => {
                  if (isSelected) {
                    setSelectedTool(null);
                  } else {
                    setSelectedTool({ type: item.type, color: 'red' });
                  }
                }}
                className={`relative inline-flex items-center justify-center gap-1 px-2.5 py-1.5 rounded-xl border text-xs font-display-serif font-bold transition-all cursor-pointer ${
                  isSelected
                    ? 'bg-red-900/80 text-amber-200 border-amber-400 shadow-[0_0_12px_rgba(239,68,68,0.5)] scale-105 ring-2 ring-amber-400/60'
                    : 'bg-[#241715] text-red-200 border-red-800/60 hover:border-red-500/80 hover:bg-red-950/40'
                }`}
                title={`Đặt ${item.label} Đỏ (${count}/${item.max})`}
              >
                <span className="text-base leading-none">{item.symbol}</span>
                <span className="text-[11px] font-sans font-normal opacity-90">{item.label}</span>
                <span
                  className={`text-[10px] font-mono-tabular px-1 rounded-sm ml-0.5 ${
                    count > item.max
                      ? 'bg-amber-500/30 text-amber-300'
                      : 'bg-black/30 text-stone-400'
                  }`}
                >
                  {count}
                </span>
              </button>
            );
          })}
        </div>

        {/* Black Pieces Row */}
        <div className="flex flex-wrap items-center gap-1.5 sm:gap-2">
          <span className="text-xs font-bold text-stone-300 min-w-[55px] flex items-center gap-1">
            <span className="w-2.5 h-2.5 rounded-full bg-stone-600 inline-block" />
            <span>Quân Đen:</span>
          </span>
          {BLACK_PIECES.map((item) => {
            const count = pieceCounts[`black-${item.type}`] ?? 0;
            const isSelected =
              selectedTool !== 'erase' &&
              selectedTool?.type === item.type &&
              selectedTool?.color === 'black';

            return (
              <button
                key={`black-${item.type}`}
                type="button"
                onClick={() => {
                  if (isSelected) {
                    setSelectedTool(null);
                  } else {
                    setSelectedTool({ type: item.type, color: 'black' });
                  }
                }}
                className={`relative inline-flex items-center justify-center gap-1 px-2.5 py-1.5 rounded-xl border text-xs font-display-serif font-bold transition-all cursor-pointer ${
                  isSelected
                    ? 'bg-stone-800 text-amber-200 border-amber-400 shadow-[0_0_12px_rgba(245,158,11,0.5)] scale-105 ring-2 ring-amber-400/60'
                    : 'bg-[#1E1C1A] text-stone-200 border-stone-700/80 hover:border-stone-500 hover:bg-stone-800'
                }`}
                title={`Đặt ${item.label} Đen (${count}/${item.max})`}
              >
                <span className="text-base leading-none">{item.symbol}</span>
                <span className="text-[11px] font-sans font-normal opacity-90">{item.label}</span>
                <span
                  className={`text-[10px] font-mono-tabular px-1 rounded-sm ml-0.5 ${
                    count > item.max
                      ? 'bg-amber-500/30 text-amber-300'
                      : 'bg-black/30 text-stone-400'
                  }`}
                >
                  {count}
                </span>
              </button>
            );
          })}
        </div>
      </div>

      {/* Match Setup Controls (First turn, sub-mode, player side, difficulty) */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-1">
        {/* Box 1: Chọn phe đi trước (Requirement 3) */}
        <div className="bg-[#181412] border border-stone-800/90 rounded-xl p-3 flex flex-col gap-2">
          <label className="text-xs font-semibold text-stone-300 flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-amber-400" />
            <span>Chọn phe đi trước:</span>
          </label>
          <div className="grid grid-cols-2 gap-2">
            <button
              type="button"
              onClick={() => setFirstTurn('red')}
              className={`inline-flex items-center justify-center gap-2 px-3 py-2 rounded-lg text-xs font-semibold border transition-all cursor-pointer ${
                firstTurn === 'red'
                  ? 'bg-red-950/80 border-red-500 text-red-200 shadow-sm ring-1 ring-red-400/40'
                  : 'bg-[#241F1C] border-stone-700/80 text-stone-400 hover:text-stone-200'
              }`}
            >
              <span className="w-3.5 h-3.5 rounded-full bg-red-600 flex items-center justify-center text-[9px] font-bold text-white">
                帥
              </span>
              <span>Bên Đỏ đi trước</span>
            </button>

            <button
              type="button"
              onClick={() => setFirstTurn('black')}
              className={`inline-flex items-center justify-center gap-2 px-3 py-2 rounded-lg text-xs font-semibold border transition-all cursor-pointer ${
                firstTurn === 'black'
                  ? 'bg-stone-800 border-amber-400 text-amber-200 shadow-sm ring-1 ring-amber-400/40'
                  : 'bg-[#241F1C] border-stone-700/80 text-stone-400 hover:text-stone-200'
              }`}
            >
              <span className="w-3.5 h-3.5 rounded-full bg-stone-700 flex items-center justify-center text-[9px] font-bold text-stone-200">
                將
              </span>
              <span>Bên Đen đi trước</span>
            </button>
          </div>
        </div>

        {/* Box 2: Chọn chế độ thi đấu (Requirement 4: Máy vs Máy, Máy vs Người, Người vs Người) */}
        <div className="bg-[#181412] border border-stone-800/90 rounded-xl p-3 flex flex-col gap-2">
          <label className="text-xs font-semibold text-stone-300 flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-amber-400" />
            <span>Chế độ giải thế cờ:</span>
          </label>
          <div className="grid grid-cols-3 gap-1.5 sm:gap-2">
            <button
              type="button"
              onClick={() => setSubMode('HumanVsComputer')}
              className={`inline-flex flex-col items-center justify-center p-2 rounded-lg text-xs font-semibold border transition-all cursor-pointer ${
                subMode === 'HumanVsComputer'
                  ? 'bg-amber-500/20 border-amber-400 text-amber-200 shadow-xs'
                  : 'bg-[#241F1C] border-stone-700/80 text-stone-400 hover:text-stone-200'
              }`}
            >
              <div className="flex items-center gap-1 mb-0.5">
                <User className="w-3.5 h-3.5 text-amber-400" />
                <span className="text-[10px] text-stone-400">vs</span>
                <Bot className="w-3.5 h-3.5 text-red-400" />
              </div>
              <span className="text-[11px] whitespace-nowrap">Máy vs Người</span>
            </button>

            <button
              type="button"
              onClick={() => setSubMode('ComputerVsComputer')}
              className={`inline-flex flex-col items-center justify-center p-2 rounded-lg text-xs font-semibold border transition-all cursor-pointer ${
                subMode === 'ComputerVsComputer'
                  ? 'bg-amber-500/20 border-amber-400 text-amber-200 shadow-xs'
                  : 'bg-[#241F1C] border-stone-700/80 text-stone-400 hover:text-stone-200'
              }`}
            >
              <div className="flex items-center gap-1 mb-0.5">
                <Bot className="w-3.5 h-3.5 text-red-400" />
                <span className="text-[10px] text-stone-400">vs</span>
                <Bot className="w-3.5 h-3.5 text-stone-300" />
              </div>
              <span className="text-[11px] whitespace-nowrap">Máy vs Máy</span>
            </button>

            <button
              type="button"
              onClick={() => setSubMode('HumanVsHuman')}
              className={`inline-flex flex-col items-center justify-center p-2 rounded-lg text-xs font-semibold border transition-all cursor-pointer ${
                subMode === 'HumanVsHuman'
                  ? 'bg-amber-500/20 border-amber-400 text-amber-200 shadow-xs'
                  : 'bg-[#241F1C] border-stone-700/80 text-stone-400 hover:text-stone-200'
              }`}
            >
              <div className="flex items-center gap-1 mb-0.5">
                <Users className="w-3.5 h-3.5 text-amber-400" />
              </div>
              <span className="text-[11px] whitespace-nowrap">Người vs Người</span>
            </button>
          </div>
        </div>
      </div>

      {/* Sub-options for Máy vs Người or AI difficulty */}
      {subMode === 'HumanVsComputer' && (
        <div className="bg-[#181412] border border-stone-800/90 rounded-xl p-3 flex flex-wrap items-center justify-between gap-3 text-xs">
          <div className="flex items-center gap-2">
            <span className="text-stone-400">Bạn điều khiển phe:</span>
            <div className="inline-flex rounded-lg bg-[#241F1C] p-0.5 border border-stone-700">
              <button
                type="button"
                onClick={() => setPlayerSide('red')}
                className={`px-2.5 py-1 rounded-md font-semibold transition-colors cursor-pointer ${
                  playerSide === 'red'
                    ? 'bg-red-700 text-white shadow-xs'
                    : 'text-stone-400 hover:text-stone-200'
                }`}
              >
                🔴 Bên Đỏ
              </button>
              <button
                type="button"
                onClick={() => setPlayerSide('black')}
                className={`px-2.5 py-1 rounded-md font-semibold transition-colors cursor-pointer ${
                  playerSide === 'black'
                    ? 'bg-stone-700 text-white shadow-xs'
                    : 'text-stone-400 hover:text-stone-200'
                }`}
              >
                ⚫ Bên Đen
              </button>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <span className="text-stone-400">Độ khó AI:</span>
            <select
              aria-label="Độ khó AI"
              value={difficulty}
              onChange={(e) => setDifficulty(Number(e.target.value) as Difficulty)}
              className="bg-[#241F1C] text-amber-300 font-medium rounded-lg px-2.5 py-1 border border-stone-700 focus:outline-none cursor-pointer"
            >
              <option value={1}>Cấp 1 · Nhập môn (1s)</option>
              <option value={2}>Cấp 2 · Cơ bản (2s)</option>
              <option value={3}>Cấp 3 · Trung cấp (5s)</option>
              <option value={4}>Cấp 4 · Cao thủ (10s)</option>
              <option value={5}>Cấp 5 · Đại sư (20s)</option>
            </select>
          </div>
        </div>
      )}

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
              Bạn có thể dán chuỗi FEN từ các trang cờ tướng hoặc sách cờ thế để nạp nhanh vị trí quân:
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
