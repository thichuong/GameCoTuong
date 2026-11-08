import React from 'react';
import type { GameMode, Piece, PieceColor } from '../types/game.js';
import type { GroupedPiece } from '../utils/capturedPieces.js';

interface CapturedPiecesTrayProps {
  /** The side this tray represents (Red or Black) */
  side: PieceColor;
  /** Whether it is currently this side's turn */
  isTurn: boolean;
  /** Whether this side is currently thinking (AI) */
  isThinking?: boolean;
  /** Captured enemy pieces in possession of this player */
  capturedPieces: GroupedPiece[];
  /** Total material points of captured pieces */
  totalCapturedValue: number;
  /** Material advantage relative to opponent (positive if ahead) */
  materialAdvantage: number;
  /** Game mode and player side to show accurate role badges */
  gameMode: GameMode;
  playerSide: PieceColor;
  difficulty?: number;
  /** Layout orientation: 'sides' (vertical sidebar) or 'stacked' (horizontal bar) */
  orientation: 'sides' | 'stacked';
  /** Whether the board is currently in Puzzle setup mode (allows dragging pieces onto board) */
  isSetupMode?: boolean;
  /** Currently selected piece from tray in setup mode */
  selectedSetupPiece?: Piece | null;
  /** Callback when a piece in the tray is clicked in setup mode */
  onSelectSetupPiece?: (piece: Piece | null) => void;
  /** Callback when a piece drag starts from the tray in setup mode */
  onDragStartSetupPiece?: (piece: Piece) => void;
  /** Callback when a piece drag ends from the tray in setup mode */
  onDragEndSetupPiece?: () => void;
}

export const CapturedPiecesTray: React.FC<CapturedPiecesTrayProps> = ({
  side,
  isTurn,
  isThinking = false,
  capturedPieces,
  totalCapturedValue,
  materialAdvantage,
  gameMode,
  playerSide,
  difficulty = 1,
  orientation,
  isSetupMode = false,
  selectedSetupPiece = null,
  onSelectSetupPiece,
  onDragStartSetupPiece,
  onDragEndSetupPiece,
}) => {
  const isRed = side === 'red';

  // Determine role label
  let roleLabel = '';
  if (gameMode === 'HumanVsComputer') {
    if (side === playerSide) {
      roleLabel = 'Bạn (Người)';
    } else {
      roleLabel = `AI Máy · Cấp ${difficulty}`;
    }
  } else if (gameMode === 'ComputerVsComputer') {
    roleLabel = `AI (${isRed ? 'Đỏ' : 'Đen'})`;
  } else if (gameMode === 'Puzzle') {
    if (isSetupMode) {
      roleLabel = isRed ? 'Giữ quân Đen bị ăn' : 'Giữ quân Đỏ bị ăn';
    } else if (side === playerSide) {
      roleLabel = 'Bạn (Giải thế)';
    } else {
      roleLabel = `Đối thủ · ${isRed ? 'Đỏ' : 'Đen'}`;
    }
  } else {
    roleLabel = isRed ? 'Tiên thủ' : 'Hậu thủ';
  }

  const pieceCount = capturedPieces.reduce((acc, p) => acc + p.count, 0);

  // Renders miniature authentic wooden piece disc
  const renderMiniPiece = (piece: GroupedPiece) => {
    // Note: If Red captured a piece, that piece is Black; if Black captured, that piece is Red.
    const isPieceRed = piece.color === 'red';
    const isSelected =
      isSetupMode &&
      selectedSetupPiece?.type === piece.type &&
      selectedSetupPiece?.color === piece.color;

    const handleDragStart = (e: React.DragEvent<HTMLDivElement>) => {
      if (!isSetupMode) return;
      const payload: Piece = { type: piece.type, color: piece.color };
      e.dataTransfer.setData('application/xiangqi-piece', JSON.stringify(payload));
      e.dataTransfer.setData('text/plain', JSON.stringify(payload));
      e.dataTransfer.effectAllowed = 'copyMove';
      onDragStartSetupPiece?.(payload);
    };

    const handleDragEnd = () => {
      if (!isSetupMode) return;
      onDragEndSetupPiece?.();
    };

    const handleClick = () => {
      if (!isSetupMode || !onSelectSetupPiece) return;
      if (isSelected) {
        onSelectSetupPiece(null);
      } else {
        onSelectSetupPiece({ type: piece.type, color: piece.color });
      }
    };

    return (
      <div
        key={`${piece.color}-${piece.type}`}
        draggable={isSetupMode}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
        onClick={handleClick}
        className={`relative group flex items-center justify-center shrink-0 transition-transform ${
          isSetupMode
            ? selectedSetupPiece
              ? 'hover:scale-115'
              : 'cursor-grab active:cursor-grabbing hover:scale-115'
            : 'hover:scale-110 cursor-help'
        }`}
        title={
          isSetupMode
            ? `Kéo vào bàn cờ hoặc nhấp chọn: ${piece.nameVi} (Còn ${piece.count})`
            : `${piece.nameVi} (${piece.baseValue}đ) · Số lượng: ${piece.count}`
        }
      >
        {/* Mini 3D wooden piece */}
        <div
          className={`w-7 h-7 sm:w-8 sm:h-8 rounded-full flex items-center justify-center font-display-serif font-bold text-sm sm:text-base border shadow-md select-none transition-all ${
            isPieceRed
              ? 'bg-gradient-to-br from-[#FFFDF8] via-[#F8EDDA] to-[#E5CCA5] border-[#B91C1C] text-[#B91C1C] shadow-[0_2px_4px_rgba(185,28,28,0.25)]'
              : 'bg-gradient-to-br from-[#FAF5EC] via-[#ECE0CA] to-[#D5BE97] border-[#262626] text-[#171717] shadow-[0_2px_4px_rgba(0,0,0,0.35)]'
          } ${
            isSelected
              ? 'ring-2 ring-amber-400 scale-110 shadow-[0_0_12px_rgba(245,158,11,0.75)]'
              : ''
          }`}
        >
          {/* Inner ring */}
          <div
            className={`w-[85%] h-[85%] rounded-full border flex items-center justify-center ${
              isPieceRed ? 'border-[#DC2626]/70' : 'border-[#404040]/70'
            }`}
          >
            <span className="leading-none pt-0.5">{piece.symbol}</span>
          </div>
        </div>

        {/* Count Badge for multiple pieces */}
        {piece.count > 1 && (
          <span
            className={`absolute -top-1 -right-1 px-1 py-0.2 rounded-full text-[10px] font-mono-tabular font-bold leading-tight shadow-sm border pointer-events-none ${
              isPieceRed
                ? 'bg-[#991B1B] text-white border-red-400/50'
                : 'bg-stone-800 text-stone-100 border-stone-600'
            }`}
          >
            ×{piece.count}
          </span>
        )}
      </div>
    );
  };

  if (orientation === 'sides') {
    // Vertical sidebar beside the board (Desktop / Landscape)
    return (
      <aside
        aria-label={`Khu vực quân đã ăn của ${isRed ? 'Bên Đỏ' : 'Bên Đen'}`}
        className={`w-36 sm:w-44 h-full min-h-0 rounded-2xl p-3 flex flex-col justify-between border transition-all duration-300 shadow-xl ${
          isTurn
            ? isRed
              ? 'bg-[#1F1714] border-red-500/50 shadow-[0_0_20px_rgba(220,38,38,0.15)] ring-1 ring-red-500/30'
              : 'bg-[#1A1A1A] border-amber-500/50 shadow-[0_0_20px_rgba(245,158,11,0.15)] ring-1 ring-amber-500/30'
            : 'bg-[#181412] border-stone-800/80 hover:border-stone-700'
        }`}
      >
        {/* Top: Player Card Header */}
        <div>
          <div className="flex items-center gap-2.5 pb-2.5 border-b border-stone-800/80">
            {/* Player King Avatar */}
            <div
              className={`w-9 h-9 rounded-full flex items-center justify-center font-display-serif font-bold text-base border shadow-md shrink-0 ${
                isRed
                  ? 'bg-gradient-to-br from-red-700 to-red-950 border-red-500/60 text-red-100'
                  : 'bg-gradient-to-br from-stone-800 to-stone-950 border-stone-600 text-stone-200'
              }`}
            >
              {isRed ? '帥' : '將'}
            </div>

            <div className="min-w-0 flex-1">
              <div className="flex items-center justify-between gap-1">
                <span
                  className={`text-xs sm:text-sm font-bold truncate ${
                    isRed ? 'text-red-400' : 'text-amber-300'
                  }`}
                >
                  {isRed ? 'Bên Đỏ' : 'Bên Đen'}
                </span>
                {isTurn && (
                  <span
                    className={`inline-block w-2 h-2 rounded-full shrink-0 animate-ping ${
                      isRed ? 'bg-red-400' : 'bg-amber-400'
                    }`}
                  />
                )}
              </div>
              <p className="text-[11px] text-stone-400 truncate">{roleLabel}</p>
            </div>
          </div>

          {/* Turn / Thinking status banner */}
          <div className="mt-2 flex items-center justify-between text-[11px]">
            <span className="text-stone-400">Trạng thái:</span>
            {isSetupMode ? (
              <span className="text-amber-300 font-semibold">Xếp cờ thế</span>
            ) : isTurn ? (
              <span
                className={`font-semibold flex items-center gap-1 ${
                  isRed ? 'text-red-400' : 'text-amber-300'
                }`}
              >
                {isThinking ? (
                  <>
                    <span className="inline-block w-1.5 h-1.5 rounded-full bg-current animate-pulse" />
                    Đang tính...
                  </>
                ) : (
                  'Lượt đi'
                )}
              </span>
            ) : (
              <span className="text-stone-500">Chờ đối thủ</span>
            )}
          </div>

          {/* Material Advantage Pill */}
          <div className="mt-2 flex items-center justify-between text-[11px]">
            <span className="text-stone-400">Ưu thế quân:</span>
            {materialAdvantage > 0 ? (
              <span className="px-2 py-0.5 rounded-md bg-emerald-500/20 border border-emerald-500/35 text-emerald-300 font-mono-tabular font-bold text-xs">
                +{materialAdvantage}
              </span>
            ) : materialAdvantage < 0 ? (
              <span className="px-1.5 py-0.5 rounded-md bg-stone-800/80 text-stone-400 font-mono-tabular text-xs">
                {materialAdvantage}
              </span>
            ) : (
              <span className="text-stone-500 font-mono-tabular text-xs">Cân bằng</span>
            )}
          </div>
        </div>

        {/* Center: Captured Pieces Tray */}
        <div className="my-3 flex-1 flex flex-col justify-start">
          <div className="flex items-center justify-between text-xs mb-1.5">
            <span className="text-stone-300 font-medium">
              {isSetupMode ? 'Kéo quân vào bàn:' : 'Quân đã ăn:'}
            </span>
            <span className="font-mono-tabular text-stone-400 text-[11px]">
              {pieceCount} quân
            </span>
          </div>

          {capturedPieces.length > 0 ? (
            <div className="grid grid-cols-3 sm:grid-cols-3 gap-2 p-2 rounded-xl bg-[#14110F]/80 border border-stone-800/90 min-h-[120px] max-h-[260px] overflow-y-auto content-start">
              {capturedPieces.map((p) => renderMiniPiece(p))}
            </div>
          ) : (
            <div className="flex-1 flex flex-col items-center justify-center p-3 rounded-xl border border-dashed border-stone-800/90 text-center text-stone-500 text-[11px] gap-1 min-h-[100px]">
              <div className="w-6 h-6 rounded-full border border-dashed border-stone-700/80 flex items-center justify-center opacity-40">
                ·
              </div>
              <span>{isSetupMode ? 'Đã đặt hết lên bàn' : 'Chưa ăn quân'}</span>
            </div>
          )}
        </div>

        {/* Bottom: Total Captured Material Value */}
        <div className="pt-2 border-t border-stone-800/80 flex items-center justify-between text-[11px] text-stone-400">
          <span>Tổng điểm ăn:</span>
          <span className="font-mono-tabular font-semibold text-stone-200">
            {totalCapturedValue}đ
          </span>
        </div>
      </aside>
    );
  }

  // Stacked horizontal bar above or below the board (Mobile / Portrait)
  return (
    <section
      aria-label={`Khu vực quân đã ăn của ${isRed ? 'Bên Đỏ' : 'Bên Đen'}`}
      className={`w-full max-w-[560px] rounded-xl px-3 py-2 flex items-center justify-between gap-2.5 border transition-all duration-300 shadow-md ${
        isTurn
          ? isRed
            ? 'bg-[#1F1714] border-red-500/50 ring-1 ring-red-500/30'
            : 'bg-[#1A1A1A] border-amber-500/50 ring-1 ring-amber-500/30'
          : 'bg-[#181412] border-stone-800/80'
      }`}
    >
      {/* Left: Player identity & Turn */}
      <div className="flex items-center gap-2 shrink-0">
        <div
          className={`w-7 h-7 sm:w-8 sm:h-8 rounded-full flex items-center justify-center font-display-serif font-bold text-xs sm:text-sm border shadow-sm shrink-0 ${
            isRed
              ? 'bg-gradient-to-br from-red-700 to-red-950 border-red-500/60 text-red-100'
              : 'bg-gradient-to-br from-stone-800 to-stone-950 border-stone-600 text-stone-200'
          }`}
        >
          {isRed ? '帥' : '將'}
        </div>

        <div className="flex flex-col">
          <div className="flex items-center gap-1.5">
            <span
              className={`text-xs font-bold ${
                isRed ? 'text-red-400' : 'text-amber-300'
              }`}
            >
              {isRed ? 'Bên Đỏ' : 'Bên Đen'}
            </span>
            {isTurn && (
              <span
                className={`text-[10px] px-1.5 py-0.2 rounded font-semibold ${
                  isRed
                    ? 'bg-red-500/20 text-red-300 border border-red-500/40'
                    : 'bg-amber-500/20 text-amber-300 border border-amber-500/40'
                }`}
              >
                {isThinking ? 'Đang tính...' : 'Lượt đi'}
              </span>
            )}
          </div>
          <span className="text-[10px] text-stone-400 truncate max-w-[110px]">
            {roleLabel}
          </span>
        </div>
      </div>

      {/* Center: Captured Pieces row */}
      <div className="flex-1 flex items-center justify-end sm:justify-center gap-1.5 overflow-x-auto py-0.5 px-1 min-w-0">
        {capturedPieces.length > 0 ? (
          capturedPieces.map((p) => renderMiniPiece(p))
        ) : (
          <span className="text-[11px] text-stone-500 italic">
            {isSetupMode ? 'Đã đặt hết lên bàn' : 'Chưa ăn quân'}
          </span>
        )}
      </div>

      {/* Right: Advantage & Points summary */}
      <div className="flex items-center gap-2 shrink-0 pl-1 border-l border-stone-800">
        {materialAdvantage > 0 && (
          <span className="px-1.5 py-0.5 rounded bg-emerald-500/20 border border-emerald-500/35 text-emerald-300 font-mono-tabular font-bold text-[11px]">
            +{materialAdvantage}
          </span>
        )}
        <div className="text-right leading-none">
          <span className="text-[10px] text-stone-400 block">Điểm ăn</span>
          <span className="font-mono-tabular text-xs font-semibold text-stone-200">
            {totalCapturedValue}đ
          </span>
        </div>
      </div>
    </section>
  );
};
