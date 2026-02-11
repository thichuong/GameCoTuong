use crate::engine::Move;
use crate::logic::board::{Board, BoardCoordinate, Color};
use crate::logic::generator::MoveGenerator;
use crate::logic::rules::{is_valid_move, MoveError};
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
    pub from: BoardCoordinate,
    pub to: BoardCoordinate,
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
    pub last_move: Option<(BoardCoordinate, BoardCoordinate)>,
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
        from: BoardCoordinate,
        to: BoardCoordinate,
    ) -> Result<(), MoveError> {
        if self.status != GameStatus::Playing {
            return Err(MoveError::NotYourTurn);
        }

        is_valid_move(&self.board, from, to, self.turn)?;

        // Execute move
        let mut next_board = self.board.clone();

        // Construct Move for apply_move
        let mv = Move {
            from_row: from.row as u8,
            from_col: from.col as u8,
            to_row: to.row as u8,
            to_col: to.col as u8,
            score: 0,
        };

        // Capture piece info before apply_move
        let piece = next_board
            .get_piece(from)
            .ok_or(MoveError::NoPieceAtSource)?;
        let captured = next_board.get_piece(to);

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
            from,
            to,
            piece,
            captured,
            color: self.turn,
            note: None,
            hash: self.board.zobrist_hash,
        });

        self.turn = self.turn.opposite();
        self.last_move = Some((from, to));

        self.update_status();

        Ok(())
    }

    fn update_status(&mut self) {
        let current_turn = self.turn;

        // Check if current player has any valid moves
        let has_moves = self.has_any_valid_move(current_turn);

        if !has_moves {
            // In Chinese Chess (and this variant), if you have no moves, you lose.
            // This applies whether you are in check or not (Stalemate is a loss).
            self.status = GameStatus::Checkmate(current_turn.opposite());
        }
    }

    fn has_any_valid_move(&self, color: Color) -> bool {
        let generator = MoveGenerator::new();
        generator.has_legal_moves(&self.board, color)
    }

    fn has_more_than_one_valid_move(&self, color: Color) -> bool {
        let mut count = 0;
        let bb = self.board.get_color_bb(color);

        for sq in crate::logic::board::BitboardIterator::new(bb) {
            let (r, c) = Board::index_to_coord(sq);
            let pos = unsafe { BoardCoordinate::new_unchecked(r, c) };
            if let Some(piece) = self.board.get_piece(pos) {
                count += self.count_valid_moves_capped(pos, piece.piece_type, color, 2 - count);
                if count > 1 {
                    return true;
                }
            }
        }
        false
    }


    fn count_valid_moves_capped(
        &self,
        from: BoardCoordinate,
        piece_type: crate::logic::board::PieceType,
        color: Color,
        cap: i32,
    ) -> i32 {
        use crate::logic::board::PieceType;
        if cap <= 0 {
            return 0;
        }

        match piece_type {
            PieceType::General => {
                self.count_offsets_capped(from, color, &[(0, 1), (0, -1), (1, 0), (-1, 0)], cap)
            }
            PieceType::Advisor => {
                self.count_offsets_capped(from, color, &[(1, 1), (1, -1), (-1, 1), (-1, -1)], cap)
            }
            PieceType::Elephant => {
                self.count_offsets_capped(from, color, &[(2, 2), (2, -2), (-2, 2), (-2, -2)], cap)
            }
            PieceType::Horse => self.count_offsets_capped(
                from,
                color,
                &[
                    (2, 1),
                    (2, -1),
                    (-2, 1),
                    (-2, -1),
                    (1, 2),
                    (1, -2),
                    (-1, 2),
                    (-1, -2),
                ],
                cap,
            ),
            PieceType::Chariot => {
                self.count_linear_capped(from, color, &[(0, 1), (0, -1), (1, 0), (-1, 0)], cap)
            }
            PieceType::Cannon => self.count_linear_cannon_capped(
                from,
                color,
                &[(0, 1), (0, -1), (1, 0), (-1, 0)],
                cap,
            ),
            PieceType::Soldier => {
                let forward = match color {
                    Color::Red => 1,
                    Color::Black => -1,
                };
                self.count_offsets_capped(from, color, &[(forward, 0), (0, 1), (0, -1)], cap)
            }
        }
    }


    fn count_offsets_capped(
        &self,
        from: BoardCoordinate,
        color: Color,
        offsets: &[(isize, isize)],
        cap: i32,
    ) -> i32 {
        let mut count = 0;
        for &(dr, dc) in offsets {
            let r = from.row as isize + dr;
            let c = from.col as isize + dc;
            if r >= 0 && r < 10 && c >= 0 && c < 9 {
                let to = unsafe { BoardCoordinate::new_unchecked(r as usize, c as usize) };
                if is_valid_move(&self.board, from, to, color).is_ok() {
                    count += 1;
                    if count >= cap {
                        return count;
                    }
                }
            }
        }
        count
    }

    fn count_linear_capped(
        &self,
        from: BoardCoordinate,
        color: Color,
        dirs: &[(isize, isize)],
        cap: i32,
    ) -> i32 {
        let mut count = 0;
        for &(dr, dc) in dirs {
            let mut r = from.row as isize + dr;
            let mut c = from.col as isize + dc;
            while r >= 0 && r < 10 && c >= 0 && c < 9 {
                let to = unsafe { BoardCoordinate::new_unchecked(r as usize, c as usize) };
                if is_valid_move(&self.board, from, to, color).is_ok() {
                    count += 1;
                    if count >= cap {
                        return count;
                    }
                }
                if self.board.get_piece(to).is_some() {
                    break;
                }
                r += dr;
                c += dc;
            }
        }
        count
    }

    fn count_linear_cannon_capped(
        &self,
        from: BoardCoordinate,
        color: Color,
        dirs: &[(isize, isize)],
        cap: i32,
    ) -> i32 {
        let mut count = 0;
        for &(dr, dc) in dirs {
            let mut r = from.row as isize + dr;
            let mut c = from.col as isize + dc;
            while r >= 0 && r < 10 && c >= 0 && c < 9 {
                let to = unsafe { BoardCoordinate::new_unchecked(r as usize, c as usize) };
                if is_valid_move(&self.board, from, to, color).is_ok() {
                    count += 1;
                    if count >= cap {
                        return count;
                    }
                }
                r += dr;
                c += dc;
            }
        }
        count
    }

    pub fn undo_move(&mut self) -> bool {
        if let Some(record) = self.history.pop() {
            let mv = Move {
                from_row: record.from.row as u8,
                from_col: record.from.col as u8,
                to_row: record.to.row as u8,
                to_col: record.to.col as u8,
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
        game.make_move(
            BoardCoordinate::new(3, 4).unwrap(),
            BoardCoordinate::new(4, 4).unwrap(),
        )
        .unwrap();

        assert_eq!(game.history.len(), 1);
        assert_eq!(game.turn, Color::Black);
        assert!(game
            .board
            .get_piece(BoardCoordinate::new(3, 4).unwrap())
            .is_none());
        assert!(game
            .board
            .get_piece(BoardCoordinate::new(4, 4).unwrap())
            .is_some());

        // Undo
        let success = game.undo_move();
        assert!(success);

        assert_eq!(game.history.len(), 0);
        assert_eq!(game.turn, Color::Red);
        assert!(game
            .board
            .get_piece(BoardCoordinate::new(3, 4).unwrap())
            .is_some());
        assert!(game
            .board
            .get_piece(BoardCoordinate::new(4, 4).unwrap())
            .is_none());

        let restored_fen = game.board.to_fen_string(game.turn);
        assert_eq!(initial_fen, restored_fen);
    }

    #[test]
    fn test_undo_capture() {
        let mut game = GameState::new();

        // 1. Red Soldier (3,4) -> (4,4)
        game.make_move(
            BoardCoordinate::new(3, 4).unwrap(),
            BoardCoordinate::new(4, 4).unwrap(),
        )
        .unwrap();
        // 2. Black Soldier (6,4) -> (5,4)
        game.make_move(
            BoardCoordinate::new(6, 4).unwrap(),
            BoardCoordinate::new(5, 4).unwrap(),
        )
        .unwrap();
        // 3. Red Soldier (4,4) -> (5,4) Capture!
        game.make_move(
            BoardCoordinate::new(4, 4).unwrap(),
            BoardCoordinate::new(5, 4).unwrap(),
        )
        .unwrap();

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
        let p = game
            .board
            .get_piece(BoardCoordinate::new(4, 4).unwrap())
            .unwrap();
        assert_eq!(p.piece_type, PieceType::Soldier);
        assert_eq!(p.color, Color::Red);

        // Check Black Soldier restored at (5,4)
        let cap = game
            .board
            .get_piece(BoardCoordinate::new(5, 4).unwrap())
            .unwrap();
        assert_eq!(cap.piece_type, PieceType::Soldier);
        assert_eq!(cap.color, Color::Black);
    }

    #[test]
    fn test_stalemate_is_loss() {
        let mut game = GameState::new();
        game.board.clear();

        // Setup Stalemate position
        // Red General at (0,0)
        game.board.add_piece(
            BoardCoordinate::new(0, 0).unwrap(),
            PieceType::General,
            Color::Red,
        );

        // Black General at (9,4) (Safe)
        game.board.add_piece(
            BoardCoordinate::new(9, 4).unwrap(),
            PieceType::General,
            Color::Black,
        );

        // Black Chariot at (9,1).
        // We will move it to (1,1) to trap Red.
        game.board.add_piece(
            BoardCoordinate::new(9, 1).unwrap(),
            PieceType::Chariot,
            Color::Black,
        );

        // It's Black's turn to make the trapping move
        game.turn = Color::Black;

        // Move Black Chariot (9,1) -> (1,1)
        // This covers row 1 and col 1.
        // Red General at (0,0) has moves: (0,1) and (1,0).
        // (0,1) is attacked by Chariot (vertical).
        // (1,0) is attacked by Chariot (horizontal).
        // (0,0) is NOT attacked (Chariot is at (1,1)).
        // So Red is NOT in check, but has NO moves. -> Stalemate.

        let result = game.make_move(
            BoardCoordinate::new(9, 1).unwrap(),
            BoardCoordinate::new(1, 1).unwrap(),
        );
        assert!(result.is_ok());

        // Expect Checkmate (Black Wins) because Stalemate is Loss
        match game.status {
            GameStatus::Checkmate(winner) => {
                assert_eq!(winner, Color::Black);
            }
            status => {
                panic!("Expected Checkmate(Black), got {:?}", status);
            }
        }
    }
}
