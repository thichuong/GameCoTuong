import React from 'react';
import type { GameMode, Piece, PieceColor } from '../../types/game.js';
import type { GroupedPiece } from '../../utils/capturedPieces.js';

export function getTrayRoleLabel(
  side: PieceColor,
  gameMode: GameMode,
  playerSide: PieceColor,
  difficulty: number,
  isSetupMode: boolean
): string {
  const isRed = side === 'red';
  if (gameMode === 'HumanVsComputer') {
    return side === playerSide ? 'Bạn (Người)' : `AI Máy · Cấp ${difficulty}`;
  }
  if (gameMode === 'ComputerVsComputer') {
    return `AI (${isRed ? 'Đỏ' : 'Đen'})`;
  }
  if (gameMode === 'Puzzle') {
    if (isSetupMode) {
      return isRed ? 'Giữ quân Đen bị ăn' : 'Giữ quân Đỏ bị ăn';
    }
    return side === playerSide
      ? 'Bạn (Giải thế)'
      : `Đối thủ · ${isRed ? 'Đỏ' : 'Đen'}`;
  }
  return isRed ? 'Tiên thủ' : 'Hậu thủ';
}

interface MiniWoodenPieceProps {
  piece: GroupedPiece;
  isSetupMode: boolean;
  selectedSetupPiece: Piece | null;
  onSelectSetupPiece?: (piece: Piece | null) => void;
  onDragStartSetupPiece?: (piece: Piece) => void;
  onDragEndSetupPiece?: () => void;
}

export const MiniWoodenPiece: React.FC<MiniWoodenPieceProps> = ({
  piece,
  isSetupMode,
  selectedSetupPiece,
  onSelectSetupPiece,
  onDragStartSetupPiece,
  onDragEndSetupPiece,
}) => {
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
