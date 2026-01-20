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
        // 1. Material & PST (Base)
        // Note: PSTs now include significant positional knowledge (e.g., Horse center control, Pawn advancement)
        let red_material = board.red_material + board.red_pst;
        let black_material = board.black_material + board.black_pst;

        let mut score = red_material - black_material;

        // 2. Tempo (Side to move bonus) - REMOVED (Board doesn't store turn)
        // We can add this in search.rs if needed, but for now strict static eval.

        // 3. Mobility & Positional
        use crate::logic::board::BitboardIterator;
        use crate::logic::lookup::AttackTables;

        let tables = AttackTables::get();

        // Mobility Weights (Scaled to new system: 1 pawn = 100)
        // 1 square = 10 points (0.1 pawn)
        const MOBILITY_WEIGHT: i32 = 10;

        // Defensive Bonus
        const DEFENDER_BONUS: i32 = 40; // Per defender (Advisor/Elephant)

        let mut red_mobility = 0;
        let mut black_mobility = 0;

        // --- DEFENDER COUNTS ---
        // Advisors (1) + Elephants (2)
        let red_defenders =
            (board.bitboards[1].count_ones() + board.bitboards[2].count_ones()) as i32;
        let black_defenders =
            (board.bitboards[8].count_ones() + board.bitboards[9].count_ones()) as i32;

        score += (red_defenders - black_defenders) * DEFENDER_BONUS;

        // --- MOBILITY ---

        // Red Chariots (Index 4)
        for sq in BitboardIterator::new(board.bitboards[4]) {
            let (r, c) = Board::index_to_coord(sq);
            let rank_occ = board.occupied_rows[r];
            let file_occ = board.occupied_cols[c];
            let attacks = tables.get_rook_attacks(c, rank_occ, 9).count_ones()
                + tables.get_rook_attacks(r, file_occ, 10).count_ones();
            red_mobility += attacks as i32;
        }

        // Black Chariots (Index 11)
        for sq in BitboardIterator::new(board.bitboards[11]) {
            let (r, c) = Board::index_to_coord(sq);
            let rank_occ = board.occupied_rows[r];
            let file_occ = board.occupied_cols[c];
            let attacks = tables.get_rook_attacks(c, rank_occ, 9).count_ones()
                + tables.get_rook_attacks(r, file_occ, 10).count_ones();
            black_mobility += attacks as i32;
        }

        // Red Cannons (Index 5)
        for sq in BitboardIterator::new(board.bitboards[5]) {
            let (r, c) = Board::index_to_coord(sq);
            let rank_occ = board.occupied_rows[r];
            let file_occ = board.occupied_cols[c];
            let attacks = tables.get_cannon_attacks(c, rank_occ, 9).count_ones()
                + tables.get_cannon_attacks(r, file_occ, 10).count_ones();
            red_mobility += attacks as i32;
        }

        // Black Cannons (Index 12)
        for sq in BitboardIterator::new(board.bitboards[12]) {
            let (r, c) = Board::index_to_coord(sq);
            let rank_occ = board.occupied_rows[r];
            let file_occ = board.occupied_cols[c];
            let attacks = tables.get_cannon_attacks(c, rank_occ, 9).count_ones()
                + tables.get_cannon_attacks(r, file_occ, 10).count_ones();
            black_mobility += attacks as i32;
        }

        // Red Horses (Index 3)
        // For Horses, we don't have a cheap bitboard lookup for moves *blocked* by pieces easily without precomputation
        // that handles blocking.
        // But we can give a small bonus for being "centered" which is already in PST.
        // For actual mobility, we might skip it for now to save cycles, or implement a simple check.
        // Let's stick to Rooks/Cannons for mobility as they are longest range.

        score += (red_mobility - black_mobility) * MOBILITY_WEIGHT;

        // 4. King Safety (Cannon Exposure & Empty Center)

        // Penalty for Exposed King vs Cannons
        let exposed_penalty = self.config.king_exposed_cannon_penalty;

        // --- Red King Safety ---
        if let Some(red_king_sq) = BitboardIterator::new(board.bitboards[0]).next() {
            let (kr, kc) = Board::index_to_coord(red_king_sq);

            // Check against Black Cannons (12)
            for cannon_sq in BitboardIterator::new(board.bitboards[12]) {
                let (cr, cc) = Board::index_to_coord(cannon_sq);

                let mut is_exposed = false;
                // Same File
                if kc == cc {
                    // Check blockers
                    let min_r = kr.min(cr);
                    let max_r = kr.max(cr);
                    if max_r > min_r + 1 {
                        let mask = ((1u16 << max_r) - 1) ^ ((1u16 << (min_r + 1)) - 1);
                        let count = (board.occupied_cols[kc] & mask).count_ones();
                        if count <= 1 {
                            is_exposed = true;
                        } // 0 (check) or 1 (cannon mount)
                    } else {
                        is_exposed = true; // Adjacent
                    }
                } else if kr == cr {
                    // Same Rank (Ironbolt)
                    let min_c = kc.min(cc);
                    let max_c = kc.max(cc);
                    if max_c > min_c + 1 {
                        let mask = ((1u16 << max_c) - 1) ^ ((1u16 << (min_c + 1)) - 1);
                        let count = (board.occupied_rows[kr] & mask).count_ones();
                        if count <= 1 {
                            is_exposed = true;
                        }
                    } else {
                        is_exposed = true;
                    }
                }

                if is_exposed {
                    score -= exposed_penalty;
                }
            }

            // Penalty for empty center file if King is there (and exposed)
            // If King is on file 4 (center), and no advisors/elephants protect it...
            // Simplification: Just check if King is on 4 and has no friends in front?
            // Maybe too specific.
        }

        // --- Black King Safety ---
        if let Some(black_king_sq) = BitboardIterator::new(board.bitboards[7]).next() {
            let (kr, kc) = Board::index_to_coord(black_king_sq);

            // Check against Red Cannons (5)
            for cannon_sq in BitboardIterator::new(board.bitboards[5]) {
                let (cr, cc) = Board::index_to_coord(cannon_sq);

                let mut is_exposed = false;
                if kc == cc {
                    let min_r = kr.min(cr);
                    let max_r = kr.max(cr);
                    if max_r > min_r + 1 {
                        let mask = ((1u16 << max_r) - 1) ^ ((1u16 << (min_r + 1)) - 1);
                        let count = (board.occupied_cols[kc] & mask).count_ones();
                        if count <= 1 {
                            is_exposed = true;
                        }
                    } else {
                        is_exposed = true;
                    }
                } else if kr == cr {
                    let min_c = kc.min(cc);
                    let max_c = kc.max(cc);
                    if max_c > min_c + 1 {
                        let mask = ((1u16 << max_c) - 1) ^ ((1u16 << (min_c + 1)) - 1);
                        let count = (board.occupied_rows[kr] & mask).count_ones();
                        if count <= 1 {
                            is_exposed = true;
                        }
                    } else {
                        is_exposed = true;
                    }
                }

                if is_exposed {
                    score += exposed_penalty;
                }
            }
        }

        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::board::Board;

    #[test]
    fn test_eval_material_balance() {
        let config = Arc::new(EngineConfig::default());
        let evaluator = SimpleEvaluator::new(config);
        let board = Board::new(); // Default start position

        let score = evaluator.evaluate(&board);
        // Start position is balanced (score 0)
        assert_eq!(score, 0);
    }
}
