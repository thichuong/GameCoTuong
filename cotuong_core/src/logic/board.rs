use crate::engine::zobrist::ZobristKeys;
use crate::engine::Move;
use crate::logic::eval_constants::{get_piece_value, get_pst_value};
use serde::{Deserialize, Serialize};

pub type Bitboard = u128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardCoordinate {
    pub row: usize,
    pub col: usize,
}

impl BoardCoordinate {
    pub fn new(row: usize, col: usize) -> Option<Self> {
        if row < 10 && col < 9 {
            Some(Self { row, col })
        } else {
            None
        }
    }

    /// Creates a coordinate without checking bounds.
    /// # Safety
    /// Caller must ensure row < 10 and col < 9.
    pub unsafe fn new_unchecked(row: usize, col: usize) -> Self {
        Self { row, col }
    }

    pub fn index(self) -> usize {
        self.row * 9 + self.col
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Color {
    Red,
    Black,
}

impl Color {
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Red => Self::Black,
            Self::Black => Self::Red,
        }
    }

    pub const fn index(self) -> usize {
        match self {
            Self::Red => 0,
            Self::Black => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PieceType {
    General = 0,
    Advisor = 1,
    Elephant = 2,
    Horse = 3,
    Chariot = 4,
    Cannon = 5,
    Soldier = 6,
}

impl PieceType {
    pub const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Piece {
    pub piece_type: PieceType,
    pub color: Color,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Board {
    // Bitboards for each piece type and color
    // Index: color.index() * 7 + piece_type.index()
    pub bitboards: [Bitboard; 14],
    pub occupied: Bitboard,
    // Mailbox for O(1) lookup
    #[serde(with = "serde_big_array::BigArray")]
    pub grid: [Option<Piece>; 90],

    // Fast occupancy for sliding pieces
    pub occupied_rows: [u16; 10], // 9 bits used
    pub occupied_cols: [u16; 9],  // 10 bits used

    pub zobrist_hash: u64,
    pub red_material: i32,
    pub black_material: i32,
    pub red_pst: i32,
    pub black_pst: i32,
}

impl Board {
    pub fn to_fen_string(&self, turn: Color) -> String {
        let mut fen = String::new();
        // 1. Piece placement
        // Iterate from rank 9 (top) to 0 (bottom)
        for r in (0..10).rev() {
            let mut empty_count = 0;
            for c in 0..9 {
                // Buffer bound check is handled by loop range
                let coord = unsafe { BoardCoordinate::new_unchecked(r, c) };
                if let Some(piece) = self.get_piece(coord) {
                    if empty_count > 0 {
                        fen.push_str(&empty_count.to_string());
                        empty_count = 0;
                    }
                    let char_code = match piece.piece_type {
                        PieceType::General => 'k',
                        PieceType::Advisor => 'a',
                        PieceType::Elephant => 'b',
                        PieceType::Horse => 'n',
                        PieceType::Chariot => 'r',
                        PieceType::Cannon => 'c',
                        PieceType::Soldier => 'p',
                    };
                    let final_char = if piece.color == Color::Red {
                        char_code.to_ascii_uppercase()
                    } else {
                        char_code
                    };
                    fen.push(final_char);
                } else {
                    empty_count += 1;
                }
            }
            if empty_count > 0 {
                fen.push_str(&empty_count.to_string());
            }
            if r > 0 {
                fen.push('/');
            }
        }

        // 2. Turn
        fen.push(' ');
        fen.push(if turn == Color::Red { 'w' } else { 'b' });

        fen
    }

    pub fn from_fen(fen: &str) -> Result<(Self, Color), String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 2 {
            return Err("Invalid FEN string: missing parts".to_string());
        }

        let mut board = Self::new();
        board.clear();

        // 1. Piece placement
        let rows: Vec<&str> = parts[0].split('/').collect();
        if rows.len() != 10 {
            return Err("Invalid FEN: must have 10 rows".to_string());
        }

        for (r_idx, row_str) in rows.iter().enumerate() {
            let r = 9 - r_idx;
            let mut c = 0;
            for ch in row_str.chars() {
                if c >= 9 {
                    return Err(format!("Row {} is too long", r));
                }
                if let Some(digit) = ch.to_digit(10) {
                    c += digit as usize;
                } else {
                    let color = if ch.is_uppercase() {
                        Color::Red
                    } else {
                        Color::Black
                    };
                    let piece_type = match ch.to_ascii_lowercase() {
                        'k' => PieceType::General,
                        'a' => PieceType::Advisor,
                        'b' => PieceType::Elephant,
                        'n' => PieceType::Horse,
                        'r' => PieceType::Chariot,
                        'c' => PieceType::Cannon,
                        'p' => PieceType::Soldier,
                        _ => return Err(format!("Invalid piece char: {}", ch)),
                    };

                    if let Some(pos) = BoardCoordinate::new(r, c) {
                        board.add_piece(pos, piece_type, color);
                    }
                    c += 1;
                }
            }
            if c != 9 {
                return Err(format!("Row {} has invalid length {}", r, c));
            }
        }

        // 2. Turn
        let turn = match parts[1] {
            "w" | "r" => Color::Red,
            "b" => Color::Black,
            _ => return Err("Invalid turn color".to_string()),
        };

        // Recalculate internals
        board.zobrist_hash = board.calculate_initial_hash();
        if turn == Color::Black {
            let keys = ZobristKeys::get();
            board.zobrist_hash ^= keys.side_key;
        }
        board.calculate_initial_score();

        Ok((board, turn))
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl Board {
    #[must_use]
    pub fn new() -> Self {
        let mut board = Self {
            bitboards: [0; 14],
            occupied: 0,
            grid: [None; 90],
            occupied_rows: [0; 10],
            occupied_cols: [0; 9],
            zobrist_hash: 0,
            red_material: 0,
            black_material: 0,
            red_pst: 0,
            black_pst: 0,
        };
        board.setup_initial_position();
        board.zobrist_hash = board.calculate_initial_hash();
        board.calculate_initial_score();
        board
    }

    fn setup_initial_position(&mut self) {
        // Setup Red (Bottom, rows 0-4)
        self.setup_pieces(Color::Red, 0, 2, 3);

        // Setup Black (Top, rows 9-5)
        self.setup_pieces(Color::Black, 9, 7, 6);
    }

    fn setup_pieces(
        &mut self,
        color: Color,
        back_row: usize,
        cannon_row: usize,
        soldier_row: usize,
    ) {
        // Safe wrapper to avoid panic
        let make_coord = |r, c| BoardCoordinate::new(r, c);

        let pieces = [
            PieceType::Chariot,
            PieceType::Horse,
            PieceType::Elephant,
            PieceType::Advisor,
            PieceType::General,
            PieceType::Advisor,
            PieceType::Elephant,
            PieceType::Horse,
            PieceType::Chariot,
        ];

        // Back row
        for (col, &pt) in pieces.iter().enumerate() {
            if let Some(pos) = make_coord(back_row, col) {
                self.add_piece(pos, pt, color);
            }
        }

        // Cannons
        if let Some(pos) = make_coord(cannon_row, 1) {
            self.add_piece(pos, PieceType::Cannon, color);
        }
        if let Some(pos) = make_coord(cannon_row, 7) {
            self.add_piece(pos, PieceType::Cannon, color);
        }

        // Soldiers
        for col in (0..9).step_by(2) {
            if let Some(pos) = make_coord(soldier_row, col) {
                self.add_piece(pos, PieceType::Soldier, color);
            }
        }
    }

    pub fn move_piece_quiet(
        &mut self,
        from: BoardCoordinate,
        to: BoardCoordinate,
    ) -> Option<Piece> {
        let piece = self.get_piece(from)?;
        self.remove_piece(from, piece.piece_type, piece.color);
        let captured = self.get_piece(to);
        if let Some(cap) = captured {
            self.remove_piece(to, cap.piece_type, cap.color);
        }
        self.add_piece(to, piece.piece_type, piece.color);
        captured
    }

    pub fn undo_move_quiet(
        &mut self,
        from: BoardCoordinate,
        to: BoardCoordinate,
        captured: Option<Piece>,
    ) {
        if let Some(piece) = self.get_piece(to) {
            self.remove_piece(to, piece.piece_type, piece.color);
            self.add_piece(from, piece.piece_type, piece.color);
            if let Some(cap) = captured {
                self.add_piece(to, cap.piece_type, cap.color);
            }
        } else {
            // Logic error: Tried to undo move but piece not at 'to'
            // For production safety, we just log or ignore rather than panic.
            // In severe cases, we could return a Result, but method signature is void.
            // We'll proceed safely by doing nothing if piece is missing.
        }
    }

    pub fn set_piece(&mut self, pos: BoardCoordinate, piece: Option<Piece>) {
        // Remove existing
        if let Some(p) = self.get_piece(pos) {
            self.remove_piece(pos, p.piece_type, p.color);
        }
        // Add new
        if let Some(p) = piece {
            self.add_piece(pos, p.piece_type, p.color);
        }
    }

    pub fn clear(&mut self) {
        self.bitboards = [0; 14];
        self.occupied = 0;
        self.grid = [None; 90];
        self.occupied_rows = [0; 10];
        self.occupied_cols = [0; 9];
        self.zobrist_hash = 0;
        self.red_material = 0;
        self.black_material = 0;
        self.red_pst = 0;
        self.black_pst = 0;
    }

    // Helper to add a piece
    pub fn add_piece(&mut self, pos: BoardCoordinate, piece_type: PieceType, color: Color) {
        let sq = pos.index();
        let bit = 1u128 << sq;
        let idx = color.index() * 7 + piece_type.index();
        self.bitboards[idx] |= bit;
        self.occupied |= bit;
        self.grid[sq] = Some(Piece { piece_type, color });

        self.occupied_rows[pos.row] |= 1 << pos.col;
        self.occupied_cols[pos.col] |= 1 << pos.row;
    }

    // Helper to remove a piece
    fn remove_piece(&mut self, pos: BoardCoordinate, piece_type: PieceType, color: Color) {
        let sq = pos.index();
        let bit = 1u128 << sq;
        let idx = color.index() * 7 + piece_type.index();
        self.bitboards[idx] &= !bit;
        self.occupied &= !bit;
        self.grid[sq] = None;

        self.occupied_rows[pos.row] &= !(1 << pos.col);
        self.occupied_cols[pos.col] &= !(1 << pos.row);
    }

    #[must_use]
    pub const fn square_index(row: usize, col: usize) -> usize {
        row * 9 + col
    }

    #[must_use]
    pub const fn index_to_coord(sq: usize) -> (usize, usize) {
        (sq / 9, sq % 9)
    }

    #[must_use]
    pub fn get_piece(&self, pos: BoardCoordinate) -> Option<Piece> {
        self.grid[pos.index()]
    }

    pub fn get_color_bb(&self, color: Color) -> u128 {
        let start = color.index() * 7;
        let mut bb = 0;
        for i in 0..7 {
            bb |= self.bitboards[start + i];
        }
        bb
    }

    pub fn calculate_initial_hash(&self) -> u64 {
        let keys = ZobristKeys::get();
        let mut hash = 0;
        for r in 0..10 {
            for c in 0..9 {
                // Safety: Loop bounds are correct
                let pos = unsafe { BoardCoordinate::new_unchecked(r, c) };
                if let Some(piece) = self.get_piece(pos) {
                    hash ^= keys.get_piece_key(piece.piece_type, piece.color, r, c);
                }
            }
        }
        hash
    }

    pub fn calculate_initial_score(&mut self) {
        self.red_material = 0;
        self.black_material = 0;
        self.red_pst = 0;
        self.black_pst = 0;

        for r in 0..10 {
            for c in 0..9 {
                let pos = unsafe { BoardCoordinate::new_unchecked(r, c) };
                if let Some(piece) = self.get_piece(pos) {
                    let val = get_piece_value(piece.piece_type);
                    let pst = get_pst_value(piece.piece_type, piece.color, r, c);

                    if piece.color == Color::Red {
                        self.red_material += val;
                        self.red_pst += pst;
                    } else {
                        self.black_material += val;
                        self.black_pst += pst;
                    }
                }
            }
        }
    }

    pub fn apply_null_move(&mut self) {
        let keys = ZobristKeys::get();
        self.zobrist_hash ^= keys.side_key;
    }

    pub fn apply_move(&mut self, mv: &Move, _turn: Color) {
        let keys = ZobristKeys::get();
        let from_row = mv.from_row as usize;
        let from_col = mv.from_col as usize;
        let to_row = mv.to_row as usize;
        let to_col = mv.to_col as usize;

        // Safety: Engine moves should be valid.
        let from = if let Some(pos) = BoardCoordinate::new(from_row, from_col) {
            pos
        } else {
            return;
        };
        let to = if let Some(pos) = BoardCoordinate::new(to_row, to_col) {
            pos
        } else {
            return;
        };

        // 1. Get piece at source
        let piece = if let Some(p) = self.get_piece(from) {
            p
        } else {
            return;
        };

        // Remove from source
        self.remove_piece(from, piece.piece_type, piece.color);
        self.zobrist_hash ^= keys.get_piece_key(piece.piece_type, piece.color, from_row, from_col);

        // Update Score (Remove from source)
        let pst_from = get_pst_value(piece.piece_type, piece.color, from_row, from_col);
        if piece.color == Color::Red {
            self.red_pst -= pst_from;
        } else {
            self.black_pst -= pst_from;
        }

        // 2. Remove captured piece (if any)
        if let Some(captured) = self.get_piece(to) {
            self.remove_piece(to, captured.piece_type, captured.color);
            self.zobrist_hash ^=
                keys.get_piece_key(captured.piece_type, captured.color, to_row, to_col);

            // Update Score (Remove captured)
            let cap_val = get_piece_value(captured.piece_type);
            let cap_pst = get_pst_value(captured.piece_type, captured.color, to_row, to_col);

            if captured.color == Color::Red {
                self.red_material -= cap_val;
                self.red_pst -= cap_pst;
            } else {
                self.black_material -= cap_val;
                self.black_pst -= cap_pst;
            }
        }

        // 3. Place piece at destination
        self.add_piece(to, piece.piece_type, piece.color);
        self.zobrist_hash ^= keys.get_piece_key(piece.piece_type, piece.color, to_row, to_col);

        // Update Score (Add to dest)
        let pst_to = get_pst_value(piece.piece_type, piece.color, to_row, to_col);
        if piece.color == Color::Red {
            self.red_pst += pst_to;
        } else {
            self.black_pst += pst_to;
        }

        // 4. Switch turn hash
        self.zobrist_hash ^= keys.side_key;
    }

    pub fn undo_move(&mut self, mv: &Move, captured: Option<Piece>, _turn: Color) {
        let keys = ZobristKeys::get();
        let from_row = mv.from_row as usize;
        let from_col = mv.from_col as usize;
        let to_row = mv.to_row as usize;
        let to_col = mv.to_col as usize;

        let from = if let Some(pos) = BoardCoordinate::new(from_row, from_col) {
            pos
        } else {
            return;
        };
        let to = if let Some(pos) = BoardCoordinate::new(to_row, to_col) {
            pos
        } else {
            return;
        };

        // 4. Switch turn hash (back)
        self.zobrist_hash ^= keys.side_key;

        // 3. Move piece back from destination to source
        let piece = if let Some(p) = self.get_piece(to) {
            p
        } else {
            return;
        };

        // Remove from destination
        self.remove_piece(to, piece.piece_type, piece.color);
        self.zobrist_hash ^= keys.get_piece_key(piece.piece_type, piece.color, to_row, to_col);

        // Update Score (Remove from dest)
        let pst_to = get_pst_value(piece.piece_type, piece.color, to_row, to_col);
        if piece.color == Color::Red {
            self.red_pst -= pst_to;
        } else {
            self.black_pst -= pst_to;
        }

        // Place back at source
        self.add_piece(from, piece.piece_type, piece.color);
        self.zobrist_hash ^= keys.get_piece_key(piece.piece_type, piece.color, from_row, from_col);

        // Update Score (Add to source)
        let pst_from = get_pst_value(piece.piece_type, piece.color, from_row, from_col);
        if piece.color == Color::Red {
            self.red_pst += pst_from;
        } else {
            self.black_pst += pst_from;
        }

        // 2. Restore captured piece (if any)
        if let Some(cap) = captured {
            self.add_piece(to, cap.piece_type, cap.color);
            self.zobrist_hash ^= keys.get_piece_key(cap.piece_type, cap.color, to_row, to_col);

            // Update Score (Restore captured)
            let cap_val = get_piece_value(cap.piece_type);
            let cap_pst = get_pst_value(cap.piece_type, cap.color, to_row, to_col);

            if cap.color == Color::Red {
                self.red_material += cap_val;
                self.red_pst += cap_pst;
            } else {
                self.black_material += cap_val;
                self.black_pst += cap_pst;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_setup() {
        let board = Board::new();
        // Check Red General
        let piece = board
            .get_piece(BoardCoordinate::new(0, 4).unwrap())
            .unwrap();
        assert_eq!(piece.piece_type, PieceType::General);
        assert_eq!(piece.color, Color::Red);

        // Check Black General
        let piece = board
            .get_piece(BoardCoordinate::new(9, 4).unwrap())
            .unwrap();
        assert_eq!(piece.piece_type, PieceType::General);
        assert_eq!(piece.color, Color::Black);
    }

    #[test]
    fn test_fen_generation() {
        let board = Board::new();
        let fen = board.to_fen_string(Color::Red);
        // Standard starting FEN
        assert_eq!(
            fen,
            "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w"
        );
    }

    #[test]
    fn test_apply_move() {
        let mut board = Board::new();
        // Move Red Central Soldier forward
        let mv = Move {
            from_row: 3,
            from_col: 4,
            to_row: 4,
            to_col: 4,
            score: 0,
        };
        board.apply_move(&mv, Color::Red);

        assert!(board
            .get_piece(BoardCoordinate::new(3, 4).unwrap())
            .is_none());
        let piece = board
            .get_piece(BoardCoordinate::new(4, 4).unwrap())
            .unwrap();
        assert_eq!(piece.piece_type, PieceType::Soldier);
        assert_eq!(piece.color, Color::Red);
    }
}

pub struct BitboardIterator {
    bb: u128,
}

impl BitboardIterator {
    pub const fn new(bb: u128) -> Self {
        Self { bb }
    }
}

impl Iterator for BitboardIterator {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bb == 0 {
            None
        } else {
            let lsb = self.bb.trailing_zeros() as usize;
            self.bb &= self.bb - 1;
            Some(lsb)
        }
    }
}
