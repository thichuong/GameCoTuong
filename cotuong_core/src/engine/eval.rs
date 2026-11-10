use crate::engine::config::EngineConfig;
use crate::engine::nnue::{evaluate_nnue_with_weights, NnueWeights};
use crate::engine::Evaluator;
use crate::logic::board::Board;
use std::sync::Arc;

pub struct SimpleEvaluator {
    config: Arc<EngineConfig>,
    weights: Option<Arc<NnueWeights>>,
}

impl SimpleEvaluator {
    #[must_use]
    pub const fn new(config: Arc<EngineConfig>) -> Self {
        Self {
            config,
            weights: None,
        }
    }

    #[must_use]
    pub fn with_weights(config: Arc<EngineConfig>, weights: Arc<NnueWeights>) -> Self {
        Self {
            config,
            weights: Some(weights),
        }
    }

    pub fn set_weights(&mut self, weights: Option<Arc<NnueWeights>>) {
        self.weights = weights;
    }

    #[must_use]
    pub fn weights(&self) -> &NnueWeights {
        match &self.weights {
            Some(w) => w.as_ref(),
            None => NnueWeights::get(),
        }
    }
}

impl Evaluator for SimpleEvaluator {
    #[inline]
    fn evaluate(&self, board: &Board) -> i32 {
        if self.config.nnue_weight == 0 {
            return 0;
        }
        let nnue_weights = match &self.weights {
            Some(w) => w.as_ref(),
            None => NnueWeights::get(),
        };
        let raw = evaluate_nnue_with_weights(board, nnue_weights);
        (raw * self.config.nnue_weight) / 100
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::board::{Board, BoardCoordinate, Color, PieceType};

    #[test]
    fn test_nnue_initial_symmetry() {
        let config = Arc::new(EngineConfig::default());
        let evaluator = SimpleEvaluator::new(config);
        let board = Board::new();
        let score = evaluator.evaluate(&board);
        assert_eq!(score, 0, "Initial symmetrical board must evaluate to 0");
    }

    #[test]
    fn test_nnue_material_advantage() {
        let config = Arc::new(EngineConfig::default());
        let evaluator = SimpleEvaluator::new(config);

        let mut board = Board::new();
        let score_initial = evaluator.evaluate(&board);
        assert_eq!(score_initial, 0);

        // Remove Black's left Chariot at (9, 0)
        board.set_piece(BoardCoordinate::new(9, 0).unwrap(), None);
        let score_no_black_chariot = evaluator.evaluate(&board);

        // Verify that the evaluation reacts deterministically to material changes
        assert_ne!(
            score_no_black_chariot, score_initial,
            "Board evaluation must reflect piece count changes"
        );
    }

    #[test]
    fn test_king_exposed_penalty() {
        let config = Arc::new(EngineConfig::default());
        let evaluator = SimpleEvaluator::new(config);

        let mut board = Board::new();
        board.clear();

        // Red King at (0, 4)
        board.add_piece(
            BoardCoordinate::new(0, 4).unwrap(),
            PieceType::General,
            Color::Red,
        );
        // Black King at (9, 4)
        board.add_piece(
            BoardCoordinate::new(9, 4).unwrap(),
            PieceType::General,
            Color::Black,
        );
        // Black Cannon at (5, 4) - Same file as Red King (Empty Cannon, 0 blockers)
        board.add_piece(
            BoardCoordinate::new(5, 4).unwrap(),
            PieceType::Cannon,
            Color::Black,
        );

        let score_exposed_0 = evaluator.evaluate(&board);

        // Move Black Cannon to safe file (5, 3)
        board.set_piece(BoardCoordinate::new(5, 4).unwrap(), None);
        board.add_piece(
            BoardCoordinate::new(5, 3).unwrap(),
            PieceType::Cannon,
            Color::Black,
        );
        let score_safe_file = evaluator.evaluate(&board);

        assert!(
            score_safe_file > score_exposed_0 + 30,
            "NNUE should penalize 0-blocker Empty Cannon on King file (safe={score_safe_file}, exposed={score_exposed_0})"
        );

        // Check 1 piece (Cannon Check on King file)
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

        board.set_piece(BoardCoordinate::new(5, 4).unwrap(), None);
        board.add_piece(
            BoardCoordinate::new(5, 3).unwrap(),
            PieceType::Cannon,
            Color::Black,
        );
        let score_safe_1 = evaluator.evaluate(&board);

        assert!(
            score_safe_1 > score_exposed_1 + 30,
            "NNUE should penalize 1-blocker Cannon mount on King file (safe={score_safe_1}, exposed={score_exposed_1})"
        );
    }
}
