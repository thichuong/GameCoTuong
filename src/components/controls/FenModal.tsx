import React, { useState, useEffect } from 'react';
import { FileCode, Copy, Check } from 'lucide-react';

export interface FenModalProps {
  isOpen: boolean;
  initialFen: string;
  currentFen: string;
  description?: string;
  onClose: () => void;
  onApplyFen: (fen: string) => void;
}

export const FenModal: React.FC<FenModalProps> = ({
  isOpen,
  initialFen,
  currentFen,
  description = 'Dán chuỗi FEN để nạp nhanh thế cờ hoặc sao chép chuỗi FEN của bàn cờ hiện tại:',
  onClose,
  onApplyFen,
}) => {
  const [fenInput, setFenInput] = useState(initialFen);
  const [fenError, setFenError] = useState<string | null>(null);
  const [copiedFen, setCopiedFen] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setFenInput(initialFen);
      setFenError(null);
      setCopiedFen(false);
    }
  }, [isOpen, initialFen]);

  if (!isOpen) return null;

  const handleCopyFen = () => {
    navigator.clipboard.writeText(currentFen).then(() => {
      setCopiedFen(true);
      setTimeout(() => setCopiedFen(false), 2000);
    });
  };

  const handleConfirm = () => {
    try {
      setFenError(null);
      onApplyFen(fenInput);
      onClose();
    } catch (err: unknown) {
      setFenError(err instanceof Error ? err.message : 'FEN không hợp lệ');
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-xs"
      onClick={onClose}
    >
      <div
        className="w-full max-w-lg rounded-2xl bg-[#1C1815] border border-amber-500/40 shadow-2xl p-5 flex flex-col gap-3"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between pb-2 border-b border-stone-800">
          <h3 className="font-display-serif text-base font-bold text-amber-200 flex items-center gap-2">
            <FileCode className="w-4 h-4 text-amber-400" />
            <span>Nhập / Xuất FEN Cờ Thế</span>
          </h3>
          <button
            type="button"
            onClick={onClose}
            aria-label="Đóng"
            className="text-stone-400 hover:text-stone-200 text-sm font-bold px-2 py-0.5 rounded cursor-pointer"
          >
            ✕
          </button>
        </div>

        <p className="text-xs text-stone-400">{description}</p>

        <textarea
          value={fenInput}
          onChange={(e) => {
            setFenInput(e.target.value);
            setFenError(null);
          }}
          rows={3}
          placeholder="Ví dụ: 3ak4/4a4/9/9/9/9/9/4N4/4p4/4K4 w"
          className="w-full bg-[#120F0D] text-amber-100 font-mono text-xs rounded-xl p-3 border border-stone-700 focus:border-amber-400 focus:outline-none resize-none"
        />

        {fenError && <p className="text-xs text-red-400">{fenError}</p>}

        <div className="flex items-center justify-between gap-2 pt-2">
          <button
            type="button"
            onClick={handleCopyFen}
            className="inline-flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-semibold bg-[#26201C] hover:bg-stone-800 text-stone-200 border border-stone-700 transition-colors cursor-pointer"
          >
            {copiedFen ? (
              <>
                <Check className="w-3.5 h-3.5 text-emerald-400" />
                <span>Đã sao chép!</span>
              </>
            ) : (
              <>
                <Copy className="w-3.5 h-3.5 text-amber-400" />
                <span>Sao chép FEN hiện tại</span>
              </>
            )}
          </button>

          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={onClose}
              className="px-3 py-2 rounded-lg text-xs font-medium text-stone-400 hover:text-stone-200 cursor-pointer"
            >
              Đóng
            </button>
            <button
              type="button"
              onClick={handleConfirm}
              className="px-4 py-2 rounded-lg text-xs font-semibold bg-amber-600 hover:bg-amber-500 text-white transition-colors cursor-pointer shadow-md"
            >
              Áp dụng FEN
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
