#![allow(clippy::indexing_slicing)]
use crate::engine::zobrist::ZobristKeys;
use crate::engine::Move;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Red,
    Black,
}

impl Color {
    #[must_use]
    pub fn opposite(self) -> Self {
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
}

impl Board {
    pub fn to_fen_string(&self, turn: Color) -> String {
        let mut fen = String::new();
        // 1. Piece placement
        // Iterate from rank 9 (top) to 0 (bottom)
        for r in (0..10).rev() {
            let mut empty_count = 0;
            for c in 0..9 {
                if let Some(piece) = self.grid[r][c] {
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
        };
        board.setup_initial_position();
        board.zobrist_hash = board.calculate_initial_hash();
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
            self.grid[back_row][col] = Some(Piece {
                piece_type: pt,
                color,
            });
        }

        // Cannons
        self.grid[cannon_row][1] = Some(Piece {
            piece_type: PieceType::Cannon,
            color,
        });
        self.grid[cannon_row][7] = Some(Piece {
            piece_type: PieceType::Cannon,
            color,
        });

        // Soldiers
        for col in (0..9).step_by(2) {
            self.grid[soldier_row][col] = Some(Piece {
                piece_type: PieceType::Soldier,
                color,
            });
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
        let keys = ZobristKeys::new();
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

    pub fn apply_move(&mut self, mv: &Move, _turn: Color) {
        let keys = ZobristKeys::new(); // In a real engine, we'd pass this in or have it static.
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
            }

            // 3. Place piece at destination
            self.zobrist_hash ^=
                keys.get_piece_key(piece.piece_type, piece.color, mv.to_row, mv.to_col);

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
