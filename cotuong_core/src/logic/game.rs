use crate::engine::Move;
use crate::logic::board::{Board, BoardCoordinate, Color};
use crate::logic::generator::MoveGenerator;
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
    pub from: BoardCoordinate,
    pub to: BoardCoordinate,
    pub piece: crate::logic::board::Piece,
    pub captured: Option<crate::logic::board::Piece>,
    pub color: Color,
    pub note: Option<String>, // For AI stats or other info
    pub hash: u64,
    #[serde(default)]
    pub is_check: bool,
}

fn default_initial_hash() -> u64 {
    Board::new().zobrist_hash
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub board: Board,
    pub turn: Color,
    pub status: GameStatus,
    pub last_move: Option<(BoardCoordinate, BoardCoordinate)>,
    pub history: Vec<MoveRecord>,
    #[serde(default = "default_initial_hash")]
    pub initial_hash: u64,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    #[must_use]
    pub fn new() -> Self {
        let board = Board::new();
        let initial_hash = board.zobrist_hash;
        Self {
            board,
            turn: Color::Red,
            status: GameStatus::Playing,
            last_move: None,
            history: Vec::new(),
            initial_hash,
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

        if !self.is_move_legal(from, to) {
            return Err(MoveError::ThreeFoldRepetition);
        }

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

        let is_check = is_in_check(&next_board, self.turn.opposite());

        self.board = next_board;
        self.history.push(MoveRecord {
            from,
            to,
            piece,
            captured,
            color: self.turn,
            note: None,
            hash: self.board.zobrist_hash,
            is_check,
        });

        self.turn = self.turn.opposite();
        self.last_move = Some((from, to));

        self.update_status();

        Ok(())
    }

    fn update_status(&mut self) {
        // Stalemate is a loss in Chinese Chess. If no strictly legal moves remain, the player loses.
        if self.get_strictly_legal_moves().is_empty() {
            self.status = GameStatus::Checkmate(self.turn.opposite());
        }
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

    /// Count how many times the current player has consecutively repeated this exact move back and forth.
    /// In Xiangqi: a player is allowed to repeat a move at most 3 times.
    /// If it has already been executed 3 times, attempting it a 4th time is forbidden.
    #[must_use]
    pub fn count_move_back_and_forth(&self, from: BoardCoordinate, to: BoardCoordinate) -> usize {
        let player_moves: Vec<&MoveRecord> = self
            .history
            .iter()
            .filter(|r| r.color == self.turn)
            .collect();

        let mut count = 0;
        let mut i = player_moves.len();

        while i >= 2 {
            let last = player_moves[i - 1];
            let prev = player_moves[i - 2];
            if last.from == to && last.to == from && prev.from == from && prev.to == to {
                count += 1;
                i -= 2;
            } else {
                break;
            }
        }

        count
    }

    /// Check whether a resulting board hash and check status violates repetition rules.
    /// In Xiangqi:
    /// - Cho phép lặp lại tối đa 3 lần: thế cờ được xuất hiện tối đa 3 lần. Nếu đã xuất hiện 3 lần, lần thứ 4 bị cấm.
    /// - Trường Chiếu (Perpetual Check): nếu liên tục chiếu lặp lại quá 3 lần, bị cấm.
    #[must_use]
    pub fn is_repetition_violation(&self, next_hash: u64, is_check: bool) -> bool {
        let mut history_count = 0;
        let mut history_matches = Vec::new();

        for (idx, r) in self.history.iter().enumerate() {
            if r.hash == next_hash {
                history_count += 1;
                history_matches.push(idx);
            }
        }

        let initial_count = if next_hash == self.initial_hash { 1 } else { 0 };
        let total_count = history_count + initial_count;

        // 1. Cho lặp lại tối đa 3 lần: thế cờ được xuất hiện tối đa 3 lần, lần thứ 4 bị cấm
        if total_count >= 4 {
            return true;
        }

        // 2. Perpetual Check (Trường Chiếu): nếu liên tục chiếu lặp lại từ 3 lần trở lên
        if is_check && total_count >= 3 && !history_matches.is_empty() {
            if let Some(&last_idx) = history_matches.last() {
                if let Some(slice) = self.history.get(last_idx..) {
                    let all_checks = slice
                        .iter()
                        .filter(|r| r.color == self.turn)
                        .all(|r| r.is_check);
                    if all_checks {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check whether a move from `from` to `to` is completely legal,
    /// enforcing all piece movement rules, king-safety, and repetition rules.
    #[must_use]
    pub fn is_move_legal(&self, from: BoardCoordinate, to: BoardCoordinate) -> bool {
        let piece = match self.board.get_piece(from) {
            Some(p) => p,
            None => return false,
        };
        if piece.color != self.turn {
            return false;
        }

        let strictly_legal = self.get_strictly_legal_moves();
        strictly_legal.iter().any(|m| {
            m.from_row as usize == from.row
                && m.from_col as usize == from.col
                && m.to_row as usize == to.row
                && m.to_col as usize == to.col
        })
    }

    /// Return all strictly legal moves for the current player,
    /// excluding moves that would cause a forbidden repetition or perpetual check.
    ///
    /// If the number of strictly legal (clean) moves is less than 2 (i.e. 0 or 1),
    /// the moves otherwise forbidden due to repetition are allowed back into the legal set.
    #[must_use]
    pub fn get_strictly_legal_moves(&self) -> Vec<Move> {
        let generator = MoveGenerator::new();
        let candidate_moves = generator.generate_moves(&self.board, self.turn);

        let mut clean_moves = Vec::with_capacity(candidate_moves.len());
        let mut rep_moves = Vec::with_capacity(candidate_moves.len());

        for mv in candidate_moves {
            let from = BoardCoordinate::new(mv.from_row as usize, mv.from_col as usize);
            let to = BoardCoordinate::new(mv.to_row as usize, mv.to_col as usize);

            let mut next_board = self.board.clone();
            next_board.apply_move(&mv, self.turn);

            let is_check = is_in_check(&next_board, self.turn.opposite());
            let back_forth_count = match (from, to) {
                (Some(f), Some(t)) => self.count_move_back_and_forth(f, t),
                _ => 0,
            };

            let rep_violation = self.is_repetition_violation(next_board.zobrist_hash, is_check)
                || back_forth_count >= 3;

            if rep_violation {
                rep_moves.push(mv);
            } else {
                clean_moves.push(mv);
            }
        }

        if clean_moves.len() < 2 {
            clean_moves.extend(rep_moves);
        }

        clean_moves
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
