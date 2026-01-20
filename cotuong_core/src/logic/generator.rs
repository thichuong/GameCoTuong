use crate::engine::Move;
use crate::logic::board::{BitboardIterator, Board, BoardCoordinate, Color, PieceType};
use crate::logic::lookup::AttackTables;
use crate::logic::rules::is_valid_move;

pub struct MoveGenerator;

impl MoveGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_moves(&self, board: &Board, turn: Color) -> Vec<Move> {
        let mut moves = Vec::with_capacity(64);

        // Iterate over bitboards for the current turn
        let start_idx = turn.index() * 7;
        for i in 0..7 {
            let bb = board.bitboards[start_idx + i];
            for sq in BitboardIterator::new(bb) {
                let piece_type = match i {
                    0 => PieceType::General,
                    1 => PieceType::Advisor,
                    2 => PieceType::Elephant,
                    3 => PieceType::Horse,
                    4 => PieceType::Chariot,
                    5 => PieceType::Cannon,
                    6 => PieceType::Soldier,
                    _ => unreachable!(),
                };

                // Safety: BitboardIterator returns valid indices 0..89
                let (r, c) = Board::index_to_coord(sq);
                let from = unsafe { BoardCoordinate::new_unchecked(r, c) };

                self.generate_piece_moves(board, from, piece_type, turn, &mut moves);
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
        // We still use is_valid_move for complex checks (like flying general or self-check),
        // but simple geometry is now handled by lookup tables.
        // Optimization: In a real engine, we would defer "is_valid_move" (self-check)
        // until *making* the move in search, or check it here if we want strictly legal moves.
        // `is_valid_move` does:
        // 1. Bounds check (handled by lookup)
        // 2. Color check (handled by lookup/generator)
        // 3. Piece rules (handled by lookup)
        // 4. Flying general (NOT handled)
        // 5. Self-check (NOT handled)

        // For now, to keep behavior identical, we call is_valid_move.
        // But since we pre-filtered geometry, it should be faster if is_valid_move short-circuits?
        // Actually is_valid_move re-checks everything.
        // Optimally, we should just check the crucial parts: Flying General & Self-Check.

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
        let tables = AttackTables::get();
        let sq = from.index();

        for &target_sq in &tables.general_moves[sq] {
            let (tr, tc) = Board::index_to_coord(target_sq);
            let to = unsafe { BoardCoordinate::new_unchecked(tr, tc) };
            self.try_add_move(board, from, to, turn, moves);
        }
    }

    fn generate_advisor_moves(
        &self,
        board: &Board,
        from: BoardCoordinate,
        turn: Color,
        moves: &mut Vec<Move>,
    ) {
        let tables = AttackTables::get();
        let sq = from.index();

        for &target_sq in &tables.advisor_moves[sq] {
            let (tr, tc) = Board::index_to_coord(target_sq);
            let to = unsafe { BoardCoordinate::new_unchecked(tr, tc) };
            self.try_add_move(board, from, to, turn, moves);
        }
    }

    fn generate_elephant_moves(
        &self,
        board: &Board,
        from: BoardCoordinate,
        turn: Color,
        moves: &mut Vec<Move>,
    ) {
        let tables = AttackTables::get();
        let sq = from.index();

        for &(target_sq, eye_sq) in &tables.elephant_moves[sq] {
            // Check eye
            if board.grid[eye_sq].is_none() {
                let (tr, tc) = Board::index_to_coord(target_sq);
                let to = unsafe { BoardCoordinate::new_unchecked(tr, tc) };
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
        let tables = AttackTables::get();
        let sq = from.index();

        for &(target_sq, leg_sq) in &tables.horse_moves[sq] {
            // Check leg
            if board.grid[leg_sq].is_none() {
                let (tr, tc) = Board::index_to_coord(target_sq);
                let to = unsafe { BoardCoordinate::new_unchecked(tr, tc) };
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
        // Use Magic/Rotated bitboards or simple lookups for sliding pieces?
        // We have `get_rook_attacks` in tables.
        let tables = AttackTables::get();
        let r = from.row;
        let c = from.col;

        let rank_occ = board.occupied_rows[r];
        let mut attacks = tables.get_rook_attacks(c, rank_occ, 9);
        while attacks != 0 {
            let col = attacks.trailing_zeros() as usize;
            attacks &= attacks - 1;
            let to = unsafe { BoardCoordinate::new_unchecked(r, col) };
            self.try_add_move(board, from, to, turn, moves);
        }

        let file_occ = board.occupied_cols[c];
        let mut attacks = tables.get_rook_attacks(r, file_occ, 10);
        while attacks != 0 {
            let row = attacks.trailing_zeros() as usize;
            attacks &= attacks - 1;
            let to = unsafe { BoardCoordinate::new_unchecked(row, c) };
            self.try_add_move(board, from, to, turn, moves);
        }
    }

    fn generate_cannon_moves(
        &self,
        board: &Board,
        from: BoardCoordinate,
        turn: Color,
        moves: &mut Vec<Move>,
    ) {
        let tables = AttackTables::get();
        let r = from.row;
        let c = from.col;

        let rank_occ = board.occupied_rows[r];
        let mut attacks = tables.get_cannon_attacks(c, rank_occ, 9);
        while attacks != 0 {
            let col = attacks.trailing_zeros() as usize;
            attacks &= attacks - 1;
            let to = unsafe { BoardCoordinate::new_unchecked(r, col) };
            self.try_add_move(board, from, to, turn, moves);
        }

        let file_occ = board.occupied_cols[c];
        let mut attacks = tables.get_cannon_attacks(r, file_occ, 10);
        while attacks != 0 {
            let row = attacks.trailing_zeros() as usize;
            attacks &= attacks - 1;
            let to = unsafe { BoardCoordinate::new_unchecked(row, c) };
            self.try_add_move(board, from, to, turn, moves);
        }
    }

    fn generate_soldier_moves(
        &self,
        board: &Board,
        from: BoardCoordinate,
        turn: Color,
        moves: &mut Vec<Move>,
    ) {
        let tables = AttackTables::get();
        let sq = from.index();
        let color_idx = turn.index();

        for &target_sq in &tables.soldier_moves[color_idx][sq] {
            let (tr, tc) = Board::index_to_coord(target_sq);
            let to = unsafe { BoardCoordinate::new_unchecked(tr, tc) };
            self.try_add_move(board, from, to, turn, moves);
        }
    }

    // `offset` helper is no longer needed but we can keep it if strictly necessary,
    // but the above implementation replaces it.
    // We'll remove it to clean up.
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
        // Previously > 5. Now we generate exact set.
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
