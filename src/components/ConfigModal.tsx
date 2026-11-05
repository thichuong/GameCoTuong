import React, { useState } from 'react';
import {
  SlidersHorizontal,
  Upload,
  Download,
  RotateCcw,
  X,
  CheckCircle2,
  AlertCircle,
} from 'lucide-react';
import type { EngineConfig } from '../types/game.js';
import { DEFAULT_ENGINE_CONFIG } from '../types/game.js';
import { exportConfigJson } from '../utils/export.js';

interface ConfigModalProps {
  isOpen: boolean;
  onClose: () => void;
  redConfig: EngineConfig;
  setRedConfig: React.Dispatch<React.SetStateAction<EngineConfig>>;
  blackConfig: EngineConfig;
  setBlackConfig: React.Dispatch<React.SetStateAction<EngineConfig>>;
}

export const ConfigModal: React.FC<ConfigModalProps> = ({
  isOpen,
  onClose,
  redConfig,
  setRedConfig,
  blackConfig,
  setBlackConfig,
}) => {
  const [activeTab, setActiveTab] = useState<'red' | 'black'>('red');
  const [feedback, setFeedback] = useState<{
    type: 'success' | 'error';
    message: string;
  } | null>(null);

  if (!isOpen) return null;

  const currentConfig = activeTab === 'red' ? redConfig : blackConfig;
  const setConfig = activeTab === 'red' ? setRedConfig : setBlackConfig;

  const handleSliderChange = (key: keyof EngineConfig, value: number) => {
    setConfig((prev) => ({
      ...prev,
      [key]: value,
    }));
  };

  const handleResetDefault = () => {
    setConfig({ ...DEFAULT_ENGINE_CONFIG });
    setFeedback({
      type: 'success',
      message: `Đã khôi phục mặc định cho AI ${activeTab === 'red' ? 'Đỏ' : 'Đen'}.`,
    });
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      try {
        const parsed = JSON.parse(event.target?.result as string);
        setConfig((prev) => ({
          ...prev,
          ...parsed,
        }));
        setFeedback({
          type: 'success',
          message: 'Nhập cấu hình JSON thành công!',
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

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-xs p-4"
      onClick={onClose}
    >
      <div
        className="w-full max-w-2xl max-h-[88vh] bg-[#1A1613] border border-stone-700/80 rounded-2xl shadow-2xl flex flex-col overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Modal Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-stone-800">
          <div className="flex items-center gap-2.5">
            <SlidersHorizontal className="w-5 h-5 text-amber-400" />
            <h2 className="text-base sm:text-lg font-bold text-[#F5EFE6]">
              Tùy Chỉnh Tham Số Engine AI
            </h2>
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
        <div className="grid grid-cols-2 bg-[#14110F] p-1.5 gap-1.5 border-b border-stone-800">
          <button
            type="button"
            onClick={() => {
              setActiveTab('red');
              setFeedback(null);
            }}
            className={`py-2 px-4 rounded-xl text-xs sm:text-sm font-semibold transition-all cursor-pointer flex items-center justify-center gap-2 ${
              activeTab === 'red'
                ? 'bg-red-950/80 text-red-200 border border-red-500/40 shadow-sm'
                : 'text-stone-400 hover:text-stone-200'
            }`}
          >
            <span className="font-display-serif text-sm">帥</span>
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
                ? 'bg-stone-800 text-amber-200 border border-amber-500/40 shadow-sm'
                : 'text-stone-400 hover:text-stone-200'
            }`}
          >
            <span className="font-display-serif text-sm">將</span>
            <span>Cấu hình AI Đen (Black)</span>
          </button>
        </div>

        {/* Actions Bar */}
        <div className="flex flex-wrap items-center justify-between gap-2 px-6 py-3 bg-[#211C18] border-b border-stone-800">
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
                exportConfigJson(currentConfig, `${activeTab}_engine_config.json`)
              }
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-stone-800 hover:bg-stone-700 text-stone-200 border border-stone-700 transition-colors cursor-pointer"
            >
              <Download className="w-3.5 h-3.5 text-amber-400" />
              <span>Xuất JSON</span>
            </button>
          </div>

          <button
            type="button"
            onClick={handleResetDefault}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-stone-800/70 hover:bg-stone-700 text-stone-300 border border-stone-700/80 transition-colors cursor-pointer"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            <span>Mặc định</span>
          </button>
        </div>

        {/* Inline Feedback Banner */}
        {feedback && (
          <div
            className={`mx-6 mt-3 px-3.5 py-2 rounded-lg text-xs font-medium flex items-center gap-2 ${
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

        {/* Sliders Body */}
        <div className="flex-1 overflow-y-auto px-6 py-4 space-y-6">
          <section>
            <h3 className="text-xs font-semibold tracking-wider text-amber-400 pb-2 mb-3 border-b border-stone-800">
              01. Giá trị quân cờ cơ bản (Piece Values)
            </h3>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <Slider
                label="Tốt (Pawn)"
                value={currentConfig.val_pawn}
                min={0}
                max={200}
                step={1}
                onChange={(v) => handleSliderChange('val_pawn', v)}
              />
              <Slider
                label="Sĩ (Advisor)"
                value={currentConfig.val_advisor}
                min={0}
                max={400}
                step={1}
                onChange={(v) => handleSliderChange('val_advisor', v)}
              />
              <Slider
                label="Tượng (Elephant)"
                value={currentConfig.val_elephant}
                min={0}
                max={400}
                step={1}
                onChange={(v) => handleSliderChange('val_elephant', v)}
              />
              <Slider
                label="Mã (Horse)"
                value={currentConfig.val_horse}
                min={0}
                max={800}
                step={1}
                onChange={(v) => handleSliderChange('val_horse', v)}
              />
              <Slider
                label="Pháo (Cannon)"
                value={currentConfig.val_cannon}
                min={0}
                max={900}
                step={1}
                onChange={(v) => handleSliderChange('val_cannon', v)}
              />
              <Slider
                label="Xe (Rook)"
                value={currentConfig.val_rook}
                min={0}
                max={1800}
                step={1}
                onChange={(v) => handleSliderChange('val_rook', v)}
              />
              <Slider
                label="Tướng (King)"
                value={currentConfig.val_king}
                min={5000}
                max={20000}
                step={100}
                onChange={(v) => handleSliderChange('val_king', v)}
              />
            </div>
          </section>

          <section>
            <h3 className="text-xs font-semibold tracking-wider text-amber-400 pb-2 mb-3 border-b border-stone-800">
              02. Trọng số Tìm kiếm & Heuristics
            </h3>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <Slider
                label="Hash Move Score"
                value={currentConfig.score_hash_move}
                min={0}
                max={5000000}
                step={100000}
                onChange={(v) => handleSliderChange('score_hash_move', v)}
              />
              <Slider
                label="Capture Base Score"
                value={currentConfig.score_capture_base}
                min={0}
                max={2000000}
                step={100000}
                onChange={(v) => handleSliderChange('score_capture_base', v)}
              />
              <Slider
                label="Killer Move Score"
                value={currentConfig.score_killer_move}
                min={0}
                max={2000000}
                step={100000}
                onChange={(v) => handleSliderChange('score_killer_move', v)}
              />
              <Slider
                label="History Max Score"
                value={currentConfig.score_history_max}
                min={0}
                max={2000000}
                step={100000}
                onChange={(v) => handleSliderChange('score_history_max', v)}
              />
            </div>
          </section>

          <section>
            <h3 className="text-xs font-semibold tracking-wider text-amber-400 pb-2 mb-3 border-b border-stone-800">
              03. Cắt tỉa & Điểm chiếu bí (Pruning & Mate)
            </h3>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div className="flex flex-col gap-1.5">
                <label
                  className="text-xs font-medium text-stone-300"
                  htmlFor="pruning-method-select"
                >
                  Phương pháp cắt tỉa
                </label>
                <select
                  id="pruning-method-select"
                  value={currentConfig.pruning_method}
                  onChange={(e) =>
                    handleSliderChange('pruning_method', Number(e.target.value))
                  }
                  className="bg-[#26201C] text-[#F5EFE6] text-xs font-medium rounded-lg px-3 py-2 border border-stone-700 focus:border-amber-400 focus:outline-none"
                >
                  <option value={0}>Dynamic Limiting</option>
                  <option value={1}>Late Move Reductions (LMR)</option>
                  <option value={2}>Kết hợp cả hai (Aggressive)</option>
                </select>
              </div>

              <Slider
                label="Pruning Multiplier"
                value={currentConfig.pruning_multiplier}
                min={0.1}
                max={2.0}
                step={0.1}
                onChange={(v) => handleSliderChange('pruning_multiplier', v)}
              />

              <Slider
                label="Mate Score"
                value={currentConfig.mate_score}
                min={10000}
                max={50000}
                step={1000}
                onChange={(v) => handleSliderChange('mate_score', v)}
              />
            </div>
          </section>
        </div>

        {/* Footer */}
        <div className="px-6 py-3.5 border-t border-stone-800 bg-[#14110F] flex justify-end">
          <button
            type="button"
            onClick={onClose}
            className="px-5 py-2 rounded-lg text-xs font-semibold bg-amber-500 hover:bg-amber-400 text-stone-950 transition-colors cursor-pointer"
          >
            Hoàn tất
          </button>
        </div>
      </div>
    </div>
  );
};

interface SliderProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
}

const Slider: React.FC<SliderProps> = ({
  label,
  value,
  min,
  max,
  step,
  onChange,
}) => {
  return (
    <div className="flex flex-col gap-1.5 bg-[#221D19] p-3 rounded-xl border border-stone-800/80">
      <div className="flex items-center justify-between text-xs">
        <label className="text-stone-300 font-medium" htmlFor={`slider-${label}`}>
          {label}
        </label>
        <span className="font-mono-tabular font-semibold text-amber-400">
          {value.toLocaleString('vi-VN')}
        </span>
      </div>
      <input
        id={`slider-${label}`}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-full accent-amber-500 cursor-pointer h-1.5 bg-stone-800 rounded-lg"
      />
    </div>
  );
};
