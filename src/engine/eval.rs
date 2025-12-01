use crate::engine::config::EngineConfig;
use crate::engine::Evaluator;
use crate::logic::board::Board;
use std::sync::Arc;

pub struct SimpleEvaluator {
    #[allow(dead_code)]
    config: Arc<EngineConfig>,
}

impl SimpleEvaluator {
    pub fn new(config: Arc<EngineConfig>) -> Self {
        Self { config }
    }
}

impl Evaluator for SimpleEvaluator {
    fn evaluate(&self, board: &Board) -> i32 {
        // Incremental Evaluation
        // Note: This uses the standard piece values and PSTs defined in eval_constants.
        // If EngineConfig has different values, this will be inaccurate relative to the config,
        // but much faster. For optimization, we prioritize speed here.

        let red_score = board.red_material + board.red_pst;
        let black_score = board.black_material + board.black_pst;

        red_score - black_score
    }
}
