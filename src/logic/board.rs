#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Red,
    Black,
}

impl Color {
    #[must_use]
    pub fn opposite(&self) -> Self {
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
pub struct Board {
    // 10 rows (0..9), 9 columns (0..8)
    // (0,0) is bottom-left from Red's perspective (if Red is at bottom)
    pub grid: [[Option<Piece>; 9]; 10],
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
            grid: [[None; 9]; 10],
        };
        board.setup_initial_position();
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
        if row >= 10 || col >= 9 {
            return None;
        }
        self.grid[row][col]
    }
}
