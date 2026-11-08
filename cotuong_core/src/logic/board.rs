use crate::engine::nnue::{NnueAccumulator, NnueWeights};
use crate::engine::zobrist::ZobristKeys;
use crate::engine::Move;
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
    #[serde(skip)]
    pub nnue_acc: NnueAccumulator,
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
                        board.raw_add_piece(pos, piece_type, color);
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
        board.refresh_nnue();

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
            nnue_acc: NnueAccumulator::default(),
        };
        board.setup_initial_position();
        board.zobrist_hash = board.calculate_initial_hash();
        board.refresh_nnue();
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
                self.raw_add_piece(pos, pt, color);
            }
        }

        // Cannons
        if let Some(pos) = make_coord(cannon_row, 1) {
            self.raw_add_piece(pos, PieceType::Cannon, color);
        }
        if let Some(pos) = make_coord(cannon_row, 7) {
            self.raw_add_piece(pos, PieceType::Cannon, color);
        }

        // Soldiers
        for col in (0..9).step_by(2) {
            if let Some(pos) = make_coord(soldier_row, col) {
                self.raw_add_piece(pos, PieceType::Soldier, color);
            }
        }
    }

    pub fn move_piece_quiet(
        &mut self,
        from: BoardCoordinate,
        to: BoardCoordinate,
    ) -> Option<Piece> {
        let piece = self.get_piece(from)?;
        self.raw_remove_piece(from, piece.piece_type, piece.color);
        let captured = self.get_piece(to);
        if let Some(cap) = captured {
            self.raw_remove_piece(to, cap.piece_type, cap.color);
        }
        self.raw_add_piece(to, piece.piece_type, piece.color);
        captured
    }

    pub fn undo_move_quiet(
        &mut self,
        from: BoardCoordinate,
        to: BoardCoordinate,
        captured: Option<Piece>,
    ) {
        if let Some(piece) = self.get_piece(to) {
            self.raw_remove_piece(to, piece.piece_type, piece.color);
            self.raw_add_piece(from, piece.piece_type, piece.color);
            if let Some(cap) = captured {
                self.raw_add_piece(to, cap.piece_type, cap.color);
            }
        }
    }

    pub fn set_piece(&mut self, pos: BoardCoordinate, piece: Option<Piece>) {
        // Remove existing
        if let Some(p) = self.get_piece(pos) {
            self.raw_remove_piece(pos, p.piece_type, p.color);
        }
        // Add new
        if let Some(p) = piece {
            self.raw_add_piece(pos, p.piece_type, p.color);
        }
        self.refresh_nnue();
    }

    pub fn clear(&mut self) {
        self.bitboards = [0; 14];
        self.occupied = 0;
        self.grid = [None; 90];
        self.occupied_rows = [0; 10];
        self.occupied_cols = [0; 9];
        self.zobrist_hash = 0;
        self.refresh_nnue();
    }

    // Helper to add a piece and synchronize NNUE accumulator
    pub fn add_piece(&mut self, pos: BoardCoordinate, piece_type: PieceType, color: Color) {
        self.raw_add_piece(pos, piece_type, color);
        self.refresh_nnue();
    }

    #[inline]
    fn raw_add_piece(&mut self, pos: BoardCoordinate, piece_type: PieceType, color: Color) {
        let sq = pos.index();
        let bit = 1u128 << sq;
        let idx = color.index() * 7 + piece_type.index();
        self.bitboards[idx] |= bit;
        self.occupied |= bit;
        self.grid[sq] = Some(Piece { piece_type, color });

        self.occupied_rows[pos.row] |= 1 << pos.col;
        self.occupied_cols[pos.col] |= 1 << pos.row;
    }

    #[inline]
    fn raw_remove_piece(&mut self, pos: BoardCoordinate, piece_type: PieceType, color: Color) {
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

    /// Returns true if `color` has at least one major attacking piece (Horse, Chariot, or Cannon).
    #[inline]
    #[must_use]
    pub const fn has_major_pieces(&self, color: Color) -> bool {
        match color {
            Color::Red => (self.bitboards[3] | self.bitboards[4] | self.bitboards[5]) != 0,
            Color::Black => (self.bitboards[10] | self.bitboards[11] | self.bitboards[12]) != 0,
        }
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

    pub fn apply_null_move(&mut self) {
        let keys = ZobristKeys::get();
        self.zobrist_hash ^= keys.side_key;
    }

    #[inline]
    pub fn apply_move_board_only(
        &mut self,
        mv: &Move,
    ) -> Option<(Piece, Option<Piece>, usize, usize)> {
        let from_row = mv.from_row as usize;
        let from_col = mv.from_col as usize;
        let to_row = mv.to_row as usize;
        let to_col = mv.to_col as usize;

        let from = BoardCoordinate::new(from_row, from_col)?;
        let to = BoardCoordinate::new(to_row, to_col)?;
        let piece = self.get_piece(from)?;

        let keys = ZobristKeys::get();
        self.raw_remove_piece(from, piece.piece_type, piece.color);
        self.zobrist_hash ^= keys.get_piece_key(piece.piece_type, piece.color, from_row, from_col);

        let captured = self.get_piece(to);
        if let Some(cap) = captured {
            self.raw_remove_piece(to, cap.piece_type, cap.color);
            self.zobrist_hash ^= keys.get_piece_key(cap.piece_type, cap.color, to_row, to_col);
        }

        self.raw_add_piece(to, piece.piece_type, piece.color);
        self.zobrist_hash ^= keys.get_piece_key(piece.piece_type, piece.color, to_row, to_col);
        self.zobrist_hash ^= keys.side_key;

        Some((piece, captured, from.index(), to.index()))
    }

    #[inline]
    pub fn undo_move_board_only(&mut self, mv: &Move, piece: Piece, captured: Option<Piece>) {
        let from_row = mv.from_row as usize;
        let from_col = mv.from_col as usize;
        let to_row = mv.to_row as usize;
        let to_col = mv.to_col as usize;

        let Some(from) = BoardCoordinate::new(from_row, from_col) else {
            return;
        };
        let Some(to) = BoardCoordinate::new(to_row, to_col) else {
            return;
        };

        let keys = ZobristKeys::get();
        self.zobrist_hash ^= keys.side_key;

        self.raw_remove_piece(to, piece.piece_type, piece.color);
        self.zobrist_hash ^= keys.get_piece_key(piece.piece_type, piece.color, to_row, to_col);

        self.raw_add_piece(from, piece.piece_type, piece.color);
        self.zobrist_hash ^= keys.get_piece_key(piece.piece_type, piece.color, from_row, from_col);

        if let Some(cap) = captured {
            self.raw_add_piece(to, cap.piece_type, cap.color);
            self.zobrist_hash ^= keys.get_piece_key(cap.piece_type, cap.color, to_row, to_col);
        }
    }

    #[inline]
    pub fn update_nnue_for_move(
        &mut self,
        from_sq: usize,
        to_sq: usize,
        piece: Piece,
        captured: Option<Piece>,
    ) {
        self.update_nnue_for_move_with_weights(from_sq, to_sq, piece, captured, NnueWeights::get());
    }

    #[inline]
    pub fn update_nnue_for_move_with_weights(
        &mut self,
        from_sq: usize,
        to_sq: usize,
        piece: Piece,
        captured: Option<Piece>,
        weights: &NnueWeights,
    ) {
        let k_red = crate::engine::nnue::get_red_king_palace_idx(self);
        let k_black = crate::engine::nnue::get_black_king_palace_idx(self);
        let is_general = piece.piece_type == PieceType::General;

        if is_general {
            if piece.color == Color::Red {
                let new_k_red = crate::engine::nnue::palace_index(to_sq);
                self.nnue_acc.refresh_red(&self.grid, new_k_red, weights);
                if let Some(cap) = captured {
                    self.nnue_acc
                        .move_piece_capture_black(piece, from_sq, to_sq, cap, k_black, weights);
                } else {
                    self.nnue_acc
                        .move_piece_quiet_black(piece, from_sq, to_sq, k_black, weights);
                }
            } else {
                let flipped_to = 89_usize.saturating_sub(to_sq);
                let new_k_black = crate::engine::nnue::palace_index(flipped_to);
                self.nnue_acc
                    .refresh_black(&self.grid, new_k_black, weights);
                if let Some(cap) = captured {
                    self.nnue_acc
                        .move_piece_capture_red(piece, from_sq, to_sq, cap, k_red, weights);
                } else {
                    self.nnue_acc
                        .move_piece_quiet_red(piece, from_sq, to_sq, k_red, weights);
                }
            }
        } else if let Some(cap) = captured {
            self.nnue_acc
                .move_piece_capture_red(piece, from_sq, to_sq, cap, k_red, weights);
            self.nnue_acc
                .move_piece_capture_black(piece, from_sq, to_sq, cap, k_black, weights);
        } else {
            self.nnue_acc
                .move_piece_quiet_red(piece, from_sq, to_sq, k_red, weights);
            self.nnue_acc
                .move_piece_quiet_black(piece, from_sq, to_sq, k_black, weights);
        }
    }

    pub fn apply_move(&mut self, mv: &Move, _turn: Color) {
        if let Some((piece, captured, from_sq, to_sq)) = self.apply_move_board_only(mv) {
            self.update_nnue_for_move(from_sq, to_sq, piece, captured);
        }
    }

    pub fn undo_move(&mut self, mv: &Move, captured: Option<Piece>, _turn: Color) {
        let keys = ZobristKeys::get();
        let from_row = mv.from_row as usize;
        let from_col = mv.from_col as usize;
        let to_row = mv.to_row as usize;
        let to_col = mv.to_col as usize;

        let Some(from) = BoardCoordinate::new(from_row, from_col) else {
            return;
        };
        let Some(to) = BoardCoordinate::new(to_row, to_col) else {
            return;
        };

        // 4. Switch turn hash (back)
        self.zobrist_hash ^= keys.side_key;

        // 3. Move piece back from destination to source
        let Some(piece) = self.get_piece(to) else {
            return;
        };

        let weights = NnueWeights::get();
        let k_red = crate::engine::nnue::get_red_king_palace_idx(self);
        let k_black = crate::engine::nnue::get_black_king_palace_idx(self);
        let is_general = piece.piece_type == PieceType::General;

        // Remove from destination
        self.raw_remove_piece(to, piece.piece_type, piece.color);
        self.zobrist_hash ^= keys.get_piece_key(piece.piece_type, piece.color, to_row, to_col);

        // Place back at source
        self.raw_add_piece(from, piece.piece_type, piece.color);
        self.zobrist_hash ^= keys.get_piece_key(piece.piece_type, piece.color, from_row, from_col);

        // 2. Restore captured piece (if any)
        if let Some(cap) = captured {
            self.raw_add_piece(to, cap.piece_type, cap.color);
            self.zobrist_hash ^= keys.get_piece_key(cap.piece_type, cap.color, to_row, to_col);
        }

        // 1. Update NNUE accumulator accurately
        if is_general {
            if piece.color == Color::Red {
                let new_k_red = crate::engine::nnue::palace_index(from.index());
                self.nnue_acc.refresh_red(&self.grid, new_k_red, weights);
                self.nnue_acc
                    .remove_piece_black(piece, to.index(), k_black, weights);
                self.nnue_acc
                    .add_piece_black(piece, from.index(), k_black, weights);
                if let Some(cap) = captured {
                    self.nnue_acc
                        .add_piece_black(cap, to.index(), k_black, weights);
                }
            } else {
                let flipped_from = 89_usize.saturating_sub(from.index());
                let new_k_black = crate::engine::nnue::palace_index(flipped_from);
                self.nnue_acc
                    .refresh_black(&self.grid, new_k_black, weights);
                self.nnue_acc
                    .remove_piece_red(piece, to.index(), k_red, weights);
                self.nnue_acc
                    .add_piece_red(piece, from.index(), k_red, weights);
                if let Some(cap) = captured {
                    self.nnue_acc.add_piece_red(cap, to.index(), k_red, weights);
                }
            }
        } else {
            self.nnue_acc
                .remove_piece(piece, to.index(), k_red, k_black, weights);
            self.nnue_acc
                .add_piece(piece, from.index(), k_red, k_black, weights);
            if let Some(cap) = captured {
                self.nnue_acc
                    .add_piece(cap, to.index(), k_red, k_black, weights);
            }
        }
    }

    #[inline]
    pub fn refresh_nnue(&mut self) {
        self.refresh_nnue_with_weights(NnueWeights::get());
    }

    #[inline]
    pub fn refresh_nnue_with_weights(&mut self, weights: &NnueWeights) {
        let k_red = crate::engine::nnue::get_red_king_palace_idx(self);
        let k_black = crate::engine::nnue::get_black_king_palace_idx(self);
        self.nnue_acc.refresh_red(&self.grid, k_red, weights);
        self.nnue_acc.refresh_black(&self.grid, k_black, weights);
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
