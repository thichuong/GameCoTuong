use crate::engine::zobrist::ZobristKeys;
use crate::engine::Move;
use crate::logic::eval_constants::{get_piece_value, get_pst_value};

pub type Bitboard = u128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub piece_type: PieceType,
    pub color: Color,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    // Bitboards for each piece type and color
    // Index: color.index() * 7 + piece_type.index()
    pub bitboards: [Bitboard; 14],
    pub occupied: Bitboard,
    // Mailbox for O(1) lookup
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
                if let Some(piece) = self.get_piece(r, c) {
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
            self.add_piece(back_row, col, pt, color);
        }

        // Cannons
        self.add_piece(cannon_row, 1, PieceType::Cannon, color);
        self.add_piece(cannon_row, 7, PieceType::Cannon, color);

        // Soldiers
        for col in (0..9).step_by(2) {
            self.add_piece(soldier_row, col, PieceType::Soldier, color);
        }
    }

    pub fn move_piece_quiet(
        &mut self,
        from_row: usize,
        from_col: usize,
        to_row: usize,
        to_col: usize,
    ) -> Option<Piece> {
        let piece = self.get_piece(from_row, from_col)?;
        self.remove_piece(from_row, from_col, piece.piece_type, piece.color);
        let captured = self.get_piece(to_row, to_col);
        if let Some(cap) = captured {
            self.remove_piece(to_row, to_col, cap.piece_type, cap.color);
        }
        self.add_piece(to_row, to_col, piece.piece_type, piece.color);
        captured
    }

    pub fn undo_move_quiet(
        &mut self,
        from_row: usize,
        from_col: usize,
        to_row: usize,
        to_col: usize,
        captured: Option<Piece>,
    ) {
        let piece = self
            .get_piece(to_row, to_col)
            .expect("No piece at destination in undo_move_quiet");
        self.remove_piece(to_row, to_col, piece.piece_type, piece.color);
        self.add_piece(from_row, from_col, piece.piece_type, piece.color);
        if let Some(cap) = captured {
            self.add_piece(to_row, to_col, cap.piece_type, cap.color);
        }
    }

    pub fn set_piece(&mut self, row: usize, col: usize, piece: Option<Piece>) {
        // Remove existing
        if let Some(p) = self.get_piece(row, col) {
            self.remove_piece(row, col, p.piece_type, p.color);
        }
        // Add new
        if let Some(p) = piece {
            self.add_piece(row, col, p.piece_type, p.color);
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
    pub fn add_piece(&mut self, row: usize, col: usize, piece_type: PieceType, color: Color) {
        let sq = Self::square_index(row, col);
        let bit = 1u128 << sq;
        let idx = color.index() * 7 + piece_type.index();
        self.bitboards[idx] |= bit;
        self.occupied |= bit;
        self.grid[sq] = Some(Piece { piece_type, color });

        self.occupied_rows[row] |= 1 << col;
        self.occupied_cols[col] |= 1 << row;
    }

    // Helper to remove a piece
    fn remove_piece(&mut self, row: usize, col: usize, piece_type: PieceType, color: Color) {
        let sq = Self::square_index(row, col);
        let bit = 1u128 << sq;
        let idx = color.index() * 7 + piece_type.index();
        self.bitboards[idx] &= !bit;
        self.occupied &= !bit;
        self.grid[sq] = None;

        self.occupied_rows[row] &= !(1 << col);
        self.occupied_cols[col] &= !(1 << row);
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
    pub fn get_piece(&self, row: usize, col: usize) -> Option<Piece> {
        let sq = Self::square_index(row, col);
        self.grid[sq]
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
                if let Some(piece) = self.get_piece(r, c) {
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
                if let Some(piece) = self.get_piece(r, c) {
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

        // 1. Get piece at source
        let piece = self
            .get_piece(from_row, from_col)
            .expect("No piece at source in apply_move");

        // Remove from source
        self.remove_piece(from_row, from_col, piece.piece_type, piece.color);
        self.zobrist_hash ^= keys.get_piece_key(piece.piece_type, piece.color, from_row, from_col);

        // Update Score (Remove from source)
        let pst_from = get_pst_value(piece.piece_type, piece.color, from_row, from_col);
        if piece.color == Color::Red {
            self.red_pst -= pst_from;
        } else {
            self.black_pst -= pst_from;
        }

        // 2. Remove captured piece (if any)
        if let Some(captured) = self.get_piece(to_row, to_col) {
            self.remove_piece(to_row, to_col, captured.piece_type, captured.color);
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
        self.add_piece(to_row, to_col, piece.piece_type, piece.color);
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

        // 4. Switch turn hash (back)
        self.zobrist_hash ^= keys.side_key;

        // 3. Move piece back from destination to source
        let piece = self
            .get_piece(to_row, to_col)
            .expect("No piece at destination in undo_move");

        // Remove from destination
        self.remove_piece(to_row, to_col, piece.piece_type, piece.color);
        self.zobrist_hash ^= keys.get_piece_key(piece.piece_type, piece.color, to_row, to_col);

        // Update Score (Remove from dest)
        let pst_to = get_pst_value(piece.piece_type, piece.color, to_row, to_col);
        if piece.color == Color::Red {
            self.red_pst -= pst_to;
        } else {
            self.black_pst -= pst_to;
        }

        // Place back at source
        self.add_piece(from_row, from_col, piece.piece_type, piece.color);
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
            self.add_piece(to_row, to_col, cap.piece_type, cap.color);
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
        let piece = board.get_piece(0, 4).unwrap();
        assert_eq!(piece.piece_type, PieceType::General);
        assert_eq!(piece.color, Color::Red);

        // Check Black General
        let piece = board.get_piece(9, 4).unwrap();
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

        assert!(board.get_piece(3, 4).is_none());
        let piece = board.get_piece(4, 4).unwrap();
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
