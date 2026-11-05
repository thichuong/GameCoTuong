import React, { useRef, useEffect, useState } from 'react';
import {
  ScrollText,
  ChevronUp,
  ChevronDown,
  Download,
  Swords,
  Bot,
  X,
} from 'lucide-react';
import type { HistoryRecord, PieceColor, PieceType } from '../types/game.js';

interface LogPanelProps {
  history: HistoryRecord[];
  isOpen?: boolean;
  onToggle?: (open: boolean) => void;
  onExportCsv?: () => void;
}

const PIECE_NAMES_VI: Record<PieceType, string> = {
  General: 'Tướng',
  Advisor: 'Sĩ',
  Elephant: 'Tượng',
  Horse: 'Mã',
  Chariot: 'Xe',
  Cannon: 'Pháo',
  Soldier: 'Tốt',
};

function getPieceChar(type: PieceType, color: PieceColor): string {
  switch (type) {
    case 'General':
      return color === 'red' ? '帥' : '將';
    case 'Advisor':
      return color === 'red' ? '仕' : '士';
    case 'Elephant':
      return color === 'red' ? '相' : '象';
    case 'Horse':
      return color === 'red' ? '傌' : '馬';
    case 'Chariot':
      return color === 'red' ? '俥' : '車';
    case 'Cannon':
      return color === 'red' ? '炮' : '砲';
    case 'Soldier':
      return color === 'red' ? '兵' : '卒';
  }
}

export const LogPanel: React.FC<LogPanelProps> = ({
  history,
  isOpen: controlledIsOpen,
  onToggle,
  onExportCsv,
}) => {
  const [internalOpen, setInternalOpen] = useState(false);
  const isExpanded = controlledIsOpen !== undefined ? controlledIsOpen : internalOpen;

  const setExpanded = (next: boolean) => {
    if (onToggle) {
      onToggle(next);
    } else {
      setInternalOpen(next);
    }
  };

  const logEndRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (isExpanded) {
      logEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [history.length, isExpanded]);

  const lastRecord = history.length > 0 ? history[history.length - 1] : null;
  const lastMoveNumber = Math.ceil(history.length / 2);

  return (
    <>
      {/* Collapsed Dock Bar at the bottom ("ẩn xuống, khi nào cần thì mở lên xem") */}
      <div className="w-full max-w-[520px] mx-auto mt-3">
        <button
          type="button"
          onClick={() => setExpanded(!isExpanded)}
          className="w-full flex items-center justify-between gap-3 px-4 py-2.5 rounded-xl bg-[#1E1915] hover:bg-[#26201B] border border-stone-800 hover:border-amber-500/40 text-left transition-all duration-150 shadow-lg group cursor-pointer"
          aria-expanded={isExpanded}
        >
          <div className="flex items-center gap-3 min-w-0">
            <div className="w-8 h-8 rounded-lg bg-amber-500/10 border border-amber-500/25 flex items-center justify-center text-amber-400 shrink-0">
              <ScrollText className="w-4 h-4" />
            </div>
            <div className="min-w-0">
              <div className="flex items-center gap-2 text-xs font-semibold text-[#F5EFE6]">
                <span>Biên bản ván đấu</span>
                <span aria-hidden="true" className="text-stone-600">·</span>
                <span className="font-mono-tabular text-amber-400">
                  {history.length} nước
                </span>
              </div>
              <div className="text-xs text-stone-400 truncate mt-0.5">
                {lastRecord ? (
                  <span>
                    Nước cuối #{lastMoveNumber}{' '}
                    <span
                      className={
                        lastRecord.piece.color === 'red'
                          ? 'text-red-400 font-medium'
                          : 'text-sky-300 font-medium'
                      }
                    >
                      {lastRecord.piece.color === 'red' ? 'Đỏ' : 'Đen'}
                    </span>{' '}
                    · {PIECE_NAMES_VI[lastRecord.piece.type]}{' '}
                    <span className="font-mono-tabular">
                      ({lastRecord.from_row},{lastRecord.from_col}) → (
                      {lastRecord.to_row},{lastRecord.to_col})
                    </span>
                  </span>
                ) : (
                  <span>Chưa có nước đi nào — Nhấn để mở biên bản</span>
                )}
              </div>
            </div>
          </div>

          <div className="flex items-center gap-1.5 text-xs font-medium text-amber-300/90 group-hover:text-amber-200 whitespace-nowrap shrink-0">
            <span>{isExpanded ? 'Ẩn xuống' : 'Mở lên xem'}</span>
            {isExpanded ? (
              <ChevronDown className="w-4 h-4" />
            ) : (
              <ChevronUp className="w-4 h-4" />
            )}
          </div>
        </button>
      </div>

      {/* Expanded Slide-Up Sheet / Drawer */}
      {isExpanded && (
        <div
          className="fixed inset-0 z-50 flex flex-col justify-end bg-black/65 backdrop-blur-xs p-2 sm:p-4"
          onClick={() => setExpanded(false)}
        >
          <div
            className="w-full max-w-2xl mx-auto bg-[#1A1613] border border-stone-700/80 rounded-2xl shadow-[0_-16px_48px_rgba(0,0,0,0.85)] flex flex-col max-h-[75vh] animate-drawer-up overflow-hidden"
            onClick={(e) => e.stopPropagation()}
          >
            {/* Drawer Handle Bar */}
            <div className="flex justify-center pt-2.5 pb-1">
              <div className="w-10 h-1 rounded-full bg-stone-700" />
            </div>

            {/* Header */}
            <div className="flex items-center justify-between gap-4 px-5 py-3 border-b border-stone-800">
              <div className="flex items-center gap-2.5">
                <ScrollText className="w-4 h-4 text-amber-400" />
                <h2 className="text-base font-bold text-[#F5EFE6]">
                  Biên bản ván đấu
                </h2>
                <span aria-hidden="true" className="text-stone-600">·</span>
                <span className="text-xs font-mono-tabular text-stone-400">
                  {history.length} nước đã đi
                </span>
              </div>

              <div className="flex items-center gap-2">
                {onExportCsv && history.length > 0 && (
                  <button
                    type="button"
                    onClick={onExportCsv}
                    className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-stone-800 hover:bg-stone-700 text-stone-200 border border-stone-700 transition-colors cursor-pointer whitespace-nowrap shrink-0"
                  >
                    <Download className="w-3.5 h-3.5 text-amber-400" />
                    <span>Xuất CSV</span>
                  </button>
                )}

                <button
                  type="button"
                  onClick={() => setExpanded(false)}
                  className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-amber-500/15 hover:bg-amber-500/25 text-amber-300 border border-amber-500/30 transition-colors cursor-pointer whitespace-nowrap shrink-0"
                >
                  <ChevronDown className="w-3.5 h-3.5" />
                  <span>Ẩn xuống</span>
                </button>

                <button
                  type="button"
                  onClick={() => setExpanded(false)}
                  aria-label="Đóng biên bản"
                  className="p-1.5 rounded-lg text-stone-400 hover:text-stone-200 hover:bg-stone-800 transition-colors cursor-pointer"
                >
                  <X className="w-4 h-4" />
                </button>
              </div>
            </div>

            {/* Move List Body */}
            <div className="flex-1 overflow-y-auto p-4">
              {history.length === 0 ? (
                <div className="py-12 text-center text-stone-500 text-sm">
                  Chưa có nước đi nào trong ván đấu này. Hãy thực hiện nước cờ đầu tiên trên bàn cờ.
                </div>
              ) : (
                <ul className="space-y-2">
                  {history.map((record, i) => {
                    const turnNum = Math.floor(i / 2) + 1;
                    const isRed = record.piece.color === 'red';
                    const pieceVi = PIECE_NAMES_VI[record.piece.type];
                    const pieceChar = getPieceChar(record.piece.type, record.piece.color);

                    return (
                      <li
                        key={`${i}-${record.from_row}-${record.from_col}-${record.to_row}-${record.to_col}`}
                        className="flex flex-col gap-1.5 px-3.5 py-2.5 rounded-xl bg-[#221D19] border border-stone-800/80 hover:border-stone-700 transition-colors"
                      >
                        <div className="flex items-center justify-between gap-2">
                          <div className="flex items-center gap-2.5 min-w-0">
                            <span className="font-mono-tabular text-xs text-stone-500 w-7 shrink-0">
                              #{i + 1}
                            </span>

                            <span
                              className={`w-6 h-6 rounded-full flex items-center justify-center font-display-serif text-xs font-bold shrink-0 ${
                                isRed
                                  ? 'bg-red-950/90 text-red-300 border border-red-500/40'
                                  : 'bg-stone-800 text-stone-200 border border-stone-600'
                              }`}
                            >
                              {pieceChar}
                            </span>

                            <div className="text-xs sm:text-sm font-medium text-[#F5EFE6] truncate">
                              <span
                                className={
                                  isRed ? 'text-red-400 font-semibold' : 'text-sky-300 font-semibold'
                                }
                              >
                                {isRed ? 'Đỏ' : 'Đen'}
                              </span>
                              <span aria-hidden="true" className="mx-1.5 text-stone-600">·</span>
                              <span>{pieceVi}</span>
                              <span className="ml-2 font-mono-tabular text-stone-300">
                                ({record.from_row},{record.from_col}) → ({record.to_row},{record.to_col})
                              </span>
                            </div>
                          </div>

                          <span className="text-xs font-mono-tabular text-stone-500 shrink-0">
                            Hiệp {turnNum}
                          </span>
                        </div>

                        {(record.captured || record.note) && (
                          <div className="flex flex-wrap items-center gap-3 pl-9 text-xs">
                            {record.captured && (
                              <span className="inline-flex items-center gap-1 text-red-400 font-medium">
                                <Swords className="w-3 h-3" />
                                <span>
                                  Ăn quân: {PIECE_NAMES_VI[record.captured.type]} (
                                  {getPieceChar(record.captured.type, record.captured.color)})
                                </span>
                              </span>
                            )}
                            {record.note && (
                              <span className="inline-flex items-center gap-1 text-amber-300/85 font-mono-tabular">
                                <Bot className="w-3 h-3 text-amber-400" />
                                <span>{record.note.replace(/^[🤖📖]\s*/u, '')}</span>
                              </span>
                            )}
                          </div>
                        )}
                      </li>
                    );
                  })}
                  <div ref={logEndRef} />
                </ul>
              )}
            </div>
          </div>
        </div>
      )}
    </>
  );
};
