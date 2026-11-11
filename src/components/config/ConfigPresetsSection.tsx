import React from 'react';
import { Cpu } from 'lucide-react';
import type { EngineConfig } from '../../types/game.js';
import { getDefaultThreads } from '../../types/game.js';
import { PRESETS, type Preset } from './configPresets.js';

interface ConfigPresetsSectionProps {
  currentConfig: EngineConfig;
  onApplyPreset: (preset: Preset) => void;
}

export const ConfigPresetsSection: React.FC<ConfigPresetsSectionProps> = ({
  currentConfig,
  onApplyPreset,
}) => {
  return (
    <section>
      <div className="flex items-center gap-2 pb-2 mb-3 border-b border-stone-800">
        <Cpu className="w-4 h-4 text-amber-400" />
        <h3 className="text-xs font-semibold tracking-wider uppercase text-amber-400">
          Cấu hình mẫu định sẵn (Presets)
        </h3>
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
        {PRESETS.map((preset) => {
          const isSelected =
            currentConfig.mate_score === preset.config.mate_score &&
            currentConfig.tt_size_mb === preset.config.tt_size_mb &&
            currentConfig.nnue_weight === preset.config.nnue_weight &&
            (currentConfig.threads || getDefaultThreads()) === preset.config.threads;

          const totalTtMb = preset.config.tt_size_mb * preset.config.threads;

          return (
            <button
              key={preset.name}
              type="button"
              onClick={() => onApplyPreset(preset)}
              className={`text-left p-2.5 rounded-xl border transition-all cursor-pointer ${
                isSelected
                  ? 'bg-amber-950/40 border-amber-500/60 shadow-sm'
                  : 'bg-[#221D19] border-stone-800 hover:border-stone-700 hover:bg-[#28221D]'
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="text-xs font-bold text-[#F5EFE6]">
                  {preset.name}
                </span>
                {isSelected && (
                  <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-amber-500 text-stone-950 font-bold">
                    Đang dùng
                  </span>
                )}
              </div>
              <p className="text-[11px] text-stone-400 mt-1 leading-relaxed">
                {preset.desc}
              </p>
              <div className="flex flex-wrap items-center gap-1.5 mt-2 text-[10px] font-mono text-stone-400">
                <span>
                  TT: {preset.config.tt_size_mb}MB/luồng (Tổng:{' '}
                  {totalTtMb >= 1024 ? `${totalTtMb / 1024}GB` : `${totalTtMb}MB`})
                </span>
                <span>•</span>
                <span>
                  Mate: {(preset.config.mate_score / 1000).toLocaleString('vi-VN')}k
                </span>
                <span>•</span>
                <span>NNUE: {preset.config.nnue_weight}%</span>
                <span>•</span>
                <span>Luồng: {preset.config.threads}</span>
              </div>
            </button>
          );
        })}
      </div>
    </section>
  );
};
