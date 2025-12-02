use crate::engine::zobrist::ZobristKeys;
use crate::engine::Move;
use crate::logic::eval_constants::{get_piece_value, get_pst_value};

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceType {
    General,  // King/General
    Advisor,  // Guard
    Elephant, // Bishop/Elephant
    Horse,    // Knight/Horse
    Chariot,  // Rook/Chariot
    Cannon,
    Soldier, // Pawn/Soldier
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub piece_type: PieceType,
    pub color: Color,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::indexing_slicing)]
pub struct Board {
    // 10 rows (0..9), 9 columns (0..8)
    // (0,0) is bottom-left from Red's perspective (if Red is at bottom)
    pub grid: [[Option<Piece>; 9]; 10],
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
                if let Some(piece) = self
                    .grid
                    .get(r)
                    .and_then(|row| row.get(c))
                    .copied()
                    .flatten()
                {
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
        fen.push(if turn == Color::Red { 'w' } else { 'b' }); // 'w' for Red (White equivalent), 'b' for Black

        fen
    }
}

impl Board {
    #[must_use]
    pub fn new() -> Self {
        let mut board = Self {
            grid: [[None; 9]; 10],
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
            if let Some(cell) = self.grid.get_mut(back_row).and_then(|row| row.get_mut(col)) {
                *cell = Some(Piece {
                    piece_type: pt,
                    color,
                });
            }
        }

        // Cannons
        if let Some(cell) = self.grid.get_mut(cannon_row).and_then(|row| row.get_mut(1)) {
            *cell = Some(Piece {
                piece_type: PieceType::Cannon,
                color,
            });
        }
        if let Some(cell) = self.grid.get_mut(cannon_row).and_then(|row| row.get_mut(7)) {
            *cell = Some(Piece {
                piece_type: PieceType::Cannon,
                color,
            });
        }

        // Soldiers
        for col in (0..9).step_by(2) {
            if let Some(cell) = self
                .grid
                .get_mut(soldier_row)
                .and_then(|row| row.get_mut(col))
            {
                *cell = Some(Piece {
                    piece_type: PieceType::Soldier,
                    color,
                });
            }
        }
    }

    #[must_use]
    pub fn get_piece(&self, row: usize, col: usize) -> Option<Piece> {
        self.grid
            .get(row)
            .and_then(|r| r.get(col))
            .copied()
            .flatten()
    }

    pub fn calculate_initial_hash(&self) -> u64 {
        let keys = ZobristKeys::get();
        let mut hash = 0;
        for r in 0..10 {
            for c in 0..9 {
                if let Some(piece) = self
                    .grid
                    .get(r)
                    .and_then(|row| row.get(c))
                    .copied()
                    .flatten()
                {
                    hash ^= keys.get_piece_key(piece.piece_type, piece.color, r, c);
                }
            }
        }
        // We assume Red starts, so we don't XOR side_key initially if Red is 0 and side_key is for Black?
        // Actually, usually we XOR side_key if it's Black's turn.
        // Let's assume Red starts and hash starts without side_key.
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
        let keys = ZobristKeys::get(); // Use global static instance
                                       // Since we made it cheap to create (just constants/XorShift), it's okay-ish.
                                       // But ideally we should have a static instance.
                                       // For now, let's just create it. It's fast enough.

        // 1. Remove piece from source
        if let Some(piece) = self
            .grid
            .get(mv.from_row)
            .and_then(|r| r.get(mv.from_col))
            .copied()
            .flatten()
        {
            self.zobrist_hash ^=
                keys.get_piece_key(piece.piece_type, piece.color, mv.from_row, mv.from_col);

            // Update Score (Remove from source)
            let pst_from = get_pst_value(piece.piece_type, piece.color, mv.from_row, mv.from_col);

            if piece.color == Color::Red {
                self.red_pst -= pst_from;
            } else {
                self.black_pst -= pst_from;
            }

            // 2. Remove captured piece (if any)
            if let Some(captured) = self
                .grid
                .get(mv.to_row)
                .and_then(|r| r.get(mv.to_col))
                .copied()
                .flatten()
            {
                self.zobrist_hash ^=
                    keys.get_piece_key(captured.piece_type, captured.color, mv.to_row, mv.to_col);

                // Update Score (Remove captured)
                let cap_val = get_piece_value(captured.piece_type);
                let cap_pst =
                    get_pst_value(captured.piece_type, captured.color, mv.to_row, mv.to_col);

                if captured.color == Color::Red {
                    self.red_material -= cap_val;
                    self.red_pst -= cap_pst;
                } else {
                    self.black_material -= cap_val;
                    self.black_pst -= cap_pst;
                }
            }

            // 3. Place piece at destination
            self.zobrist_hash ^=
                keys.get_piece_key(piece.piece_type, piece.color, mv.to_row, mv.to_col);

            // Update Score (Add to dest)
            let pst_to = get_pst_value(piece.piece_type, piece.color, mv.to_row, mv.to_col);
            if piece.color == Color::Red {
                self.red_pst += pst_to;
            } else {
                self.black_pst += pst_to;
            }

            // Update grid
            // Move piece
            if let Some(row) = self.grid.get_mut(mv.to_row) {
                if let Some(cell) = row.get_mut(mv.to_col) {
                    *cell = Some(piece);
                }
            }
            if let Some(row) = self.grid.get_mut(mv.from_row) {
                if let Some(cell) = row.get_mut(mv.from_col) {
                    *cell = None;
                }
            }
        }

        // 4. Switch turn hash
        self.zobrist_hash ^= keys.side_key;
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
