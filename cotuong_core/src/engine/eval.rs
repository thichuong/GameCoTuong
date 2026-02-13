use crate::engine::config::EngineConfig;
use crate::engine::Evaluator;
use crate::logic::board::Board;
use std::sync::Arc;

pub struct SimpleEvaluator {
    #[allow(dead_code)]
    config: Arc<EngineConfig>,
}

impl SimpleEvaluator {
    pub const fn new(config: Arc<EngineConfig>) -> Self {
        Self { config }
    }
}

impl Evaluator for SimpleEvaluator {
    fn evaluate(&self, board: &Board) -> i32 {
        use crate::logic::eval_constants::*;

        // 1. Material & PST (Base)
        let red_material = board.red_material + board.red_pst;
        let black_material = board.black_material + board.black_pst;

        let mut score = red_material - black_material;

        // 2. Mobility & Positional
        use crate::logic::board::BitboardIterator;
        use crate::logic::lookup::AttackTables;

        let tables = AttackTables::get();

        // Helper buffers for mobility
        let mut red_mobility = 0;
        let mut black_mobility = 0;

        let all_pieces = board.occupied;

        // -- Red Mobility --
        // Chariots (Index 4)
        for sq in BitboardIterator::new(board.bitboards[4]) {
            let (r, c) = SQ_TO_COORD[sq];
            let rank_occ = board.occupied_rows[r];
            let file_occ = board.occupied_cols[c];
            let attacks =
                tables.get_rook_attacks(c, rank_occ, 9) | tables.get_rook_attacks(r, file_occ, 10);
            red_mobility += (attacks.count_ones() as i32) * WEIGHT_MOBILITY_ROOK;
        }
        // Horses (Index 3)
        for sq in BitboardIterator::new(board.bitboards[3]) {
            let (r, c) = SQ_TO_COORD[sq];
            // Horse attacks need blocking check.
            // AttackTable `get_horse_moves` usually returns pseudo-legal moves.
            // We need to check blocking pieces.
            // Since we don't have a cheap "count valid horse moves" without generating them,
            // we can use the precomputed attack table logic if exposed, OR just bitboard hacks.
            // For now, let's assume `get_horse_moves` *without* blocking is available,
            // but `tables` in current codebase might only have `get_horse_attacks(sq)`.
            // Let's rely on `gen_horse_moves` equivalent logic if possible, or simplified:
            // Simplified: +1 for each empty neighbor? No, that's not accuracy.
            // Let's stick to standard attacks.

            // Note: `tables` here is `&AttackTables`.
            // We'll iterate standard horse moves and check blocking.
            // Since we want speed, maybe we skip full blocking check for mobility?
            // No, blocking is critical for Horse value ("Mã ngọa tào" vs "Mã què").

            // Optimization: Just check the 4 orth neighbors for blockers.
            // If neighbor is empty, add mobility for the 2 corresponding jumps?
            // This logic is duplicated from MoveGen but simplified for score.

            // Hardcoding offsets for speed context (simplified):
            let neighbors = [(-1, 0), (1, 0), (0, -1), (0, 1)]; // N, S, W, E
            for &(dr, dc) in &neighbors {
                let br = r as i32 + dr;
                let bc = c as i32 + dc;
                if (0..10).contains(&br) && (0..9).contains(&bc) {
                    let block_sq = Board::square_index(br as usize, bc as usize);
                    if (all_pieces & (1 << block_sq)) == 0 {
                        // Leg is free, adding "potential" mobility.
                        // Real mobility depends on target square emptiness/capture, but "control" is good too.
                        red_mobility += 2 * WEIGHT_MOBILITY_HORSE;
                    }
                }
            }
        }
        // Cannons (Index 5)
        for sq in BitboardIterator::new(board.bitboards[5]) {
            let (r, c) = SQ_TO_COORD[sq];
            let rank_occ = board.occupied_rows[r];
            let file_occ = board.occupied_cols[c];
            let attacks = tables.get_cannon_attacks(c, rank_occ, 9)
                | tables.get_cannon_attacks(r, file_occ, 10);
            red_mobility += (attacks.count_ones() as i32) * WEIGHT_MOBILITY_CANNON;
        }

        // -- Black Mobility --
        // Chariots (Index 11)
        for sq in BitboardIterator::new(board.bitboards[11]) {
            let (r, c) = SQ_TO_COORD[sq];
            let rank_occ = board.occupied_rows[r];
            let file_occ = board.occupied_cols[c];
            let attacks =
                tables.get_rook_attacks(c, rank_occ, 9) | tables.get_rook_attacks(r, file_occ, 10);
            black_mobility += (attacks.count_ones() as i32) * WEIGHT_MOBILITY_ROOK;
        }
        // Horses (Index 10)
        for sq in BitboardIterator::new(board.bitboards[10]) {
            let (r, c) = SQ_TO_COORD[sq];
            let neighbors = [(-1, 0), (1, 0), (0, -1), (0, 1)];
            for &(dr, dc) in &neighbors {
                let br = r as i32 + dr;
                let bc = c as i32 + dc;
                if (0..10).contains(&br) && (0..9).contains(&bc) {
                    let block_sq = Board::square_index(br as usize, bc as usize);
                    if (all_pieces & (1 << block_sq)) == 0 {
                        black_mobility += 2 * WEIGHT_MOBILITY_HORSE;
                    }
                }
            }
        }
        // Cannons (Index 12)
        for sq in BitboardIterator::new(board.bitboards[12]) {
            let (r, c) = SQ_TO_COORD[sq];
            let rank_occ = board.occupied_rows[r];
            let file_occ = board.occupied_cols[c];
            let attacks = tables.get_cannon_attacks(c, rank_occ, 9)
                | tables.get_cannon_attacks(r, file_occ, 10);
            black_mobility += (attacks.count_ones() as i32) * WEIGHT_MOBILITY_CANNON;
        }

        score += red_mobility - black_mobility;

        // 3. Structure & Safety
        let mut red_structure = 0;
        let mut black_structure = 0;

        // Connected Advisors (Red: Index 1, Black: Index 8)
        // Simple check: if count == 2, assume connected (usually true in palace).
        // Or check adjacency? Palace is small, 2 advisors are almost always supporting each other.
        if board.bitboards[1].count_ones() == 2 {
            red_structure += BONUS_CONNECTED_ADVISORS;
        }
        if board.bitboards[8].count_ones() == 2 {
            black_structure += BONUS_CONNECTED_ADVISORS;
        }

        // Connected Elephants (Red: Index 2, Black: Index 9)
        // Red Elephants are at (0,2), (0,6), (2,0), (2,4), (2,8), (4,2), (4,6)
        // Connected usually means they share the central eye (2,4) or support each other.
        // If 2 elephants exist and one is at 2,4 (Red) / 7,4 (Black), robust.
        if board.bitboards[2].count_ones() == 2 {
            // Check if one is at center (row 2, col 4 -> index 22)
            if (board.bitboards[2] & (1 << 22)) != 0 {
                red_structure += BONUS_CONNECTED_ELEPHANTS;
            }
        }
        if board.bitboards[9].count_ones() == 2 {
            // Check if one is at center (row 7, col 4 -> index 67)
            if (board.bitboards[9] & (1 << 67)) != 0 {
                black_structure += BONUS_CONNECTED_ELEPHANTS;
            }
        }

        score += red_structure - black_structure;

        // 4. King Safety (Sophisticated)
        let mut red_danger = 0;
        let mut black_danger = 0;

        // King Exposed on File
        if let Some(red_king_sq) = BitboardIterator::new(board.bitboards[0]).next() {
            let (kr, kc) = SQ_TO_COORD[red_king_sq];
            // Check open file (no friendly pieces in front?)
            // Actually, "Exposed" usually implies facing enemy Rook/Cannon without cover.
            // Simplified: If King is on same file as enemy Rook/Cannon with no pieces in between.

            // Check Enemy Rooks (Index 11)
            let enemy_rooks = board.bitboards[11];
            for rsq in BitboardIterator::new(enemy_rooks) {
                let (rr, rc) = SQ_TO_COORD[rsq];
                if rc == kc {
                    // Rook on same file.
                    let min_r = kr.min(rr);
                    let max_r = kr.max(rr);
                    if max_r > min_r + 1 {
                        let mask = ((1u16 << max_r) - 1) ^ ((1u16 << (min_r + 1)) - 1);
                        let count = (board.occupied_cols[kc] & mask).count_ones();
                        if count == 0 {
                            red_danger += WEIGHT_KING_EXPOSED;
                        }
                    }
                }
            }
        }

        if let Some(black_king_sq) = BitboardIterator::new(board.bitboards[7]).next() {
            let (kr, kc) = SQ_TO_COORD[black_king_sq];
            let enemy_rooks = board.bitboards[4]; // Red Rooks
            for rsq in BitboardIterator::new(enemy_rooks) {
                let (rr, rc) = SQ_TO_COORD[rsq];
                if rc == kc {
                    let min_r = kr.min(rr);
                    let max_r = kr.max(rr);
                    if max_r > min_r + 1 {
                        let mask = ((1u16 << max_r) - 1) ^ ((1u16 << (min_r + 1)) - 1);
                        let count = (board.occupied_cols[kc] & mask).count_ones();
                        if count == 0 {
                            black_danger += WEIGHT_KING_EXPOSED;
                        }
                    }
                }
            }
        }

        // Cannon Danger (Existing "King Exposed to Cannon" logic + Mount check + Empty Cannon)
        // Red King vs Black Cannons (Index 12)
        if let Some(red_king_sq) = BitboardIterator::new(board.bitboards[0]).next() {
            let (kr, kc) = SQ_TO_COORD[red_king_sq];
            for csq in BitboardIterator::new(board.bitboards[12]) {
                let (cr, cc) = SQ_TO_COORD[csq];
                if kr == cr {
                    // Rank
                    let min_c = kc.min(cc);
                    let max_c = kc.max(cc);
                    if max_c > min_c + 1 {
                        let mask = ((1u16 << max_c) - 1) ^ ((1u16 << (min_c + 1)) - 1);
                        let count = (board.occupied_rows[kr] & mask).count_ones();
                        if count <= 1 {
                            // 0 pieces (Empty Cannon) or 1 piece (Check) = Danger!
                            red_danger += self.config.king_exposed_cannon_penalty;
                        }
                    }
                } else if kc == cc {
                    // File
                    let min_r = kr.min(cr);
                    let max_r = kr.max(cr);
                    if max_r > min_r + 1 {
                        let mask = ((1u16 << max_r) - 1) ^ ((1u16 << (min_r + 1)) - 1);
                        let count = (board.occupied_cols[kc] & mask).count_ones();
                        if count <= 1 {
                            red_danger += self.config.king_exposed_cannon_penalty;
                        }
                    }
                }
            }
        }

        // Black King vs Red Cannons (Index 5)
        if let Some(black_king_sq) = BitboardIterator::new(board.bitboards[7]).next() {
            let (kr, kc) = SQ_TO_COORD[black_king_sq];
            for csq in BitboardIterator::new(board.bitboards[5]) {
                let (cr, cc) = SQ_TO_COORD[csq];
                if kr == cr {
                    let min_c = kc.min(cc);
                    let max_c = kc.max(cc);
                    if max_c > min_c + 1 {
                        let mask = ((1u16 << max_c) - 1) ^ ((1u16 << (min_c + 1)) - 1);
                        let count = (board.occupied_rows[kr] & mask).count_ones();
                        if count <= 1 {
                            black_danger += self.config.king_exposed_cannon_penalty;
                        }
                    }
                } else if kc == cc {
                    let min_r = kr.min(cr);
                    let max_r = kr.max(cr);
                    if max_r > min_r + 1 {
                        let mask = ((1u16 << max_r) - 1) ^ ((1u16 << (min_r + 1)) - 1);
                        let count = (board.occupied_cols[kc] & mask).count_ones();
                        if count <= 1 {
                            black_danger += self.config.king_exposed_cannon_penalty;
                        }
                    }
                }
            }
        }

        score -= red_danger - black_danger; // Danger is bad

        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::board::{Board, BoardCoordinate, Color, PieceType};

    #[test]
    fn test_king_exposed_penalty() {
        let mut config = EngineConfig::default();
        config.king_exposed_cannon_penalty = 100; // Use a large penalty for clarity
        let evaluator = SimpleEvaluator::new(Arc::new(config));

        // 1. Setup Red King exposed to Black Cannon (1 piece between) - Check
        let mut board = Board::new();
        board.clear();

        // Red King at (0, 4)
        board.add_piece(
            BoardCoordinate::new(0, 4).unwrap(),
            PieceType::General,
            Color::Red,
        );

        // Black King at (9, 4) (Safe for now)
        board.add_piece(
            BoardCoordinate::new(9, 4).unwrap(),
            PieceType::General,
            Color::Black,
        );

        // Black Cannon at (5, 4) - Same File
        board.add_piece(
            BoardCoordinate::new(5, 4).unwrap(),
            PieceType::Cannon,
            Color::Black,
        );

        // Intervening piece (Red Advisor at (1, 4))
        board.add_piece(
            BoardCoordinate::new(1, 4).unwrap(),
            PieceType::Advisor,
            Color::Red,
        );

        // Calculate score
        let _score_exposed_1 = evaluator.evaluate(&board);

        // Test 0 pieces (Empty cannon)
        board.set_piece(BoardCoordinate::new(1, 4).unwrap(), None);
        // Now 0 pieces between King (0,4) and Cannon (5,4).

        let score_exposed_0 = evaluator.evaluate(&board);

        // Let's check non-exposed (Cannon on different file)
        board.set_piece(BoardCoordinate::new(5, 4).unwrap(), None);
        board.add_piece(
            BoardCoordinate::new(5, 3).unwrap(),
            PieceType::Cannon,
            Color::Black,
        );
        let score_safe_file = evaluator.evaluate(&board);

        assert!(
            score_safe_file > score_exposed_0 + 50,
            "Penalty should be applied for 0 pieces (Empty Cannon)"
        );

        // Check 1 piece (Check)
        board.clear();
        board.add_piece(
            BoardCoordinate::new(0, 4).unwrap(),
            PieceType::General,
            Color::Red,
        );
        board.add_piece(
            BoardCoordinate::new(9, 4).unwrap(),
            PieceType::General,
            Color::Black,
        );

        // Case A: Exposed (Cannon at 5,4, 1 blocker at 2,4)
        board.add_piece(
            BoardCoordinate::new(5, 4).unwrap(),
            PieceType::Cannon,
            Color::Black,
        );
        board.add_piece(
            BoardCoordinate::new(2, 4).unwrap(),
            PieceType::Advisor,
            Color::Red,
        );
        let score_exposed_1 = evaluator.evaluate(&board);

        // Case B: Safe (Cannon at 5,3, 1 blocker at 2,4) -> Blocker irrelevant for 5,3
        board.set_piece(BoardCoordinate::new(5, 4).unwrap(), None);
        board.add_piece(
            BoardCoordinate::new(5, 3).unwrap(),
            PieceType::Cannon,
            Color::Black,
        );
        let score_safe_1 = evaluator.evaluate(&board);

        assert!(
            score_safe_1 > score_exposed_1 + 50,
            "Penalty should be applied for 1 piece (Check)"
        );

        // Check 2 pieces (Safe)
        // Cannon back to 5,4
        board.set_piece(BoardCoordinate::new(5, 3).unwrap(), None);
        board.add_piece(
            BoardCoordinate::new(5, 4).unwrap(),
            PieceType::Cannon,
            Color::Black,
        );
        // Add 2nd blocker - Elephant at 0,2 (Rank 0 occupied: King 0,4. Elephant 0,2. Advisor 0,1?)
        // Wait, simply add piece at 3,4.
        // Current blocker at 2,4. Add at 3,4.
        board.add_piece(
            BoardCoordinate::new(3, 4).unwrap(),
            PieceType::Elephant,
            Color::Red,
        );
        let score_blocked_2 = evaluator.evaluate(&board);

        // Compare with Cannon side 5,3
        board.set_piece(BoardCoordinate::new(5, 4).unwrap(), None);
        board.add_piece(
            BoardCoordinate::new(5, 3).unwrap(),
            PieceType::Cannon,
            Color::Black,
        );
        let score_blocked_side = evaluator.evaluate(&board);

        assert!(
            (score_blocked_2 - score_blocked_side).abs() < 50,
            "Penalty should NOT be applied for 2 pieces"
        );
    }
}
