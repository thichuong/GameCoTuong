import type { EngineConfig } from '../../types/game.js';
import { getDefaultThreads } from '../../types/game.js';

export interface Preset {
  name: string;
  desc: string;
  config: EngineConfig;
}

interface NavigatorWithMemory extends Navigator {
  deviceMemory?: number;
}

const ALL_PRESETS: Preset[] = [
  {
    name: 'Tiết Kiệm (1 luồng)',
    desc: 'Thiết bị di động hoặc máy yếu: 1 luồng, 256MB RAM/luồng.',
    config: {
      mate_score: 200000,
      tt_size_mb: 256,
      nnue_weight: 90,
      threads: 1,
    },
  },
  {
    name: 'Tiêu Chuẩn (2 luồng)',
    desc: 'Cấu hình cân bằng: 2 luồng, 256MB RAM/luồng.',
    config: {
      mate_score: 300000,
      tt_size_mb: 256,
      nnue_weight: 100,
      threads: 2,
    },
  },
  {
    name: 'Cao (4 luồng)',
    desc: 'Sức mạnh trung bình: 4 luồng, 512MB RAM/luồng.',
    config: {
      mate_score: 400000,
      tt_size_mb: 512,
      nnue_weight: 110,
      threads: 4,
    },
  },
  {
    name: 'Cực cao (8 luồng)',
    desc: 'Sức mạnh tối đa: 8 luồng, 512MB RAM/luồng.',
    config: {
      mate_score: 500000,
      tt_size_mb: 512,
      nnue_weight: 120,
      threads: 8,
    },
  },
];

export const PRESETS: Preset[] = ALL_PRESETS.filter((p) => {
  const hardware =
    typeof navigator !== 'undefined' ? navigator.hardwareConcurrency || 8 : 8;
  const navWithMem =
    typeof navigator !== 'undefined' ? (navigator as NavigatorWithMemory) : undefined;
  const totalMemoryMb =
    navWithMem && typeof navWithMem.deviceMemory === 'number'
      ? navWithMem.deviceMemory * 1024
      : 8192;
  const isThreadCompatible = p.config.threads <= hardware;
  // Rule: total_tt * 2 < 80% RAM
  const totalTt = p.config.tt_size_mb * p.config.threads;
  const isMemoryCompatible = totalTt * 2 < totalMemoryMb * 0.8;
  return isThreadCompatible && isMemoryCompatible;
});

/**
 * Validates and clamps a raw parsed JSON object into a valid EngineConfig
 */
export function sanitizeImportedConfig(
  rawParsed: unknown,
  fallbackConfig: EngineConfig
): EngineConfig {
  if (typeof rawParsed !== 'object' || rawParsed === null || Array.isArray(rawParsed)) {
    throw new Error('Dữ liệu JSON phải là một đối tượng cấu hình hợp lệ');
  }
  const obj = rawParsed as Record<string, unknown>;
  return {
    mate_score:
      typeof obj.mate_score === 'number'
        ? Math.max(10000, Math.min(1000000, Math.round(obj.mate_score)))
        : fallbackConfig.mate_score,
    tt_size_mb:
      typeof obj.tt_size_mb === 'number'
        ? Math.max(16, Math.min(2048, Math.round(obj.tt_size_mb)))
        : fallbackConfig.tt_size_mb,
    nnue_weight:
      typeof obj.nnue_weight === 'number'
        ? Math.max(50, Math.min(150, Math.round(obj.nnue_weight)))
        : fallbackConfig.nnue_weight,
    threads:
      typeof obj.threads === 'number'
        ? Math.max(1, Math.min(16, Math.round(obj.threads)))
        : fallbackConfig.threads || getDefaultThreads(),
  };
}

export function formatRamMb(mb: number): string {
  if (mb >= 1024) {
    return `${(mb / 1024).toFixed(mb % 1024 === 0 ? 0 : 1)} GB`;
  }
  return `${mb} MB`;
}
