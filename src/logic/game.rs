#![allow(clippy::indexing_slicing)]
use crate::engine::Move;
use crate::logic::board::{Board, Color};
use crate::logic::rules::{is_in_check, is_valid_move, MoveError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameStatus {
    Playing,
    Checkmate(Color), // Winner
    Stalemate,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MoveRecord {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub piece: crate::logic::board::Piece,
    pub captured: Option<crate::logic::board::Piece>,
    pub color: Color,
    pub note: Option<String>, // For AI stats or other info
    pub hash: u64,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub board: Board,
    pub turn: Color,
    pub status: GameStatus,
    pub last_move: Option<((usize, usize), (usize, usize))>,
    pub history: Vec<MoveRecord>,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            board: Board::new(),
            turn: Color::Red,
            status: GameStatus::Playing,
            last_move: None,
            history: Vec::new(),
        }
    }

    pub fn make_move(
        &mut self,
        from_row: usize,
        from_col: usize,
        to_row: usize,
        to_col: usize,
    ) -> Result<(), MoveError> {
        if self.status != GameStatus::Playing {
            return Err(MoveError::NotYourTurn);
        }

        is_valid_move(&self.board, from_row, from_col, to_row, to_col, self.turn)?;

        // Execute move
        let mut next_board = self.board.clone();

        // Construct Move for apply_move
        let mv = Move {
            from_row,
            from_col,
            to_row,
            to_col,
            score: 0,
        };

        // Capture piece info before apply_move
        let piece = next_board
            .get_piece(from_row, from_col)
            .ok_or(MoveError::NoPieceAtSource)?;
        let captured = next_board.get_piece(to_row, to_col);

        next_board.apply_move(&mv, self.turn);

        // 3-Fold Repetition Check
        let initial_hash = Board::new().zobrist_hash;
        let mut count = 0;

        // Check history
        count += self
            .history
            .iter()
            .filter(|r| r.hash == next_board.zobrist_hash)
            .count();

        // Check initial state
        if next_board.zobrist_hash == initial_hash {
            count += 1;
        }

        // If we already have 2 occurrences (so this would be the 3rd), forbid it.
        if count >= 2 {
            return Err(MoveError::ThreeFoldRepetition);
        }

        self.board = next_board;
        self.history.push(MoveRecord {
            from: (from_row, from_col),
            to: (to_row, to_col),
            piece,
            captured,
            color: self.turn,
            note: None,
            hash: self.board.zobrist_hash,
        });

        self.turn = self.turn.opposite();
        self.last_move = Some(((from_row, from_col), (to_row, to_col)));

        self.update_status();

        Ok(())
    }

    fn update_status(&mut self) {
        let current_turn = self.turn;

        // Check if current player has any valid moves
        let has_moves = self.has_any_valid_move(current_turn);
        let in_check = is_in_check(&self.board, current_turn);

        if !has_moves {
            if in_check {
                self.status = GameStatus::Checkmate(current_turn.opposite());
            } else {
                self.status = GameStatus::Stalemate;
            }
        }
    }

    fn has_any_valid_move(&self, color: Color) -> bool {
        for r in 0..10 {
            for c in 0..9 {
                if let Some(p) = self.board.get_piece(r, c) {
                    if p.color == color {
                        // Try all possible moves for this piece
                        // Optimization: We can be smarter, but brute force is fine for 90 squares
                        for tr in 0..10 {
                            for tc in 0..9 {
                                if is_valid_move(&self.board, r, c, tr, tc, color).is_ok() {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }
}
