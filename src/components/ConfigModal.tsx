import React, { useState } from 'react';
import {
  SlidersHorizontal,
  Upload,
  Download,
  RotateCcw,
  X,
  CheckCircle2,
  AlertCircle,
  Cpu,
  Database,
  Flame,
  BrainCircuit,
  FileCode2,
  Layers,
  Link2,
} from 'lucide-react';
import type { EngineConfig } from '../types/game.js';
import { DEFAULT_ENGINE_CONFIG, getDefaultThreads } from '../types/game.js';
import { exportConfigJson } from '../utils/export.js';

interface ConfigModalProps {
  isOpen: boolean;
  onClose: () => void;
  redConfig: EngineConfig;
  setRedConfig: React.Dispatch<React.SetStateAction<EngineConfig>>;
  blackConfig: EngineConfig;
  setBlackConfig: React.Dispatch<React.SetStateAction<EngineConfig>>;
}

interface Preset {
  name: string;
  desc: string;
  config: EngineConfig;
}

const PRESETS: Preset[] = [
  {
    name: 'Tiêu Chuẩn (Mặc định)',
    desc: 'Cân bằng hoàn hảo theo cấu hình gốc của cotuong_core với TT 512MB tối ưu đa luồng.',
    config: {
      mate_score: 300000,
      tt_size_mb: 512,
      nnue_weight: 100,
      threads: getDefaultThreads(),
    },
  },
  {
    name: 'Tấn Công / Chuẩn Xác Cao',
    desc: 'Tăng điểm chiếu bí, khuếch đại thế công mạng và dùng độ sâu tính toán tối đa.',
    config: {
      mate_score: 400000,
      tt_size_mb: 512,
      nnue_weight: 120,
      threads: Math.min(16, Math.max(2, getDefaultThreads() * 2)),
    },
  },
  {
    name: 'Tiết Kiệm RAM & CPU',
    desc: 'Bảng băm TT 64MB, chạy 1 luồng đơn, tối ưu pin và tài nguyên tối đa cho thiết bị di động.',
    config: {
      mate_score: 200000,
      tt_size_mb: 64,
      nnue_weight: 90,
      threads: 1,
    },
  },
  {
    name: 'Sát Cục / Giải Cờ Thế',
    desc: 'Bảng băm cực đại 2GB (2048MB), điểm chiếu bí tối đa 500k và huy động tối đa số luồng phần cứng.',
    config: {
      mate_score: 500000,
      tt_size_mb: 2048,
      nnue_weight: 110,
      threads: Math.min(16, typeof navigator !== 'undefined' ? navigator.hardwareConcurrency || 4 : 4),
    },
  },
];

export const ConfigModal: React.FC<ConfigModalProps> = ({
  isOpen,
  onClose,
  redConfig,
  setRedConfig,
  blackConfig,
  setBlackConfig,
}) => {
  const [activeTab, setActiveTab] = useState<'red' | 'black'>('red');
  const [syncBothSides, setSyncBothSides] = useState<boolean>(true);
  const [feedback, setFeedback] = useState<{
    type: 'success' | 'error';
    message: string;
  } | null>(null);

  if (!isOpen) return null;

  const currentConfig = activeTab === 'red' ? redConfig : blackConfig;
  const setConfig = activeTab === 'red' ? setRedConfig : setBlackConfig;

  const updateParam = <K extends keyof EngineConfig>(
    key: K,
    value: EngineConfig[K],
  ) => {
    if (syncBothSides) {
      setRedConfig((prev) => ({
        ...prev,
        [key]: value,
      }));
      setBlackConfig((prev) => ({
        ...prev,
        [key]: value,
      }));
    } else {
      setConfig((prev) => ({
        ...prev,
        [key]: value,
      }));
    }
  };

  const handleApplyPreset = (preset: Preset) => {
    if (syncBothSides) {
      setRedConfig({ ...preset.config });
      setBlackConfig({ ...preset.config });
      setFeedback({
        type: 'success',
        message: `Đã áp dụng đồng bộ Preset "${preset.name}" cho cả 2 bên AI (Đỏ & Đen). Các tùy chỉnh bên dưới đã được cập nhật tương ứng!`,
      });
    } else {
      setConfig({ ...preset.config });
      setFeedback({
        type: 'success',
        message: `Đã áp dụng Preset "${preset.name}" cho AI ${
          activeTab === 'red' ? 'Đỏ' : 'Đen'
        }. Các tùy chỉnh bên dưới đã được cập nhật tương ứng!`,
      });
    }
  };

  const handleResetDefault = () => {
    if (syncBothSides) {
      setRedConfig({ ...DEFAULT_ENGINE_CONFIG });
      setBlackConfig({ ...DEFAULT_ENGINE_CONFIG });
      setFeedback({
        type: 'success',
        message: 'Đã khôi phục cấu hình mặc định cotuong_core cho cả AI Đỏ và AI Đen.',
      });
    } else {
      setConfig({ ...DEFAULT_ENGINE_CONFIG });
      setFeedback({
        type: 'success',
        message: `Đã khôi phục cấu hình mặc định cotuong_core cho AI ${
          activeTab === 'red' ? 'Đỏ' : 'Đen'
        }.`,
      });
    }
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      try {
        const rawParsed = JSON.parse(event.target?.result as string);
        if (typeof rawParsed !== 'object' || rawParsed === null) {
          throw new Error('Dữ liệu JSON phải là một đối tượng cấu hình hợp lệ');
        }

        // Validate and extract only engine config fields recognized by cotuong_core
        const cleaned: EngineConfig = {
          mate_score:
            typeof rawParsed.mate_score === 'number'
              ? Math.max(10000, Math.min(1000000, Math.round(rawParsed.mate_score)))
              : currentConfig.mate_score,
          tt_size_mb:
            typeof rawParsed.tt_size_mb === 'number'
              ? Math.max(16, Math.min(2048, Math.round(rawParsed.tt_size_mb)))
              : currentConfig.tt_size_mb,
          nnue_weight:
            typeof rawParsed.nnue_weight === 'number'
              ? Math.max(50, Math.min(150, Math.round(rawParsed.nnue_weight)))
              : currentConfig.nnue_weight,
          threads:
            typeof rawParsed.threads === 'number'
              ? Math.max(1, Math.min(16, Math.round(rawParsed.threads)))
              : (currentConfig.threads || getDefaultThreads()),
        };

        setConfig(cleaned);
        setFeedback({
          type: 'success',
          message: 'Nhập cấu hình JSON thành công vào cotuong_core!',
        });
      } catch (err) {
        setFeedback({
          type: 'error',
          message: 'Lỗi định dạng JSON: ' + String(err),
        });
      }
    };
    reader.readAsText(file);
    e.target.value = '';
  };

  // Construct pure cotuong_core EngineConfig payload for JSON export and preview
  const pureCoreConfig = {
    mate_score: currentConfig.mate_score,
    tt_size_mb: currentConfig.tt_size_mb,
    nnue_weight: currentConfig.nnue_weight,
    threads: currentConfig.threads || getDefaultThreads(),
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-xs p-3 sm:p-4"
      onClick={onClose}
    >
      <div
        className="w-full max-w-2xl max-h-[90vh] bg-[#1A1613] border border-stone-700/80 rounded-2xl shadow-2xl flex flex-col overflow-hidden animate-in fade-in zoom-in-95 duration-150"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Modal Header */}
        <div className="flex items-center justify-between px-5 sm:px-6 py-4 border-b border-stone-800 bg-[#161210]">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-400">
              <SlidersHorizontal className="w-5 h-5" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-base sm:text-lg font-bold text-[#F5EFE6]">
                  Tùy Chỉnh Cấu Hình cotuong_core
                </h2>
                <span className="text-[11px] font-mono px-2 py-0.5 rounded-full bg-amber-950/80 text-amber-300 border border-amber-500/30">
                  EngineConfig
                </span>
              </div>
              <p className="text-xs text-stone-400 mt-0.5">
                Các tham số được đồng bộ trực tiếp tới Rust WebAssembly Engine
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Đóng"
            className="p-1.5 rounded-lg text-stone-400 hover:text-white hover:bg-stone-800 transition-colors cursor-pointer"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Side Selector Tabs */}
        <div className="grid grid-cols-2 bg-[#120F0D] p-1.5 gap-1.5 border-b border-stone-800">
          <button
            type="button"
            onClick={() => {
              setActiveTab('red');
              setFeedback(null);
            }}
            className={`py-2 px-4 rounded-xl text-xs sm:text-sm font-semibold transition-all cursor-pointer flex items-center justify-center gap-2 ${
              activeTab === 'red'
                ? 'bg-red-950/90 text-red-200 border border-red-500/50 shadow-sm'
                : 'text-stone-400 hover:text-stone-200'
            }`}
          >
            <span className="font-serif font-bold text-sm text-red-400">帥</span>
            <span>Cấu hình AI Đỏ (Red)</span>
          </button>

          <button
            type="button"
            onClick={() => {
              setActiveTab('black');
              setFeedback(null);
            }}
            className={`py-2 px-4 rounded-xl text-xs sm:text-sm font-semibold transition-all cursor-pointer flex items-center justify-center gap-2 ${
              activeTab === 'black'
                ? 'bg-stone-800 text-amber-200 border border-amber-500/50 shadow-sm'
                : 'text-stone-400 hover:text-stone-200'
            }`}
          >
            <span className="font-serif font-bold text-sm text-amber-400">將</span>
            <span>Cấu hình AI Đen (Black)</span>
          </button>
        </div>

        {/* Actions Bar */}
        <div className="flex flex-wrap items-center justify-between gap-2 px-5 sm:px-6 py-2.5 bg-[#201B17] border-b border-stone-800">
          <div className="flex flex-wrap items-center gap-2">
            <label className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-stone-800 hover:bg-stone-700 text-stone-200 border border-stone-700 transition-colors cursor-pointer">
              <Upload className="w-3.5 h-3.5 text-amber-400" />
              <span>Nhập JSON</span>
              <input
                type="file"
                accept=".json"
                className="hidden"
                onChange={handleFileUpload}
              />
            </label>

            <button
              type="button"
              onClick={() =>
                exportConfigJson(pureCoreConfig, `${activeTab}_engine_config.json`)
              }
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-stone-800 hover:bg-stone-700 text-stone-200 border border-stone-700 transition-colors cursor-pointer"
            >
              <Download className="w-3.5 h-3.5 text-amber-400" />
              <span>Xuất JSON</span>
            </button>

            <button
              type="button"
              onClick={() => setSyncBothSides(!syncBothSides)}
              className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors cursor-pointer ${
                syncBothSides
                  ? 'bg-amber-950/70 border-amber-500/60 text-amber-300'
                  : 'bg-stone-800 hover:bg-stone-700 text-stone-400 border-stone-700'
              }`}
              title="Khi bật, việc chọn Preset hoặc thay đổi tham số sẽ tự động áp dụng cho cả AI Đỏ và AI Đen"
            >
              <Link2 className="w-3.5 h-3.5" />
              <span>Đồng bộ 2 bên ({syncBothSides ? 'Bật' : 'Tắt'})</span>
            </button>
          </div>

          <button
            type="button"
            onClick={handleResetDefault}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-stone-800/80 hover:bg-stone-700 text-stone-300 border border-stone-700/80 transition-colors cursor-pointer"
          >
            <RotateCcw className="w-3.5 h-3.5 text-stone-400" />
            <span>Mặc định (cotuong_core)</span>
          </button>
        </div>

        {/* Inline Feedback Banner */}
        {feedback && (
          <div
            className={`mx-5 sm:mx-6 mt-3 px-3.5 py-2.5 rounded-xl text-xs font-medium flex items-center gap-2 ${
              feedback.type === 'success'
                ? 'bg-emerald-950/70 text-emerald-300 border border-emerald-500/30'
                : 'bg-red-950/70 text-red-300 border border-red-500/30'
            }`}
          >
            {feedback.type === 'success' ? (
              <CheckCircle2 className="w-4 h-4 shrink-0" />
            ) : (
              <AlertCircle className="w-4 h-4 shrink-0" />
            )}
            <span>{feedback.message}</span>
          </div>
        )}

        {/* Modal Body */}
        <div className="flex-1 overflow-y-auto px-5 sm:px-6 py-4 space-y-6">
          {/* Quick Presets */}
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

                return (
                  <button
                    key={preset.name}
                    type="button"
                    onClick={() => handleApplyPreset(preset)}
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
                      <span>TT: {preset.config.tt_size_mb}MB</span>
                      <span>•</span>
                      <span>Mate: {(preset.config.mate_score / 1000).toLocaleString('vi-VN')}k</span>
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

          {/* Core Engine Parameters */}
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
                        Kích thước Transposition Table
                      </span>
                      <code className="text-[11px] font-mono text-amber-300 bg-stone-900 px-1.5 py-0.5 rounded border border-stone-700/60">
                        tt_size_mb: usize
                      </code>
                    </div>
                    <p className="text-[11px] text-stone-400 mt-0.5">
                      Dung lượng bảng băm Zobrist lưu trữ thế cờ đã duyệt và các nhánh cắt tỉa (Cutoff), tăng tốc độ duyệt nút (NPS).
                    </p>
                  </div>
                </div>
                <div className="text-right shrink-0">
                  <span className="text-sm font-mono font-bold text-amber-400">
                    {currentConfig.tt_size_mb >= 1024
                      ? `${(currentConfig.tt_size_mb / 1024).toFixed(currentConfig.tt_size_mb % 1024 === 0 ? 0 : 1)} GB (${currentConfig.tt_size_mb} MB)`
                      : `${currentConfig.tt_size_mb} MB`}
                  </span>
                </div>
              </div>

              <input
                type="range"
                min={16}
                max={2048}
                step={32}
                value={currentConfig.tt_size_mb}
                onChange={(e) => updateParam('tt_size_mb', Number(e.target.value))}
                className="w-full accent-amber-500 cursor-pointer h-1.5 bg-stone-800 rounded-lg"
              />

              <div className="flex flex-wrap items-center gap-1.5 pt-1">
                <span className="text-[10px] text-stone-400 mr-1">Mức nhanh:</span>
                {[64, 128, 256, 512, 1024, 2048].map((size) => (
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
                    {size >= 1024 ? `${size / 1024} GB` : `${size} MB`}{' '}
                    {size === 512 && '(Mặc định)'}
                  </button>
                ))}
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
                      Điểm cơ sở khi phát hiện chiếu bí. Điểm thực tế tính theo công thức <span className="font-mono text-stone-300">mate_score - ply</span> (càng sát nước thắng điểm càng cao).
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
                      Tỷ lệ điều chỉnh độ nhạy điểm số của mạng sâu Pure NNUE HalfKP v3 theo công thức <span className="font-mono text-stone-300">(raw * nnue_weight) / 100</span>.
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
                      Số luồng tính toán song song qua Web Worker Pool. Mặc định là số luồng của máy chia 4 ({getDefaultThreads()} luồng trên thiết bị này / {typeof navigator !== 'undefined' ? navigator.hardwareConcurrency || 4 : 4} logical cores).
                    </p>
                  </div>
                </div>
                <div className="text-right shrink-0">
                  <span className="text-sm font-mono font-bold text-amber-400">
                    {currentConfig.threads || getDefaultThreads()} Luồng
                  </span>
                </div>
              </div>

              <input
                type="range"
                min={1}
                max={Math.max(4, Math.min(16, typeof navigator !== 'undefined' ? navigator.hardwareConcurrency || 8 : 8))}
                step={1}
                value={currentConfig.threads || getDefaultThreads()}
                onChange={(e) => updateParam('threads', Number(e.target.value))}
                className="w-full accent-amber-500 cursor-pointer h-1.5 bg-stone-800 rounded-lg"
              />

              <div className="flex flex-wrap items-center justify-between gap-2 pt-1">
                <div className="flex flex-wrap items-center gap-1.5">
                  <span className="text-[10px] text-stone-400 mr-1">Mức nhanh:</span>
                  {Array.from(new Set([1, getDefaultThreads(), 2, 4, 8].filter((t) => t <= (typeof navigator !== 'undefined' ? navigator.hardwareConcurrency || 8 : 8))))
                    .sort((a, b) => a - b)
                    .map((t) => (
                      <button
                        key={t}
                        type="button"
                        onClick={() => updateParam('threads', t)}
                        className={`px-2 py-0.5 rounded text-[11px] font-mono transition-colors cursor-pointer border ${
                          (currentConfig.threads || getDefaultThreads()) === t
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

            {/* 5. Standard NNUE Model Architecture */}
            <div className="bg-[#221D19] p-4 rounded-xl border border-stone-800/90 space-y-3">
              <div className="flex items-start justify-between gap-3">
                <div className="flex items-center gap-2">
                  <Layers className="w-4 h-4 text-amber-400 shrink-0" />
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-semibold text-stone-200">
                        Kiến Trúc Mạng Nơ-ron NNUE Chuẩn (Stockfish / Pikafish)
                      </span>
                      <code className="text-[11px] font-mono text-amber-300 bg-stone-900 px-1.5 py-0.5 rounded border border-stone-700/60">
                        Dual-Head HalfKP v3
                      </code>
                    </div>
                    <p className="text-[11px] text-stone-400 mt-0.5">
                      Mô hình lượng tử hóa số nguyên duy nhất, tối ưu SIMD và nhúng tĩnh trong WebAssembly Core Engine.
                    </p>
                  </div>
                </div>
              </div>

              <div className="p-3 rounded-xl border border-amber-500/40 bg-amber-950/20 space-y-2.5">
                <div className="flex items-center justify-between text-xs">
                  <div className="flex items-center gap-1.5 text-stone-200 font-semibold">
                    <Cpu className="w-3.5 h-3.5 text-amber-400" />
                    <span>Mô Hình Chuẩn (SIMD i16 / i8)</span>
                  </div>
                  <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-amber-500/20 text-amber-300 border border-amber-500/40 font-bold">
                    default.nnue • ~4.0 MB
                  </span>
                </div>

                <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 text-[11px] text-stone-300">
                  <div className="bg-[#1A1613] p-2.5 rounded-lg border border-stone-800/80">
                    <span className="text-amber-400 font-semibold block text-[10px] uppercase tracking-wider">
                      Feature Transformer (Accumulator)
                    </span>
                    <p className="text-stone-400 mt-1 leading-relaxed">
                      10,530 HalfKP features × 192 chiều accumulator (trọng số 16-bit), kích hoạt SCReLU thuần số nguyên.
                    </p>
                  </div>
                  <div className="bg-[#1A1613] p-2.5 rounded-lg border border-stone-800/80">
                    <span className="text-amber-400 font-semibold block text-[10px] uppercase tracking-wider">
                      Dual-Head Evaluation (Đầu ra kép)
                    </span>
                    <p className="text-stone-400 mt-1 leading-relaxed">
                      Đánh giá thế trận Positional Deep FC kết hợp 8-Bucket Phase PSQT động bám sát theo giai đoạn tàn cuộc.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* JSON Payload Inspection */}
          <section className="bg-[#14100E] p-3 rounded-xl border border-stone-800">
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-1.5 text-stone-400 text-xs">
                <FileCode2 className="w-3.5 h-3.5 text-amber-400" />
                <span className="font-semibold text-stone-300">
                  Dữ liệu gửi sang Rust Engine (JSON Payload)
                </span>
              </div>
              <span className="text-[10px] font-mono text-emerald-400 bg-emerald-950/60 border border-emerald-500/30 px-2 py-0.5 rounded">
                Khớp 100% cotuong_core::EngineConfig
              </span>
            </div>
            <pre className="font-mono text-[11px] bg-[#0C0A09] text-amber-300/90 p-2.5 rounded-lg border border-stone-800 overflow-x-auto">
              {JSON.stringify(pureCoreConfig, null, 2)}
            </pre>
          </section>
        </div>

        {/* Modal Footer */}
        <div className="px-5 sm:px-6 py-3.5 border-t border-stone-800 bg-[#161210] flex items-center justify-between">
          <div className="text-[11px] text-stone-400 hidden sm:block">
            Cấu hình đang tác động tới:{' '}
            <strong className="text-amber-400">
              {syncBothSides ? 'Cả hai bên AI (Đỏ & Đen)' : activeTab === 'red' ? 'AI Đỏ' : 'AI Đen'}
            </strong>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="w-full sm:w-auto px-6 py-2 rounded-xl text-xs font-semibold bg-amber-500 hover:bg-amber-400 text-stone-950 transition-colors cursor-pointer shadow-md"
          >
            Áp Dụng & Đóng
          </button>
        </div>
      </div>
    </div>
  );
};
