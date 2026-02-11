use crate::logic::eval_constants::{
    VAL_ADVISOR, VAL_CANNON, VAL_ELEPHANT, VAL_HORSE, VAL_KING, VAL_PAWN, VAL_ROOK,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineConfig {
    // Evaluation Parameters
    pub val_pawn: i32,
    pub val_advisor: i32,
    pub val_elephant: i32,
    pub val_horse: i32,
    pub val_cannon: i32,
    pub val_rook: i32,
    pub val_king: i32,

    // Penalties
    pub hanging_piece_penalty: i32,
    pub king_exposed_cannon_penalty: i32,

    // Search Parameters
    pub score_hash_move: i32,
    pub score_capture_base: i32,
    pub score_killer_move: i32,
    pub score_history_max: i32,

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
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            val_pawn: VAL_PAWN,
            val_advisor: VAL_ADVISOR,
            val_elephant: VAL_ELEPHANT,
            val_horse: VAL_HORSE,
            val_cannon: VAL_CANNON,
            val_rook: VAL_ROOK,
            val_king: VAL_KING,

            hanging_piece_penalty: 10,
            king_exposed_cannon_penalty: 20,

            score_hash_move: 200_000,
            score_capture_base: 200_000, // Aggressive capturing
            score_killer_move: 120_000,
            score_history_max: 80_000,

            pruning_method: 1, // Default to LMR
            pruning_multiplier: 1.0,

            probcut_depth: 5,
            probcut_margin: 200,
            probcut_reduction: 4,

            singular_extension_min_depth: 8,
            singular_extension_margin: 20,

            mate_score: 300_000, // Increased to be higher than score_capture_base (200,000)

            tt_size_mb: 256,
        }
    }
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct EngineConfigJson {
    val_pawn: Option<f32>,
    val_advisor: Option<f32>,
    val_elephant: Option<f32>,
    val_horse: Option<f32>,
    val_cannon: Option<f32>,
    val_rook: Option<f32>,
    val_king: Option<f32>,

    hanging_piece_penalty: Option<i32>,
    king_exposed_cannon_penalty: Option<i32>,

    score_hash_move: Option<f32>,
    score_capture_base: Option<f32>,
    score_killer_move: Option<f32>,
    score_history_max: Option<f32>,

    pruning_method: Option<i32>,
    pruning_multiplier: Option<f32>,

    probcut_depth: Option<u8>,
    probcut_margin: Option<i32>,
    probcut_reduction: Option<u8>,

    singular_extension_min_depth: Option<u8>,
    singular_extension_margin: Option<i32>,

    mate_score: Option<i32>,

    tt_size_mb: Option<usize>,
}

impl EngineConfig {
    #[allow(dead_code)]
    pub fn load_from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        let json_config: EngineConfigJson = serde_json::from_str(json_str)?;
        let default = Self::default();

        Ok(Self {
            val_pawn: apply_scale(default.val_pawn, json_config.val_pawn),
            val_advisor: apply_scale(default.val_advisor, json_config.val_advisor),
            val_elephant: apply_scale(default.val_elephant, json_config.val_elephant),
            val_horse: apply_scale(default.val_horse, json_config.val_horse),
            val_cannon: apply_scale(default.val_cannon, json_config.val_cannon),
            val_rook: apply_scale(default.val_rook, json_config.val_rook),
            val_king: apply_scale(default.val_king, json_config.val_king),

            hanging_piece_penalty: json_config
                .hanging_piece_penalty
                .unwrap_or(default.hanging_piece_penalty),
            king_exposed_cannon_penalty: json_config
                .king_exposed_cannon_penalty
                .unwrap_or(default.king_exposed_cannon_penalty),

            score_hash_move: apply_scale(default.score_hash_move, json_config.score_hash_move),
            score_capture_base: apply_scale(
                default.score_capture_base,
                json_config.score_capture_base,
            ),
            score_killer_move: apply_scale(
                default.score_killer_move,
                json_config.score_killer_move,
            ),
            score_history_max: apply_scale(
                default.score_history_max,
                json_config.score_history_max,
            ),

            pruning_method: json_config.pruning_method.unwrap_or(default.pruning_method),
            pruning_multiplier: json_config
                .pruning_multiplier
                .unwrap_or(default.pruning_multiplier),

            probcut_depth: json_config.probcut_depth.unwrap_or(default.probcut_depth),
            probcut_margin: json_config.probcut_margin.unwrap_or(default.probcut_margin),
            probcut_reduction: json_config
                .probcut_reduction
                .unwrap_or(default.probcut_reduction),

            singular_extension_min_depth: json_config
                .singular_extension_min_depth
                .unwrap_or(default.singular_extension_min_depth),
            singular_extension_margin: json_config
                .singular_extension_margin
                .unwrap_or(default.singular_extension_margin),

            mate_score: json_config.mate_score.unwrap_or(default.mate_score),

            tt_size_mb: json_config.tt_size_mb.unwrap_or(default.tt_size_mb),
        })
    }
}

#[allow(dead_code)]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn apply_scale(default_val: i32, scale: Option<f32>) -> i32 {
    scale.map_or(default_val, |s| (default_val as f32 * s) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config_default() {
        let json = "{}";
        let config = EngineConfig::load_from_json(json).unwrap();
        assert_eq!(config.val_pawn, VAL_PAWN);
        assert_eq!(config.score_hash_move, 200_000);
    }

    #[test]
    fn test_load_config_scaled() {
        let json = r#"{
            "val_pawn": 1.5,
            "score_hash_move": 0.5
        }"#;
        let config = EngineConfig::load_from_json(json).unwrap();
        assert_eq!(config.val_pawn, (VAL_PAWN as f32 * 1.5) as i32);
        assert_eq!(config.score_hash_move, 100_000);
    }

    #[test]
    fn test_load_config_invalid_json() {
        let json = "{ invalid json }";
        let result = EngineConfig::load_from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_partial() {
        let json = r#"{
            "val_pawn": 2.0
        }"#;
        let config = EngineConfig::load_from_json(json).unwrap();
        // Pawn should be scaled
        assert_eq!(config.val_pawn, (VAL_PAWN as f32 * 2.0) as i32);
        // Others should be default
        assert_eq!(config.val_rook, VAL_ROOK);
    }

    #[test]
    fn test_load_config_full() {
        let json = r#"{
            "val_pawn": 1.1,
            "val_advisor": 1.2,
            "val_elephant": 1.3,
            "val_horse": 1.4,
            "val_cannon": 1.5,
            "val_rook": 1.6,
            "val_king": 1.7,
            "score_hash_move": 0.5,
            "score_capture_base": 0.6,
            "score_killer_move": 0.7,
            "score_history_max": 0.8,
            "pruning_method": 1,
            "pruning_multiplier": 2.5,
            "probcut_depth": 6,
            "probcut_margin": 250,
            "probcut_reduction": 3,
            "king_exposed_cannon_penalty": 30
        }"#;
        let config = EngineConfig::load_from_json(json).unwrap();

        assert_eq!(config.val_pawn, (VAL_PAWN as f32 * 1.1) as i32);
        assert_eq!(config.val_advisor, (VAL_ADVISOR as f32 * 1.2) as i32);
        assert_eq!(config.pruning_method, 1);
        assert!((config.pruning_multiplier - 2.5).abs() < f32::EPSILON);
        assert_eq!(config.probcut_depth, 6);
        assert_eq!(config.probcut_margin, 250);
        assert_eq!(config.probcut_reduction, 3);
        assert_eq!(config.king_exposed_cannon_penalty, 30);
    }

    #[test]
    fn test_load_config_edge_cases() {
        let json = r#"{
            "val_pawn": 0.0,
            "val_rook": -1.0
        }"#;
        let config = EngineConfig::load_from_json(json).unwrap();

        assert_eq!(config.val_pawn, 0);
        assert_eq!(config.val_rook, -VAL_ROOK);
    }

    #[test]
    fn test_deserialize_absolute_config() {
        let json = r#"{
            "val_pawn": 123,
            "val_king": 9999
        }"#;

        let config: EngineConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.val_pawn, 123);
        assert_eq!(config.val_king, 9999);
        // Check default values
        assert_eq!(config.val_rook, VAL_ROOK);
    }
}
