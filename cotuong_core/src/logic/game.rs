use crate::engine::Move;
use crate::logic::board::{Board, Color};
use crate::logic::rules::{is_in_check, is_valid_move, MoveError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameStatus {
    Playing,
    Checkmate(Color), // Winner
    Stalemate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            from_row: from_row as u8,
            from_col: from_col as u8,
            to_row: to_row as u8,
            to_col: to_col as u8,
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
            // Exception: If this is the ONLY legal move, allow it.
            if self.has_more_than_one_valid_move(self.turn) {
                return Err(MoveError::ThreeFoldRepetition);
            }
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

    fn has_more_than_one_valid_move(&self, color: Color) -> bool {
        let mut count = 0;
        for r in 0..10 {
            for c in 0..9 {
                if let Some(p) = self.board.get_piece(r, c) {
                    if p.color == color {
                        for tr in 0..10 {
                            for tc in 0..9 {
                                if is_valid_move(&self.board, r, c, tr, tc, color).is_ok() {
                                    count += 1;
                                    if count > 1 {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    pub fn undo_move(&mut self) -> bool {
        if let Some(record) = self.history.pop() {
            let mv = Move {
                from_row: record.from.0 as u8,
                from_col: record.from.1 as u8,
                to_row: record.to.0 as u8,
                to_col: record.to.1 as u8,
                score: 0,
            };

            self.board
                .undo_move(&mv, record.captured, self.turn.opposite());
            self.turn = self.turn.opposite();

            // Restore last_move from the previous record in history, if any
            if let Some(prev) = self.history.last() {
                self.last_move = Some((prev.from, prev.to));
            } else {
                self.last_move = None;
            }

            // Reset status to Playing since we undid a move (even if it was checkmate)
            self.status = GameStatus::Playing;

            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::board::PieceType;

    #[test]
    fn test_undo_move() {
        let mut game = GameState::new();
        let initial_fen = game.board.to_fen_string(game.turn);

        // Make a move: Red Central Soldier forward
        // From (3, 4) to (4, 4)
        game.make_move(3, 4, 4, 4).unwrap();

        assert_eq!(game.history.len(), 1);
        assert_eq!(game.turn, Color::Black);
        assert!(game.board.get_piece(3, 4).is_none());
        assert!(game.board.get_piece(4, 4).is_some());

        // Undo
        let success = game.undo_move();
        assert!(success);

        assert_eq!(game.history.len(), 0);
        assert_eq!(game.turn, Color::Red);
        assert!(game.board.get_piece(3, 4).is_some());
        assert!(game.board.get_piece(4, 4).is_none());

        let restored_fen = game.board.to_fen_string(game.turn);
        assert_eq!(initial_fen, restored_fen);
    }

    #[test]
    fn test_undo_capture() {
        let mut game = GameState::new();

        // 1. Red Soldier (3,4) -> (4,4)
        game.make_move(3, 4, 4, 4).unwrap();
        // 2. Black Soldier (6,4) -> (5,4)
        game.make_move(6, 4, 5, 4).unwrap();
        // 3. Red Soldier (4,4) -> (5,4) Capture!
        game.make_move(4, 4, 5, 4).unwrap();

        assert_eq!(game.history.len(), 3);
        let last_record = game.history.last().unwrap();
        assert!(last_record.captured.is_some());
        assert_eq!(last_record.captured.unwrap().piece_type, PieceType::Soldier);

        // Undo Capture
        let success = game.undo_move();
        assert!(success);

        assert_eq!(game.history.len(), 2);
        assert_eq!(game.turn, Color::Red);
        // Check Red Soldier back at (4,4)
        let p = game.board.get_piece(4, 4).unwrap();
        assert_eq!(p.piece_type, PieceType::Soldier);
        assert_eq!(p.color, Color::Red);

        // Check Black Soldier restored at (5,4)
        let cap = game.board.get_piece(5, 4).unwrap();
        assert_eq!(cap.piece_type, PieceType::Soldier);
        assert_eq!(cap.color, Color::Black);
    }
}
