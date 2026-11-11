import React from 'react';
import { AlertTriangle, LayoutGrid, Columns, Rows } from 'lucide-react';
import type { GameStatus, PieceColor } from '../types/game.js';
import type { LayoutOrientation } from '../utils/capturedPieces.js';
import type { LayoutOverride } from '../hooks/useResponsiveLayout.js';

export interface TopHeaderProps {
  layoutOrientation: LayoutOrientation;
  boardSize: number;
  layoutOverride: LayoutOverride;
  setLayoutOverride: (mode: LayoutOverride) => void;
  isInCheck: boolean;
  status: GameStatus;
  isPuzzleSetup: boolean;
  activeTurn: PieceColor;
}

export const TopHeader: React.FC<TopHeaderProps> = ({
  layoutOrientation,
  boardSize,
  layoutOverride,
  setLayoutOverride,
  isInCheck,
  status,
  isPuzzleSetup,
  activeTurn,
}) => {
  return (
    <header
      style={{
        maxWidth:
          layoutOrientation === 'sides'
            ? `${Math.max(900, boardSize + 390)}px`
            : `${Math.max(340, boardSize + 24)}px`,
      }}
      className="w-full mx-auto px-3 sm:px-6 flex items-center justify-between gap-3 sm:gap-4 pb-3 mb-2 sm:mb-3 border-b border-stone-800/80 transition-[max-width] duration-150"
    >
      <div className="flex items-center gap-3">
        <div className="w-9 h-9 rounded-full bg-gradient-to-br from-red-700 to-red-950 border border-amber-400/50 flex items-center justify-center font-display-serif font-bold text-lg text-amber-100 shadow-md shrink-0">
          帥
        </div>
        <div>
          <h1 className="font-display-serif text-lg sm:text-xl font-bold tracking-wide text-[#F5EFE6] leading-tight">
            CỜ TƯỚNG ĐẠI SƯ
          </h1>
          <p className="text-xs text-stone-400">
            Xiangqi AI · WebAssembly Engine
          </p>
        </div>
      </div>

      {/* Layout Switcher & Active Turn & Check Status Indicator */}
      <div className="flex items-center gap-2 sm:gap-3">
        {/* Quick Layout Mode Switcher */}
        <div className="hidden sm:flex items-center bg-[#1E1915] border border-stone-800 rounded-xl p-0.5 text-xs">
          <button
            type="button"
            onClick={() => setLayoutOverride('auto')}
            className={`inline-flex items-center gap-1 px-2 py-1 rounded-lg transition-colors cursor-pointer ${
              layoutOverride === 'auto'
                ? 'bg-amber-500/25 text-amber-200 font-semibold'
                : 'text-stone-400 hover:text-stone-200'
            }`}
            title="Tự động theo tỉ lệ màn hình"
          >
            <LayoutGrid className="w-3.5 h-3.5" />
            <span className="text-[11px]">Tự động</span>
          </button>
          <button
            type="button"
            onClick={() => setLayoutOverride('sides')}
            className={`inline-flex items-center gap-1 px-2 py-1 rounded-lg transition-colors cursor-pointer ${
              layoutOverride === 'sides'
                ? 'bg-amber-500/25 text-amber-200 font-semibold'
                : 'text-stone-400 hover:text-stone-200'
            }`}
            title="Hai bên (Trái - Phải)"
          >
            <Columns className="w-3.5 h-3.5" />
            <span className="text-[11px]">Hai bên</span>
          </button>
          <button
            type="button"
            onClick={() => setLayoutOverride('stacked')}
            className={`inline-flex items-center gap-1 px-2 py-1 rounded-lg transition-colors cursor-pointer ${
              layoutOverride === 'stacked'
                ? 'bg-amber-500/25 text-amber-200 font-semibold'
                : 'text-stone-400 hover:text-stone-200'
            }`}
            title="Trên - Dưới"
          >
            <Rows className="w-3.5 h-3.5" />
            <span className="text-[11px]">Trên dưới</span>
          </button>
        </div>

        {isInCheck && status.status === 'Playing' && !isPuzzleSetup && (
          <div className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-amber-500/20 border border-amber-500/40 text-amber-300 text-xs font-semibold">
            <AlertTriangle className="w-3.5 h-3.5 text-amber-400 shrink-0" />
            <span className="hidden xs:inline">Chiếu tướng!</span>
          </div>
        )}

        <div className="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-[#1E1915] border border-stone-800">
          <span
            className={`w-5 h-5 rounded-full flex items-center justify-center font-display-serif text-xs font-bold ${
              activeTurn === 'red'
                ? 'bg-red-900 text-red-200 border border-red-500/50'
                : 'bg-stone-800 text-stone-200 border border-stone-600'
            }`}
          >
            {activeTurn === 'red' ? '帥' : '將'}
          </span>
          <div className="text-xs">
            <span className="text-stone-400 hidden xs:inline">
              {isPuzzleSetup ? 'Đi tiên: ' : 'Lượt đi: '}
            </span>
            <span
              className={
                activeTurn === 'red'
                  ? 'font-semibold text-red-400'
                  : 'font-semibold text-amber-300'
              }
            >
              {activeTurn === 'red' ? 'Bên Đỏ' : 'Bên Đen'}
            </span>
          </div>
        </div>
      </div>
    </header>
  );
};
