use crate::engine::Move;
use crate::logic::board::{Board, BoardCoordinate, Color, PieceType};
use crate::logic::rules::is_valid_move;

pub struct MoveGenerator;

impl MoveGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_moves(&self, board: &Board, turn: Color) -> Vec<Move> {
        let mut moves = Vec::new();

        // Iterate over all squares to find pieces of the current turn
        for r in 0..10 {
            for c in 0..9 {
                // Safety: r and c are within bounds
                let from = unsafe { BoardCoordinate::new_unchecked(r, c) };

                if let Some(piece) = board.get_piece(from) {
                    if piece.color == turn {
                        self.generate_piece_moves(board, from, piece.piece_type, turn, &mut moves);
                    }
                }
            }
        }

        moves
    }

    fn generate_piece_moves(
        &self,
        board: &Board,
        from: BoardCoordinate,
        piece_type: PieceType,
        turn: Color,
        moves: &mut Vec<Move>,
    ) {
        match piece_type {
            PieceType::General => self.generate_general_moves(board, from, turn, moves),
            PieceType::Advisor => self.generate_advisor_moves(board, from, turn, moves),
            PieceType::Elephant => self.generate_elephant_moves(board, from, turn, moves),
            PieceType::Horse => self.generate_horse_moves(board, from, turn, moves),
            PieceType::Chariot => self.generate_chariot_moves(board, from, turn, moves),
            PieceType::Cannon => self.generate_cannon_moves(board, from, turn, moves),
            PieceType::Soldier => self.generate_soldier_moves(board, from, turn, moves),
        }
    }

    fn try_add_move(
        &self,
        board: &Board,
        from: BoardCoordinate,
        to: BoardCoordinate,
        turn: Color,
        moves: &mut Vec<Move>,
    ) {
        if is_valid_move(board, from, to, turn).is_ok() {
            moves.push(Move {
                from_row: from.row as u8,
                from_col: from.col as u8,
                to_row: to.row as u8,
                to_col: to.col as u8,
                score: 0,
            });
        }
    }

    fn generate_general_moves(
        &self,
        board: &Board,
        from: BoardCoordinate,
        turn: Color,
        moves: &mut Vec<Move>,
    ) {
        let deltas = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        for (dr, dc) in deltas {
            if let Some(to) = self.offset(from, dr, dc) {
                self.try_add_move(board, from, to, turn, moves);
            }
        }
    }

    fn generate_advisor_moves(
        &self,
        board: &Board,
        from: BoardCoordinate,
        turn: Color,
        moves: &mut Vec<Move>,
    ) {
        let deltas = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
        for (dr, dc) in deltas {
            if let Some(to) = self.offset(from, dr, dc) {
                self.try_add_move(board, from, to, turn, moves);
            }
        }
    }

    fn generate_elephant_moves(
        &self,
        board: &Board,
        from: BoardCoordinate,
        turn: Color,
        moves: &mut Vec<Move>,
    ) {
        let deltas = [(2, 2), (2, -2), (-2, 2), (-2, -2)];
        for (dr, dc) in deltas {
            if let Some(to) = self.offset(from, dr, dc) {
                self.try_add_move(board, from, to, turn, moves);
            }
        }
    }

    fn generate_horse_moves(
        &self,
        board: &Board,
        from: BoardCoordinate,
        turn: Color,
        moves: &mut Vec<Move>,
    ) {
        let deltas = [
            (2, 1),
            (2, -1),
            (-2, 1),
            (-2, -1),
            (1, 2),
            (1, -2),
            (-1, 2),
            (-1, -2),
        ];
        for (dr, dc) in deltas {
            if let Some(to) = self.offset(from, dr, dc) {
                self.try_add_move(board, from, to, turn, moves);
            }
        }
    }

    fn generate_chariot_moves(
        &self,
        board: &Board,
        from: BoardCoordinate,
        turn: Color,
        moves: &mut Vec<Move>,
    ) {
        // Horizontal and Vertical
        self.generate_linear_moves(
            board,
            from,
            turn,
            moves,
            &[(0, 1), (0, -1), (1, 0), (-1, 0)],
        );
    }

    fn generate_cannon_moves(
        &self,
        board: &Board,
        from: BoardCoordinate,
        turn: Color,
        moves: &mut Vec<Move>,
    ) {
        // Horizontal and Vertical (Cannon logic handled by is_valid_move largely, but linear scan helps)
        // Actually, linear scan is better for Chariot/Cannon to stop at blocking pieces.
        // But `try_add_move` relies on `is_valid_move` which does the check.
        // To catch all moves efficiently, we should scan until board edge.
        // `is_valid_move` will reject invalid jumps, but we don't want to call it for (0,0) -> (0,8) if (0,1) is blocked for a Chariot.

        // For optimization, we should implement custom linear scan here, OR just rely on `is_valid_move` for now
        // but iterate all possible linear targets.
        // Iterating all targets is still O(10+9) = 19 calls. Safe enough compared to 90.

        let dirs = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        for (dr, dc) in dirs {
            let mut dist = 1;
            while let Some(to) = self.offset(from, dr * dist, dc * dist) {
                self.try_add_move(board, from, to, turn, moves);
                dist += 1;
            }
        }
    }

    fn generate_soldier_moves(
        &self,
        board: &Board,
        from: BoardCoordinate,
        turn: Color,
        moves: &mut Vec<Move>,
    ) {
        let forward = match turn {
            Color::Red => 1,
            Color::Black => -1,
        };

        let deltas = [(forward, 0), (0, 1), (0, -1)];
        for (dr, dc) in deltas {
            if let Some(to) = self.offset(from, dr, dc) {
                self.try_add_move(board, from, to, turn, moves);
            }
        }
    }

    // Helper to add offset
    fn generate_linear_moves(
        &self,
        board: &Board,
        from: BoardCoordinate,
        turn: Color,
        moves: &mut Vec<Move>,
        dirs: &[(isize, isize)],
    ) {
        for (dr, dc) in dirs {
            let mut dist = 1;
            while let Some(to) = self.offset(from, dr * dist, dc * dist) {
                self.try_add_move(board, from, to, turn, moves);
                dist += 1;
            }
        }
    }

    fn offset(
        &self,
        from: BoardCoordinate,
        row_delta: isize,
        col_delta: isize,
    ) -> Option<BoardCoordinate> {
        let r = from.row as isize + row_delta;
        let c = from.col as isize + col_delta;

        if (0..10).contains(&r) && (0..9).contains(&c) {
            // Safety: Bounds checked
            Some(unsafe { BoardCoordinate::new_unchecked(r as usize, c as usize) })
        } else {
            None
        }
    }
}

impl Default for MoveGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::board::Board;

    #[test]
    fn test_initial_moves() {
        let board = Board::new();
        let generator = MoveGenerator::new();
        let moves = generator.generate_moves(&board, Color::Red);

        // Initial position:
        // - 5 Soldiers can move forward (5 moves)
        // - 2 Cannons can move (various moves)
        // - 2 Horses blocked (0 moves)
        // - 2 Chariots blocked (0 moves)
        // - Advisors/Elephants/General blocked or constrained (0 moves)

        assert!(
            !moves.is_empty(),
            "Red should have moves in initial position"
        );
        assert!(moves.len() > 5);
    }

    #[test]
    fn test_stalemate_check() {
        // Create a board where Red has no moves (stalemate or checkmate)
        let mut board = Board::new();
        board.clear();

        // If board is empty, no pieces, so 0 moves.
        let generator = MoveGenerator::new();
        let moves = generator.generate_moves(&board, Color::Red);
        assert!(moves.is_empty());
    }
}
