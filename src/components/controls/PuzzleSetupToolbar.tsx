import React from 'react';
import {
  Play,
  Trash2,
  FileCode,
  ShieldAlert,
  LayoutGrid,
} from 'lucide-react';
import type { PieceColor } from '../../types/game.js';
import { CLASSIC_PUZZLES } from '../../utils/fen.js';

export interface PuzzleSetupToolbarProps {
  validationValid: boolean;
  puzzleFirstTurn: PieceColor;
  setPuzzleFirstTurn?: (turn: PieceColor) => void;
  onStartSolving?: () => void;
  onLoadPreset: (puzzleId: string) => void;
  onClearPuzzleBoard?: () => void;
  onResetStandardBoard?: () => void;
  onOpenFenModal: () => void;
}

export const PuzzleSetupToolbar: React.FC<PuzzleSetupToolbarProps> = ({
  validationValid,
  puzzleFirstTurn,
  setPuzzleFirstTurn,
  onStartSolving,
  onLoadPreset,
  onClearPuzzleBoard,
  onResetStandardBoard,
  onOpenFenModal,
}) => {
  return (
    <>
      {/* Start Solving Button */}
      <button
        type="button"
        onClick={onStartSolving}
        disabled={!validationValid}
        className={`inline-flex items-center gap-1.5 px-4 py-2 rounded-lg text-xs font-semibold transition-all cursor-pointer whitespace-nowrap shrink-0 shadow-sm ${
          validationValid
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
            onLoadPreset(e.target.value);
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
        onClick={onOpenFenModal}
        className="inline-flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-semibold bg-[#26201C] hover:bg-[#312923] text-stone-200 border border-stone-700/80 transition-all cursor-pointer whitespace-nowrap shrink-0"
        title="Nhập hoặc sao chép chuỗi FEN"
      >
        <FileCode className="w-3.5 h-3.5 text-amber-400" />
        <span>FEN</span>
      </button>
    </>
  );
};

export const PuzzleValidationStrip: React.FC<{
  valid: boolean;
  errors: string[];
}> = ({ valid, errors }) => (
  <div className="pt-1 border-t border-stone-800/60 flex flex-wrap items-center justify-between gap-2 text-xs">
    {!valid ? (
      <div className="flex items-center gap-1.5 text-red-300 font-medium">
        <ShieldAlert className="w-3.5 h-3.5 text-red-400 shrink-0" />
        <span>{errors.join(' · ')}</span>
      </div>
    ) : (
      <span className="text-stone-400">
        Kéo quân từ khay 2 bên vào bàn cờ · Giữ chuột trái để dời quân trên bàn · Chuột phải để xóa quân
      </span>
    )}
  </div>
);
