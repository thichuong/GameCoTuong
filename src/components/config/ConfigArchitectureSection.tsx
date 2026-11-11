import React from 'react';
import { Cpu, FileCode2, Layers } from 'lucide-react';

interface ConfigArchitectureSectionProps {
  pureCoreConfig: {
    mate_score: number;
    tt_size_mb: number;
    nnue_weight: number;
    threads: number;
  };
}

export const ConfigArchitectureSection: React.FC<ConfigArchitectureSectionProps> = ({
  pureCoreConfig,
}) => {
  return (
    <>
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
    </>
  );
};
