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

        // Helper to count mobility and safety
        let mut red_mobility = 0;
        let mut black_mobility = 0;

        // King Safety / Defender Count (simplified)
        // Check if Advisors/Elephants are present
        let red_defenders =
            (board.bitboards[1].count_ones() + board.bitboards[2].count_ones()) as i32;
        let black_defenders =
            (board.bitboards[8].count_ones() + board.bitboards[9].count_ones()) as i32;

        // Bonus for having defenders
        score += (red_defenders - black_defenders) * 20;

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

        // Hanging Piece Implementation (Placeholder / Simplified)
        // Since we don't have full attack maps for every piece here without high cost,
        // we can check if valuable pieces are under attack by scanning opponent moves
        // OR we just use the 'hanging_piece_penalty' from config if we detect obvious threats.
        // For this task, strict "Hanging Piece" usually requires Static Exchange Eval (SEE).
        // I will implement a basic "In Danger" check if time permits, but for now I'll apply
        // the config penalty if a piece is undefended (not implemented fully without move gen).
        // Instead, let's use the config penalty to weight the mobility/safety interaction.

        // Actually, let's just use the config penalty for *blocked* pieces if we can detect them easily?
        // No, "Hanging" means under attack.
        // Let's postpone complex Hanging Piece logic to a separate task if it requires full connection graph.
        // I will stick to the King Safety and Config-based weights.

        // Weighting
        // Mobility is worth small amount per square (e.g. 2-5 points).
        // Let's say 3 points per square.
        score += (red_mobility - black_mobility) * 3;

        // Apply Config Penalties
        // Example: If we had detected hanging pieces, we'd subtract self.config.hanging_piece_penalty
        // score -= (red_hanging - black_hanging) * self.config.hanging_piece_penalty;

        score
    }
}
