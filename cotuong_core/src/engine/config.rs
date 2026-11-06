use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineConfig {
    pub pruning_method: i32, // 0: Dynamic Limiting, 1: LMR, 2: Both
    pub pruning_multiplier: f32,

    // ProbCut Parameters
    pub probcut_depth: u8,
    pub probcut_margin: i32,
    pub probcut_reduction: u8,

    // Singular Extension Parameters
    pub singular_extension_min_depth: u8,
    pub singular_extension_margin: i32,

    // Checkmate scoring
    pub mate_score: i32, // Base score for checkmate (higher = stronger preference)

    // Transposition Table
    pub tt_size_mb: usize,

    // Evaluation mode (2: Pure NNUE)
    pub eval_mode: i32,
    // NNUE weight / scale percentage (50..=150)
    pub nnue_weight: i32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            pruning_method: 1, // Default to LMR
            pruning_multiplier: 1.0,

            probcut_depth: 5,
            probcut_margin: 200,
            probcut_reduction: 4,

            singular_extension_min_depth: 8,
            singular_extension_margin: 20,

            mate_score: 300_000,

            tt_size_mb: 256,
            eval_mode: 2,
            nnue_weight: 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EngineConfig::default();
        assert_eq!(config.pruning_method, 1);
        assert!((config.pruning_multiplier - 1.0).abs() < f32::EPSILON);
        assert_eq!(config.mate_score, 300_000);
        assert_eq!(config.eval_mode, 2);
        assert_eq!(config.nnue_weight, 100);
    }

    #[test]
    fn test_deserialize_partial_config() {
        let json = r#"{
            "pruning_method": 2,
            "pruning_multiplier": 1.5,
            "mate_score": 40000,
            "nnue_weight": 110
        }"#;

        let config: EngineConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.pruning_method, 2);
        assert!((config.pruning_multiplier - 1.5).abs() < f32::EPSILON);
        assert_eq!(config.mate_score, 40_000);
        assert_eq!(config.nnue_weight, 110);
        assert_eq!(config.probcut_depth, 5);
    }
}
