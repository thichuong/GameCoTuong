import React from 'react';
import { User, Bot, Users } from 'lucide-react';
import type { Difficulty, PieceColor, PuzzleSubMode } from '../../types/game.js';

interface PuzzleMatchSettingsProps {
  firstTurn: PieceColor;
  setFirstTurn: (turn: PieceColor) => void;
  subMode: PuzzleSubMode;
  setSubMode: (mode: PuzzleSubMode) => void;
  playerSide: PieceColor;
  setPlayerSide: (side: PieceColor) => void;
  difficulty: Difficulty;
  setDifficulty: (diff: Difficulty) => void;
}

export const PuzzleMatchSettings: React.FC<PuzzleMatchSettingsProps> = ({
  firstTurn,
  setFirstTurn,
  subMode,
  setSubMode,
  playerSide,
  setPlayerSide,
  difficulty,
  setDifficulty,
}) => {
  return (
    <>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-1">
        {/* Box 1: Chọn phe đi trước */}
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

        {/* Box 2: Chọn chế độ thi đấu */}
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
    </>
  );
};
