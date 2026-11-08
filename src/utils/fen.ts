import type { Piece, PieceColor, PieceType, Square } from '../types/game.js';

/**
 * Standard Xiangqi FEN piece mapping:
 * Red: K (General), A (Advisor), B (Elephant), N (Horse), R (Chariot), C (Cannon), P (Soldier)
 * Black: k, a, b, n, r, c, p
 * Note: 's' / 'S' or 'e' / 'E' are also accepted on input for soldiers/elephants.
 */

const PIECE_TYPE_MAP: Record<string, PieceType> = {
  k: 'General',
  a: 'Advisor',
  b: 'Elephant',
  e: 'Elephant',
  n: 'Horse',
  h: 'Horse',
  r: 'Chariot',
  c: 'Cannon',
  p: 'Soldier',
  s: 'Soldier',
};

const PIECE_CHAR_MAP: Record<PieceType, string> = {
  General: 'k',
  Advisor: 'a',
  Elephant: 'b',
  Horse: 'n',
  Chariot: 'r',
  Cannon: 'c',
  Soldier: 'p',
};

export interface ClassicPuzzle {
  id: string;
  name: string;
  subTitle: string;
  description: string;
  fen: string;
  turn: PieceColor;
  recommendedMode: 'HumanVsComputer' | 'HumanVsHuman' | 'ComputerVsComputer';
}

/**
 * Curated classic Xiangqi endgames (Cờ thế kinh điển)
 */
export const CLASSIC_PUZZLES: ClassicPuzzle[] = [
  {
    id: 'that-tinh-tu-hoi',
    name: 'Thất Tinh Tụ Hội (七星聚会)',
    subTitle: 'Đệ nhất danh cục cờ thế cổ',
    description: 'Thế cờ giang hồ lừng danh thời Thanh. Bên Đỏ công thủ biến hóa khôn lường, đòi hỏi từng nước đi chuẩn xác.',
    fen: '2b1kab2/9/3a5/2p1p1p2/9/4R4/2P1P1P2/9/4A4/2BAK1B2 w',
    turn: 'red',
    recommendedMode: 'HumanVsComputer',
  },
  {
    id: 'da-ma-thao-dien',
    name: 'Dã Mã Thao Điền (野马操田)',
    subTitle: 'Ngựa hoang trên đồng nội',
    description: 'Bên Đỏ có Xe Mã Binh đối đầu Xe Mã Tốt Đen trên diện rộng, tranh từng nhịp pháo mã.',
    fen: '2b1kab2/9/4b4/p3p3p/9/9/P3P3P/1N2B4/4A4/2BAK4 w',
    turn: 'red',
    recommendedMode: 'HumanVsComputer',
  },
  {
    id: 'don-ma-am-tuyen',
    name: 'Đơn Mã Ẩm Tuyền (单马饮泉)',
    subTitle: 'Ngựa đơn uống nước suối',
    description: 'Thế cờ tàn nghệ thuật: Mã Đỏ phối hợp cùng Binh khóa chặt cung tướng Đen.',
    fen: '3ak4/4a4/9/9/9/9/9/4N4/4p4/4K4 w',
    turn: 'red',
    recommendedMode: 'HumanVsComputer',
  },
  {
    id: 'tam-anh-chien-lu-bo',
    name: 'Tam Anh Chiến Lữ Bố (三英战吕布)',
    subTitle: 'Ba Binh vây hãm đại kích',
    description: 'Thế cờ mô phỏng tích Tam Quốc: ba Binh Đỏ kiên cường kết hợp ép bức Tướng Xe đối phương.',
    fen: '3ak4/4a4/9/3PPP3/9/9/9/9/4r4/4K4 w',
    turn: 'red',
    recommendedMode: 'HumanVsComputer',
  },
  {
    id: 'khong-minh-muon-ten',
    name: 'Khổng Minh Mượn Tên (孔明借箭)',
    subTitle: 'Pháo Binh liên hoàn sát',
    description: 'Bên Đỏ dùng Pháo và Binh tung đòn liên hoàn, ép Tướng Đen vào thế hiểm nguy không lối thoát.',
    fen: '3a1k3/4a4/9/4P4/9/9/9/4C4/9/4K4 w',
    turn: 'red',
    recommendedMode: 'HumanVsComputer',
  },
  {
    id: 'chariot-cannon-mate',
    name: 'Xe Pháo Tranh Phong (Sát cục nhập môn)',
    subTitle: 'Bài tập luyện cờ thế căn bản',
    description: 'Đỏ Xe Pháo phối hợp chiếu rút tuyệt đẹp để đoạt thắng lợi trước quân số đông của Đen.',
    fen: '3ak4/4a4/9/9/9/9/9/4C4/4R4/4K4 w',
    turn: 'red',
    recommendedMode: 'HumanVsComputer',
  },
];

/**
 * Convert 90 squares array to FEN string
 */
export function boardToFen(squares: Square[], turn: PieceColor): string {
  const grid: (Piece | null)[][] = Array.from({ length: 10 }, () => Array(9).fill(null));

  for (const sq of squares) {
    if (sq.row >= 0 && sq.row < 10 && sq.col >= 0 && sq.col < 9) {
      grid[sq.row]![sq.col] = sq.piece;
    }
  }

  const rows: string[] = [];

  // FEN orders rows from row 9 (Black back rank) down to row 0 (Red back rank)
  for (let r = 9; r >= 0; r--) {
    let emptyCount = 0;
    let rowStr = '';

    for (let c = 0; c < 9; c++) {
      const piece = grid[r]![c];
      if (!piece) {
        emptyCount++;
      } else {
        if (emptyCount > 0) {
          rowStr += emptyCount.toString();
          emptyCount = 0;
        }
        const charCode = PIECE_CHAR_MAP[piece.type] ?? 'p';
        rowStr += piece.color === 'red' ? charCode.toUpperCase() : charCode.toLowerCase();
      }
    }

    if (emptyCount > 0) {
      rowStr += emptyCount.toString();
    }
    rows.push(rowStr);
  }

  const fenTurn = turn === 'red' ? 'w' : 'b';
  return `${rows.join('/')} ${fenTurn} - - 0 1`;
}

/**
 * Parse FEN string to 90 squares and side to move
 */
export function fenToBoard(fen: string): { squares: Square[]; turn: PieceColor } {
  const parts = fen.trim().split(/\s+/);
  if (parts.length === 0 || !parts[0]) {
    throw new Error('FEN rỗng');
  }

  const rows = parts[0].split('/');
  if (rows.length !== 10) {
    throw new Error(`FEN phải có đúng 10 hàng (hiện có ${rows.length})`);
  }

  const squares: Square[] = [];
  const turn: PieceColor = parts[1]?.toLowerCase() === 'b' ? 'black' : 'red';

  for (let rIdx = 0; rIdx < 10; rIdx++) {
    const rowStr = rows[rIdx]!;
    const r = 9 - rIdx;
    let c = 0;

    for (const ch of rowStr) {
      const digit = Number.parseInt(ch, 10);
      if (!Number.isNaN(digit)) {
        for (let i = 0; i < digit; i++) {
          if (c < 9) {
            squares.push({ row: r, col: c, piece: null });
            c++;
          }
        }
      } else {
        const lower = ch.toLowerCase();
        const pieceType = PIECE_TYPE_MAP[lower];
        if (!pieceType) {
          throw new Error(`Ký tự quân cờ không hợp lệ trong FEN: ${ch}`);
        }
        const color: PieceColor = ch === ch.toUpperCase() ? 'red' : 'black';
        if (c < 9) {
          squares.push({
            row: r,
            col: c,
            piece: { type: pieceType, color },
          });
          c++;
        }
      }
    }

    while (c < 9) {
      squares.push({ row: r, col: c, piece: null });
      c++;
    }
  }

  // Sort squares by row, col for consistency
  squares.sort((a, b) => a.row !== b.row ? a.row - b.row : a.col - b.col);

  return { squares, turn };
}

/**
 * Validate a board setup before launching puzzle solving
 */
export function validateBoardSetup(squares: Square[]): {
  valid: boolean;
  errors: string[];
  redGenerals: [number, number][];
  blackGenerals: [number, number][];
} {
  const errors: string[] = [];
  const redGenerals: [number, number][] = [];
  const blackGenerals: [number, number][] = [];

  // 10x9 grid for quick lookup
  const grid: (Piece | null)[][] = Array.from({ length: 10 }, () => Array(9).fill(null));

  for (const sq of squares) {
    if (sq.piece) {
      grid[sq.row]![sq.col] = sq.piece;
      if (sq.piece.type === 'General') {
        if (sq.piece.color === 'red') {
          redGenerals.push([sq.row, sq.col]);
        } else {
          blackGenerals.push([sq.row, sq.col]);
        }
      }
    }
  }

  // Check Red General count
  if (redGenerals.length === 0) {
    errors.push('Thiếu Tướng Đỏ (帥). Bên Đỏ phải có đúng 1 Tướng trên bàn cờ.');
  } else if (redGenerals.length > 1) {
    errors.push(`Có ${redGenerals.length} Tướng Đỏ. Bên Đỏ chỉ được phép có 1 Tướng duy nhất.`);
  } else {
    const [r, c] = redGenerals[0]!;
    if (r < 0 || r > 2 || c < 3 || c > 5) {
      errors.push('Tướng Đỏ phải nằm trong Cung Tướng (hàng 0-2, cột 3-5).');
    }
  }

  // Check Black General count
  if (blackGenerals.length === 0) {
    errors.push('Thiếu Tướng Đen (將). Bên Đen phải có đúng 1 Tướng trên bàn cờ.');
  } else if (blackGenerals.length > 1) {
    errors.push(`Có ${blackGenerals.length} Tướng Đen. Bên Đen chỉ được phép có 1 Tướng duy nhất.`);
  } else {
    const [r, c] = blackGenerals[0]!;
    if (r < 7 || r > 9 || c < 3 || c > 5) {
      errors.push('Tướng Đen phải nằm trong Cung Tướng (hàng 7-9, cột 3-5).');
    }
  }

  // Check Flying Generals (Lộ mặt tướng)
  if (redGenerals.length === 1 && blackGenerals.length === 1) {
    const [rRed, cRed] = redGenerals[0]!;
    const [rBlack, cBlack] = blackGenerals[0]!;

    if (cRed === cBlack) {
      let hasScreen = false;
      const minR = Math.min(rRed, rBlack);
      const maxR = Math.max(rRed, rBlack);
      for (let r = minR + 1; r < maxR; r++) {
        if (grid[r]![cRed] !== null) {
          hasScreen = true;
          break;
        }
      }
      if (!hasScreen) {
        errors.push('Phạm luật "Lộ mặt tướng" (Flying General): Hai Tướng đang đối diện nhau trên cùng một cột mà không có quân chắn.');
      }
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    redGenerals,
    blackGenerals,
  };
}

/**
 * Create an empty 90-square board
 */
export function createEmptyBoard(): Square[] {
  const squares: Square[] = [];
  for (let r = 0; r < 10; r++) {
    for (let c = 0; c < 9; c++) {
      squares.push({ row: r, col: c, piece: null });
    }
  }
  return squares;
}

/**
 * Standard 32-piece starting FEN
 */
export const STANDARD_BOARD_FEN =
  'rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w - - 0 1';

/**
 * Default Puzzle setup FEN: Empty board with only the 2 Generals in their palaces
 */
export const TWO_GENERALS_FEN = '4k4/9/9/9/9/9/9/9/9/4K4 w - - 0 1';

/**
 * Create a board with only the 2 Generals at their default palace squares:
 * Red General at (0, 4) and Black General at (9, 4)
 */
export function createTwoGeneralsBoard(): Square[] {
  const squares = createEmptyBoard();
  for (const sq of squares) {
    if (sq.row === 0 && sq.col === 4) {
      sq.piece = { type: 'General', color: 'red' };
    } else if (sq.row === 9 && sq.col === 4) {
      sq.piece = { type: 'General', color: 'black' };
    }
  }
  return squares;
}

