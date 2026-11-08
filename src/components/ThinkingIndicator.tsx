import React, { useEffect, useState } from 'react';
import { Cpu, Clock, Layers, Gauge } from 'lucide-react';
import type { Difficulty, PieceColor, SearchStats } from '../types/game.js';

interface ThinkingIndicatorProps {
  isThinking: boolean;
  turn?: PieceColor;
  difficulty?: Difficulty;
  lastStats?: SearchStats | null;
}

const TARGET_TIME_MS: Record<Difficulty, number> = {
  1: 1000,
  2: 2000,
  3: 5000,
  4: 10000,
  5: 20000,
};

const DIFFICULTY_LABELS: Record<Difficulty, string> = {
  1: 'Nhập môn',
  2: 'Cơ bản',
  3: 'Trung cấp',
  4: 'Cao thủ',
  5: 'Đại sư',
};

export const ThinkingIndicator: React.FC<ThinkingIndicatorProps> = ({
  isThinking,
  turn = 'black',
  difficulty = 1,
  lastStats = null,
}) => {
  const [elapsedMs, setElapsedMs] = useState(0);

  useEffect(() => {
    if (!isThinking) {
      setElapsedMs(0);
      return;
    }

    const start = performance.now();
    const interval = window.setInterval(() => {
      setElapsedMs(Math.max(0, Math.floor(performance.now() - start)));
    }, 100);

    return () => window.clearInterval(interval);
  }, [isThinking]);

  if (!isThinking) return null;

  const targetMs = TARGET_TIME_MS[difficulty] ?? 1000;
  const progressPct = Math.min(96, Math.round((elapsedMs / targetMs) * 100));
  const isRed = turn === 'red';
  const elapsedSec = (elapsedMs / 1000).toFixed(1);
  const targetSec = (targetMs / 1000).toFixed(0);

  return (
    <div
      className="absolute inset-0 z-20 pointer-events-none flex items-center justify-center p-4 animate-board-overlay"
      role="status"
      aria-live="polite"
    >
      {/* Subtle board-wide dim scrim that keeps pieces visible underneath */}
      <div className="absolute inset-0 bg-gradient-to-b from-black/25 via-black/40 to-black/25 rounded-xl" />

      {/* Central Overview HUD Card on the Board */}
      <div className="relative w-full max-w-[340px] rounded-2xl bg-[#181411]/90 backdrop-blur-md border border-amber-500/35 shadow-[0_20px_50px_rgba(0,0,0,0.75)] px-5 py-4 text-[#F5EFE6]">
        {/* Top Row: AI Side Identity + Live Elapsed Timer */}
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <div className="relative flex items-center justify-center w-11 h-11 shrink-0">
              {/* Spinning ring */}
              <div
                className={`absolute inset-0 rounded-full border-2 border-t-transparent animate-spin ${
                  isRed ? 'border-red-400' : 'border-amber-400'
                }`}
              />
              {/* Traditional Piece Seal */}
              <div
                className={`w-8 h-8 rounded-full flex items-center justify-center font-display-serif font-bold text-base shadow-inner ${
                  isRed
                    ? 'bg-red-950/90 text-red-300 border border-red-500/40'
                    : 'bg-stone-800 text-amber-200 border border-amber-500/30'
                }`}
              >
                {isRed ? '帥' : '將'}
              </div>
            </div>

            <div>
              <div className="flex items-center gap-1.5">
                <Cpu className="w-3.5 h-3.5 text-amber-400 shrink-0" />
                <span className="text-xs font-semibold tracking-wider text-amber-300/90">
                  AI OVERVIEW
                </span>
              </div>
              <p className="text-sm font-bold text-[#F5EFE6] mt-0.5">
                {isRed ? 'AI Đỏ đang tính nước...' : 'AI Đen đang tính nước...'}
              </p>
            </div>
          </div>

          <div className="text-right shrink-0">
            <div className="inline-flex items-center gap-1 text-amber-300 font-mono-tabular text-sm font-semibold">
              <Clock className="w-3.5 h-3.5 text-amber-400/80" />
              <span>{elapsedSec}s</span>
            </div>
            <div className="text-[11px] text-stone-400 font-mono-tabular">
              / {targetSec}s tối đa
            </div>
          </div>
        </div>

        {/* Progress Bar */}
        <div className="mt-3.5 w-full h-1.5 bg-stone-800/90 rounded-full overflow-hidden">
          <div
            className={`h-full transition-all duration-150 rounded-full ${
              isRed
                ? 'bg-gradient-to-r from-red-500 to-amber-400'
                : 'bg-gradient-to-r from-amber-500 to-yellow-300'
            }`}
            style={{ width: `${Math.max(8, progressPct)}%` }}
          />
        </div>

        {/* Bottom Telemetry Overview Strip */}
        <div className="mt-3 pt-2.5 border-t border-stone-800/90 flex items-center justify-between text-xs text-stone-300">
          <div className="flex items-center gap-1.5">
            <Gauge className="w-3.5 h-3.5 text-amber-400/80" />
            <span>Cấp {difficulty}</span>
            <span aria-hidden="true" className="text-stone-600">·</span>
            <span className="text-stone-400">{DIFFICULTY_LABELS[difficulty]}</span>
          </div>

          <div className="flex items-center gap-1.5 font-mono-tabular text-[11px] text-stone-400">
            <Layers className="w-3.5 h-3.5 text-amber-400/80" />
            {lastStats ? (
              <span>
                Độ sâu {lastStats.depth} · {(lastStats.nodes / 1000).toFixed(1)}k nodes ·{' '}
                {lastStats.time_ms > 0
                  ? `${Math.round((lastStats.nodes / lastStats.time_ms) * 1000 / 1000)}k NPS`
                  : 'N/A'}
              </span>
            ) : (
              <span>Alpha-Beta WASM</span>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
