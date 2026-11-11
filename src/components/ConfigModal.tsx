import React, { useState, useEffect } from 'react';
import {
  SlidersHorizontal,
  Upload,
  Download,
  RotateCcw,
  X,
  CheckCircle2,
  AlertCircle,
  Link2,
} from 'lucide-react';
import type { EngineConfig } from '../types/game.js';
import { DEFAULT_ENGINE_CONFIG, getDefaultThreads } from '../types/game.js';
import { exportConfigJson } from '../utils/export.js';
import {
  formatRamMb,
  sanitizeImportedConfig,
  type Preset,
} from './config/configPresets.js';
import { ConfigPresetsSection } from './config/ConfigPresetsSection.js';
import { ConfigParamsSection } from './config/ConfigParamsSection.js';
import { ConfigArchitectureSection } from './config/ConfigArchitectureSection.js';

export interface ConfigModalProps {
  isOpen: boolean;
  onClose: () => void;
  redConfig: EngineConfig;
  setRedConfig: React.Dispatch<React.SetStateAction<EngineConfig>>;
  blackConfig: EngineConfig;
  setBlackConfig: React.Dispatch<React.SetStateAction<EngineConfig>>;
  onConfigChange?: (updatedConfig: EngineConfig) => void;
}

export const ConfigModal: React.FC<ConfigModalProps> = ({
  isOpen,
  onClose,
  redConfig,
  setRedConfig,
  blackConfig,
  setBlackConfig,
  onConfigChange,
}) => {
  const [activeTab, setActiveTab] = useState<'red' | 'black'>('red');
  const [syncBothSides, setSyncBothSides] = useState<boolean>(true);
  const [feedback, setFeedback] = useState<{
    type: 'success' | 'error';
    message: string;
  } | null>(null);

  // Local draft configs to prevent premature mutation before explicit user confirmation
  const [draftRedConfig, setDraftRedConfig] = useState<EngineConfig>({ ...redConfig });
  const [draftBlackConfig, setDraftBlackConfig] = useState<EngineConfig>({ ...blackConfig });

  // Reset local draft to current active props whenever modal is opened
  useEffect(() => {
    if (isOpen) {
      setDraftRedConfig({ ...redConfig });
      setDraftBlackConfig({ ...blackConfig });
      setFeedback(null);
    }
  }, [isOpen, redConfig, blackConfig]);

  if (!isOpen) return null;

  const currentConfig = activeTab === 'red' ? draftRedConfig : draftBlackConfig;
  const setDraftConfig = activeTab === 'red' ? setDraftRedConfig : setDraftBlackConfig;

  const updateParam = <K extends keyof EngineConfig>(
    key: K,
    value: EngineConfig[K]
  ) => {
    if (syncBothSides) {
      setDraftRedConfig((prev) => ({
        ...prev,
        [key]: value,
      }));
      setDraftBlackConfig((prev) => ({
        ...prev,
        [key]: value,
      }));
    } else {
      setDraftConfig((prev) => ({
        ...prev,
        [key]: value,
      }));
    }

    if (key === 'tt_size_mb' || key === 'threads') {
      const perWorker =
        key === 'tt_size_mb' ? Number(value) : currentConfig.tt_size_mb;
      const th = key === 'threads' ? Number(value) : currentConfig.threads;
      if (th === 1) {
        setFeedback({
          type: 'success',
          message: `Đã chọn ${perWorker} MB cho 1 luồng đơn (Tổng RAM ước tính ${perWorker} MB). Nhấn "Áp Dụng & Đóng" để kích hoạt!`,
        });
      } else {
        const sabMb = perWorker * th;
        const totalRam = sabMb * 2;
        const totalStr = formatRamMb(totalRam);
        setFeedback({
          type: 'success',
          message: `Đã chọn ${perWorker} MB/luồng × ${th} luồng. Tổng RAM ước tính: ${totalStr} (${sabMb} MB SharedArrayBuffer + ${sabMb} MB cho ${th} TT riêng). Nhấn "Áp Dụng & Đóng" để kích hoạt!`,
        });
      }
    }
  };

  const handleApplyPreset = (preset: Preset) => {
    if (syncBothSides) {
      setDraftRedConfig({ ...preset.config });
      setDraftBlackConfig({ ...preset.config });
      setFeedback({
        type: 'success',
        message: `Đã chọn Preset "${preset.name}" cho cả 2 bên. Nhấn "Áp Dụng & Đóng" để kích hoạt cấu hình!`,
      });
    } else {
      setDraftConfig({ ...preset.config });
      setFeedback({
        type: 'success',
        message: `Đã chọn Preset "${preset.name}" cho AI ${
          activeTab === 'red' ? 'Đỏ' : 'Đen'
        }. Nhấn "Áp Dụng & Đóng" để kích hoạt cấu hình!`,
      });
    }
  };

  const handleResetDefault = () => {
    if (syncBothSides) {
      setDraftRedConfig({ ...DEFAULT_ENGINE_CONFIG });
      setDraftBlackConfig({ ...DEFAULT_ENGINE_CONFIG });
      setFeedback({
        type: 'success',
        message:
          'Đã đưa về cấu hình mặc định cotuong_core (TT 512MB). Nhấn "Áp Dụng & Đóng" để xác nhận.',
      });
    } else {
      setDraftConfig({ ...DEFAULT_ENGINE_CONFIG });
      setFeedback({
        type: 'success',
        message: `Đã đưa về cấu hình mặc định cotuong_core cho AI ${
          activeTab === 'red' ? 'Đỏ' : 'Đen'
        }. Nhấn "Áp Dụng & Đóng" để xác nhận.`,
      });
    }
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      try {
        const rawParsed: unknown = JSON.parse(event.target?.result as string);
        const cleaned = sanitizeImportedConfig(rawParsed, currentConfig);

        if (syncBothSides) {
          setDraftRedConfig(cleaned);
          setDraftBlackConfig(cleaned);
        } else {
          setDraftConfig(cleaned);
        }
        setFeedback({
          type: 'success',
          message:
            'Nhập cấu hình JSON thành công! Nhấn "Áp Dụng & Đóng" để lưu và áp dụng.',
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

  // Commit draft changes to parent state and trigger engine memory reallocation
  const handleApplyAndClose = () => {
    setRedConfig(draftRedConfig);
    setBlackConfig(draftBlackConfig);
    const appliedConfig = activeTab === 'red' ? draftRedConfig : draftBlackConfig;
    onConfigChange?.(appliedConfig);
    onClose();
  };

  // Discard draft modifications and exit without saving
  const handleDiscardAndClose = () => {
    setDraftRedConfig({ ...redConfig });
    setDraftBlackConfig({ ...blackConfig });
    setFeedback(null);
    onClose();
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
      onClick={handleDiscardAndClose}
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
                Các thay đổi chỉ được lưu và tái cấp phát bộ nhớ khi nhấn "Áp Dụng & Đóng"
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={handleDiscardAndClose}
            aria-label="Đóng mà không lưu"
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
          <ConfigPresetsSection
            currentConfig={currentConfig}
            onApplyPreset={handleApplyPreset}
          />
          <ConfigParamsSection
            currentConfig={currentConfig}
            updateParam={updateParam}
          />
          <ConfigArchitectureSection pureCoreConfig={pureCoreConfig} />
        </div>

        {/* Modal Footer */}
        <div className="px-5 sm:px-6 py-3.5 border-t border-stone-800 bg-[#161210] flex flex-wrap items-center justify-between gap-3">
          <div className="text-[11px] text-stone-400 hidden sm:block">
            Cấu hình sẽ áp dụng cho:{' '}
            <strong className="text-amber-400">
              {syncBothSides
                ? 'Cả hai bên AI (Đỏ & Đen)'
                : activeTab === 'red'
                  ? 'AI Đỏ'
                  : 'AI Đen'}
            </strong>
          </div>
          <div className="flex items-center gap-2.5 w-full sm:w-auto justify-end">
            <button
              type="button"
              onClick={handleDiscardAndClose}
              className="px-4 py-2 rounded-xl text-xs font-semibold bg-stone-800 hover:bg-stone-700 text-stone-300 border border-stone-700 transition-colors cursor-pointer"
            >
              Hủy Bỏ
            </button>
            <button
              type="button"
              onClick={handleApplyAndClose}
              className="px-6 py-2 rounded-xl text-xs font-semibold bg-amber-500 hover:bg-amber-400 text-stone-950 transition-colors cursor-pointer shadow-md font-bold flex items-center justify-center gap-1.5"
            >
              <CheckCircle2 className="w-4 h-4" />
              <span>Áp Dụng & Đóng</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
