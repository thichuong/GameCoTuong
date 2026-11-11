import React from 'react';
import { BrainCircuit, Cpu, Database, Flame } from 'lucide-react';
import type { EngineConfig } from '../../types/game.js';
import { getDefaultThreads } from '../../types/game.js';
import { formatRamMb } from './configPresets.js';

interface ConfigParamsSectionProps {
  currentConfig: EngineConfig;
  updateParam: <K extends keyof EngineConfig>(key: K, value: EngineConfig[K]) => void;
}

export const ConfigParamsSection: React.FC<ConfigParamsSectionProps> = ({
  currentConfig,
  updateParam,
}) => {
  const hardwareCores =
    typeof navigator !== 'undefined' ? navigator.hardwareConcurrency || 8 : 8;
  const maxThreads = Math.max(4, Math.min(16, hardwareCores));
  const activeThreads = currentConfig.threads || getDefaultThreads();
  const sabMb = currentConfig.tt_size_mb * activeThreads;
  const multiThreadTotalRam = sabMb * 2;
  const quickThreadOptions = Array.from(
    new Set([1, getDefaultThreads(), 2, 4, 8].filter((t) => t <= hardwareCores))
  ).sort((a, b) => a - b);

  return (
    <section className="space-y-4">
      <div className="flex items-center gap-2 pb-2 border-b border-stone-800">
        <BrainCircuit className="w-4 h-4 text-amber-400" />
        <h3 className="text-xs font-semibold tracking-wider uppercase text-amber-400">
          Tham số cốt lõi trong EngineConfig (Rust struct)
        </h3>
      </div>

      {/* 1. Transposition Table Size */}
      <div className="bg-[#221D19] p-4 rounded-xl border border-stone-800/90 space-y-3">
        <div className="flex items-start justify-between gap-3">
          <div className="flex items-center gap-2">
            <Database className="w-4 h-4 text-amber-400 shrink-0" />
            <div>
              <div className="flex items-center gap-2">
                <span className="text-xs font-semibold text-stone-200">
                  Transposition Table (mỗi luồng)
                </span>
                <code className="text-[11px] font-mono text-amber-300 bg-stone-900 px-1.5 py-0.5 rounded border border-stone-700/60">
                  tt_size_mb: usize
                </code>
              </div>
              <p className="text-[11px] text-stone-400 mt-0.5">
                {activeThreads === 1 ? (
                  <>
                    Dung lượng Transposition Table cho 1 luồng đơn. Tổng RAM AI sử dụng ước tính là{' '}
                    <strong className="text-amber-400">{currentConfig.tt_size_mb} MB</strong>.
                  </>
                ) : (
                  <>
                    Dung lượng Transposition Table cho <strong>mỗi</strong> Web Worker (
                    {currentConfig.tt_size_mb} MB/luồng). Khi chạy {activeThreads} luồng, tổng RAM ước tính là{' '}
                    <strong className="text-amber-400">{formatRamMb(multiThreadTotalRam)}</strong>{' '}
                    (gồm <span className="text-stone-300 font-mono">{sabMb} MB</span> SharedArrayBuffer +{' '}
                    <span className="text-stone-300 font-mono">{sabMb} MB</span> cho {activeThreads} bảng TT riêng).
                  </>
                )}
              </p>
            </div>
          </div>
          <div className="text-right shrink-0">
            <div className="text-sm font-mono font-bold text-amber-400">
              {currentConfig.tt_size_mb} MB / luồng
            </div>
            <div className="text-[11px] font-mono text-stone-400 mt-0.5">
              {activeThreads === 1 ? (
                <span>
                  Tổng RAM: <strong className="text-stone-200">{currentConfig.tt_size_mb} MB</strong> (1 luồng)
                </span>
              ) : (
                <span>
                  Tổng RAM: <strong className="text-stone-200">{formatRamMb(multiThreadTotalRam)}</strong>
                </span>
              )}
            </div>
          </div>
        </div>

        <input
          type="range"
          min={16}
          max={1024}
          step={32}
          value={currentConfig.tt_size_mb}
          onChange={(e) => updateParam('tt_size_mb', Number(e.target.value))}
          className="w-full accent-amber-500 cursor-pointer h-1.5 bg-stone-800 rounded-lg"
        />

        <div className="flex flex-wrap items-center justify-between gap-1.5 pt-1">
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="text-[10px] text-stone-400 mr-1">Mức nhanh:</span>
            {[64, 128, 256, 512, 1024].map((size) => (
              <button
                key={size}
                type="button"
                onClick={() => updateParam('tt_size_mb', size)}
                className={`px-2 py-0.5 rounded text-[11px] font-mono transition-colors cursor-pointer border ${
                  currentConfig.tt_size_mb === size
                    ? 'bg-amber-500 text-stone-950 font-bold border-amber-400'
                    : 'bg-stone-800 hover:bg-stone-700 text-stone-300 border-stone-700'
                }`}
              >
                {size} MB {size === 256 && '(Mặc định)'}
              </button>
            ))}
          </div>
          <div className="text-[10px] font-mono text-amber-400/80 bg-stone-900/80 px-2 py-0.5 rounded border border-stone-800">
            {activeThreads === 1 ? (
              <span>RAM ước tính: {currentConfig.tt_size_mb} MB (1 luồng)</span>
            ) : (
              <span>
                RAM ước tính: {formatRamMb(multiThreadTotalRam)} ({currentConfig.tt_size_mb}MB × {activeThreads} luồng × 2)
              </span>
            )}
          </div>
        </div>
      </div>

      {/* 2. Checkmate Score Base */}
      <div className="bg-[#221D19] p-4 rounded-xl border border-stone-800/90 space-y-3">
        <div className="flex items-start justify-between gap-3">
          <div className="flex items-center gap-2">
            <Flame className="w-4 h-4 text-red-400 shrink-0" />
            <div>
              <div className="flex items-center gap-2">
                <span className="text-xs font-semibold text-stone-200">
                  Điểm Chiếu Bí Cơ Sở
                </span>
                <code className="text-[11px] font-mono text-amber-300 bg-stone-900 px-1.5 py-0.5 rounded border border-stone-700/60">
                  mate_score: i32
                </code>
              </div>
              <p className="text-[11px] text-stone-400 mt-0.5">
                Điểm cơ sở khi phát hiện chiếu bí. Điểm thực tế tính theo công thức{' '}
                <span className="font-mono text-stone-300">mate_score - ply</span> (càng sát nước thắng điểm càng cao).
              </p>
            </div>
          </div>
          <div className="text-right shrink-0">
            <span className="text-sm font-mono font-bold text-amber-400">
              {currentConfig.mate_score.toLocaleString('vi-VN')}
            </span>
          </div>
        </div>

        <input
          type="range"
          min={50000}
          max={500000}
          step={10000}
          value={currentConfig.mate_score}
          onChange={(e) => updateParam('mate_score', Number(e.target.value))}
          className="w-full accent-amber-500 cursor-pointer h-1.5 bg-stone-800 rounded-lg"
        />

        <div className="flex flex-wrap items-center gap-1.5 pt-1">
          <span className="text-[10px] text-stone-400 mr-1">Mức nhanh:</span>
          {[100000, 200000, 300000, 400000, 500000].map((score) => (
            <button
              key={score}
              type="button"
              onClick={() => updateParam('mate_score', score)}
              className={`px-2 py-0.5 rounded text-[11px] font-mono transition-colors cursor-pointer border ${
                currentConfig.mate_score === score
                  ? 'bg-amber-500 text-stone-950 font-bold border-amber-400'
                  : 'bg-stone-800 hover:bg-stone-700 text-stone-300 border-stone-700'
              }`}
            >
              {(score / 1000).toLocaleString('vi-VN')}k {score === 300000 && '(Mặc định)'}
            </button>
          ))}
        </div>
      </div>

      {/* 3. NNUE Weight */}
      <div className="bg-[#221D19] p-4 rounded-xl border border-stone-800/90 space-y-3">
        <div className="flex items-start justify-between gap-3">
          <div className="flex items-center gap-2">
            <BrainCircuit className="w-4 h-4 text-emerald-400 shrink-0" />
            <div>
              <div className="flex items-center gap-2">
                <span className="text-xs font-semibold text-stone-200">
                  Hệ Số Khuếch Đại Mạng Nơ-ron NNUE
                </span>
                <code className="text-[11px] font-mono text-amber-300 bg-stone-900 px-1.5 py-0.5 rounded border border-stone-700/60">
                  nnue_weight: i32
                </code>
              </div>
              <p className="text-[11px] text-stone-400 mt-0.5">
                Tỷ lệ điều chỉnh độ nhạy điểm số của mạng sâu Pure NNUE HalfKP v3 theo công thức{' '}
                <span className="font-mono text-stone-300">(raw * nnue_weight) / 100</span>.
              </p>
            </div>
          </div>
          <div className="text-right shrink-0">
            <span className="text-sm font-mono font-bold text-amber-400">
              {currentConfig.nnue_weight}%
            </span>
          </div>
        </div>

        <input
          type="range"
          min={50}
          max={150}
          step={5}
          value={currentConfig.nnue_weight}
          onChange={(e) => updateParam('nnue_weight', Number(e.target.value))}
          className="w-full accent-amber-500 cursor-pointer h-1.5 bg-stone-800 rounded-lg"
        />

        <div className="flex flex-wrap items-center gap-1.5 pt-1">
          <span className="text-[10px] text-stone-400 mr-1">Mức nhanh:</span>
          {[75, 90, 100, 110, 120, 125].map((w) => (
            <button
              key={w}
              type="button"
              onClick={() => updateParam('nnue_weight', w)}
              className={`px-2 py-0.5 rounded text-[11px] font-mono transition-colors cursor-pointer border ${
                currentConfig.nnue_weight === w
                  ? 'bg-amber-500 text-stone-950 font-bold border-amber-400'
                  : 'bg-stone-800 hover:bg-stone-700 text-stone-300 border-stone-700'
              }`}
            >
              {w}% {w === 100 && '(Chuẩn)'}
            </button>
          ))}
        </div>
      </div>

      {/* 4. Multi-threading Search Threads */}
      <div className="bg-[#221D19] p-4 rounded-xl border border-stone-800/90 space-y-3">
        <div className="flex items-start justify-between gap-3">
          <div className="flex items-center gap-2">
            <Cpu className="w-4 h-4 text-amber-400 shrink-0" />
            <div>
              <div className="flex items-center gap-2">
                <span className="text-xs font-semibold text-stone-200">
                  Số Luồng Tìm Kiếm AI (Multi-threading)
                </span>
                <code className="text-[11px] font-mono text-amber-300 bg-stone-900 px-1.5 py-0.5 rounded border border-stone-700/60">
                  threads: usize
                </code>
              </div>
              <p className="text-[11px] text-stone-400 mt-0.5">
                Số luồng tính toán song song qua Web Worker Pool. Mặc định là số luồng của máy chia 4 (
                {getDefaultThreads()} luồng trên thiết bị này / {hardwareCores} logical cores).
              </p>
            </div>
          </div>
          <div className="text-right shrink-0">
            <span className="text-sm font-mono font-bold text-amber-400">
              {activeThreads} Luồng
            </span>
          </div>
        </div>

        <input
          type="range"
          min={1}
          max={maxThreads}
          step={1}
          value={activeThreads}
          onChange={(e) => updateParam('threads', Number(e.target.value))}
          className="w-full accent-amber-500 cursor-pointer h-1.5 bg-stone-800 rounded-lg"
        />

        <div className="flex flex-wrap items-center justify-between gap-2 pt-1">
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="text-[10px] text-stone-400 mr-1">Mức nhanh:</span>
            {quickThreadOptions.map((t) => (
              <button
                key={t}
                type="button"
                onClick={() => updateParam('threads', t)}
                className={`px-2 py-0.5 rounded text-[11px] font-mono transition-colors cursor-pointer border ${
                  activeThreads === t
                    ? 'bg-amber-500 text-stone-950 font-bold border-amber-400'
                    : 'bg-stone-800 hover:bg-stone-700 text-stone-300 border-stone-700'
                }`}
              >
                {t} Luồng {t === getDefaultThreads() && '(Mặc định)'}
              </button>
            ))}
          </div>

          <div className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-stone-900 text-stone-300 border border-stone-800 flex items-center gap-1.5">
            <span
              className={`w-1.5 h-1.5 rounded-full ${
                typeof crossOriginIsolated !== 'undefined' && crossOriginIsolated
                  ? 'bg-emerald-400'
                  : 'bg-amber-400'
              }`}
            />
            <span>
              {typeof crossOriginIsolated !== 'undefined' && crossOriginIsolated
                ? 'WebAssembly Shared Memory (Lazy SMP)'
                : 'Worker Pool (Root Splitting)'}
            </span>
          </div>
        </div>
      </div>
    </section>
  );
};
