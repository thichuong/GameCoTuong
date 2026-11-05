import React, { useState } from 'react';
import type { EngineConfig } from '../types/game.js';
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

  if (!isOpen) return null;

  const currentConfig = activeTab === 'red' ? redConfig : blackConfig;
  const setConfig = activeTab === 'red' ? setRedConfig : setBlackConfig;

  const handleSliderChange = (key: keyof EngineConfig, value: number) => {
    setConfig((prev) => ({
      ...prev,
      [key]: value,
    }));
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
        alert('Tải cấu hình thành công!');
      } catch (err) {
        alert('Lỗi định dạng JSON: ' + String(err));
      }
    };
    reader.readAsText(file);
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>⚙ Tùy Chỉnh Cấu Hình Engine AI</h2>
          <button type="button" className="modal-close-btn" onClick={onClose}>
            ✕
          </button>
        </div>

        {/* Tab switch between Red and Black config */}
        <div className="modal-tabs">
          <button
            type="button"
            className={`tab-btn ${activeTab === 'red' ? 'tab-active red-tab' : ''}`}
            onClick={() => setActiveTab('red')}
          >
            🔴 Cấu hình Đỏ (Red)
          </button>
          <button
            type="button"
            className={`tab-btn ${activeTab === 'black' ? 'tab-active black-tab' : ''}`}
            onClick={() => setActiveTab('black')}
          >
            ⚫ Cấu hình Đen (Black)
          </button>
        </div>

        {/* Import/Export buttons */}
        <div className="modal-actions-bar">
          <label className="file-upload-btn">
            📂 Nhập file JSON
            <input
              type="file"
              accept=".json"
              style={{ display: 'none' }}
              onChange={handleFileUpload}
            />
          </label>
          <button
            type="button"
            className="action-btn"
            onClick={() =>
              exportConfigJson(currentConfig, `${activeTab}_engine_config.json`)
            }
          >
            💾 Xuất file JSON
          </button>
        </div>

        <div className="modal-body">
          <h3 className="section-title">Giá trị quân cờ cơ bản (Piece Values)</h3>
          <div className="sliders-grid">
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

          <h3 className="section-title">Trọng số Tìm kiếm & Heuristics</h3>
          <div className="sliders-grid">
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

          <h3 className="section-title">Cắt tỉa & Điểm số (Pruning & Mate)</h3>
          <div className="sliders-grid">
            <div className="slider-group">
              <label className="slider-label" htmlFor="pruning-method-select">Phương pháp cắt tỉa</label>
              <select
                id="pruning-method-select"
                className="control-select"
                value={currentConfig.pruning_method}
                onChange={(e) =>
                  handleSliderChange('pruning_method', Number(e.target.value))
                }
              >
                <option value={0}>Dynamic Limiting</option>
                <option value={1}>Late Move Reductions (LMR)</option>
                <option value={2}>Both (Aggressive)</option>
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
        </div>

        <div className="modal-footer">
          <button type="button" className="control-btn btn-primary" onClick={onClose}>
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

const Slider: React.FC<SliderProps> = ({ label, value, min, max, step, onChange }) => {
  return (
    <div className="slider-group">
      <div className="slider-header">
        <label className="slider-label" htmlFor={`slider-${label}`}>{label}</label>
        <span className="slider-value">{value}</span>
      </div>
      <input
        id={`slider-${label}`}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="slider-input"
      />
    </div>
  );
};

