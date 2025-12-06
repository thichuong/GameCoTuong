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
        let red_material = board.red_material + board.red_pst;
        let black_material = board.black_material + board.black_pst;

        let mut score = red_material - black_material;

        // 2. Mobility & Positional
        // We use a simplified mobility calculation for speed.

        use crate::logic::board::BitboardIterator;
        use crate::logic::lookup::AttackTables;

        let tables = AttackTables::get();

        // Helper to count mobility
        let mut red_mobility = 0;
        let mut black_mobility = 0;

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

        // Red Horses (Index 3) - Simplified (just count empty legs roughly or just central control)
        // For speed, we skip complex horse leg checks in eval for now, or assume average mobility.
        // But we can give bonus for horses in center.
        // (PST already handles position, so maybe just leave it).

        // Weighting
        // Mobility is worth small amount per square (e.g. 2-5 points).
        // Let's say 3 points per square.
        score += (red_mobility - black_mobility) * 3;

        score
    }
}
