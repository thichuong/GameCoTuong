import React from 'react';
import { Eraser } from 'lucide-react';
import type { Piece, PieceType } from '../../types/game.js';

export const RED_PIECES: {
  type: PieceType;
  label: string;
  symbol: string;
  max: number;
}[] = [
  { type: 'General', label: 'Tướng', symbol: '帥', max: 1 },
  { type: 'Advisor', label: 'Sĩ', symbol: '仕', max: 2 },
  { type: 'Elephant', label: 'Tượng', symbol: '相', max: 2 },
  { type: 'Horse', label: 'Mã', symbol: '傌', max: 2 },
  { type: 'Chariot', label: 'Xe', symbol: '俥', max: 2 },
  { type: 'Cannon', label: 'Pháo', symbol: '炮', max: 2 },
  { type: 'Soldier', label: 'Binh', symbol: '兵', max: 5 },
];

export const BLACK_PIECES: {
  type: PieceType;
  label: string;
  symbol: string;
  max: number;
}[] = [
  { type: 'General', label: 'Tướng', symbol: '將', max: 1 },
  { type: 'Advisor', label: 'Sĩ', symbol: '士', max: 2 },
  { type: 'Elephant', label: 'Tượng', symbol: '象', max: 2 },
  { type: 'Horse', label: 'Mã', symbol: '馬', max: 2 },
  { type: 'Chariot', label: 'Xe', symbol: '車', max: 2 },
  { type: 'Cannon', label: 'Pháo', symbol: '砲', max: 2 },
  { type: 'Soldier', label: 'Tốt', symbol: '卒', max: 5 },
];

interface PuzzlePiecePaletteProps {
  pieceCounts: Record<string, number>;
  selectedTool: Piece | 'erase' | null;
  setSelectedTool: (tool: Piece | 'erase' | null) => void;
}

export const PuzzlePiecePalette: React.FC<PuzzlePiecePaletteProps> = ({
  pieceCounts,
  selectedTool,
  setSelectedTool,
}) => {
  return (
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
              <span className="text-[11px] font-sans font-normal opacity-90">
                {item.label}
              </span>
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
              <span className="text-[11px] font-sans font-normal opacity-90">
                {item.label}
              </span>
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
  );
};
