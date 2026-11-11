import React from 'react';
import { Trophy, Handshake, RotateCcw } from 'lucide-react';
import type { GameMode, GameStatus } from '../types/game.js';

export interface GameOverOverlayProps {
  status: GameStatus;
  isPuzzleSetup: boolean;
  gameMode: GameMode;
  moveCount: number;
  onRestartPuzzle: () => void;
  onEditPuzzlePosition: () => void;
  onNewGame: () => void;
}

export const GameOverOverlay: React.FC<GameOverOverlayProps> = ({
  status,
  isPuzzleSetup,
  gameMode,
  moveCount,
  onRestartPuzzle,
  onEditPuzzlePosition,
  onNewGame,
}) => {
  if (status.status === 'Playing' || isPuzzleSetup) {
    return null;
  }

  return (
    <div className="absolute inset-0 z-30 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs rounded-2xl animate-board-overlay">
      <div className="w-full max-w-[320px] rounded-2xl bg-[#1A1512] border border-amber-500/40 shadow-2xl p-5 text-center">
        {status.status === 'Checkmate' ? (
          <>
            <div className="w-12 h-12 rounded-full bg-amber-500/15 border border-amber-500/35 flex items-center justify-center mx-auto mb-3 text-amber-400">
              <Trophy className="w-6 h-6" />
            </div>
            <h2 className="font-display-serif text-lg font-bold text-[#F5EFE6]">
              {status.winner === 'red'
                ? 'Bên Đỏ Chiến Thắng!'
                : 'Bên Đen Chiến Thắng!'}
            </h2>
            <p className="text-xs text-stone-400 mt-1">
              {gameMode === 'Puzzle'
                ? `Giải cờ thế thành công sau ${moveCount} nước!`
                : `Chiếu bí (Checkmate) sau ${moveCount} nước đi`}
            </p>
          </>
        ) : (
          <>
            <div className="w-12 h-12 rounded-full bg-stone-800 border border-stone-700 flex items-center justify-center mx-auto mb-3 text-stone-300">
              <Handshake className="w-6 h-6" />
            </div>
            <h2 className="font-display-serif text-lg font-bold text-[#F5EFE6]">
              Ván Cờ Hòa!
            </h2>
            <p className="text-xs text-stone-400 mt-1">
              Hết nước đi hợp lệ (Stalemate)
            </p>
          </>
        )}

        {gameMode === 'Puzzle' ? (
          <div className="mt-4 flex flex-col gap-2">
            <button
              type="button"
              onClick={onRestartPuzzle}
              className="w-full inline-flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl text-xs font-semibold bg-amber-600 hover:bg-amber-500 text-white transition-colors cursor-pointer shadow-md"
            >
              <RotateCcw className="w-4 h-4" />
              <span>Giải lại từ đầu</span>
            </button>
            <button
              type="button"
              onClick={onEditPuzzlePosition}
              className="w-full inline-flex items-center justify-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold bg-[#26201C] hover:bg-stone-800 text-stone-200 border border-stone-700 transition-colors cursor-pointer"
            >
              <span>Xếp lại cờ thế</span>
            </button>
          </div>
        ) : (
          <button
            type="button"
            onClick={onNewGame}
            className="mt-4 w-full inline-flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl text-xs font-semibold bg-[#B91C1C] hover:bg-[#DC2626] text-white transition-colors cursor-pointer shadow-md"
          >
            <RotateCcw className="w-4 h-4" />
            <span>Chơi ván mới</span>
          </button>
        )}
      </div>
    </div>
  );
};
