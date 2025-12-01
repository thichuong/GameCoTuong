use crate::engine::eval_constants::{
    PST_CANNON, PST_HORSE, PST_PAWN, PST_ROOK, VAL_ADVISOR, VAL_CANNON, VAL_ELEPHANT, VAL_HORSE,
    VAL_KING, VAL_PAWN, VAL_ROOK,
};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    // Evaluation Parameters
    pub val_pawn: i32,
    pub val_advisor: i32,
    pub val_elephant: i32,
    pub val_horse: i32,
    pub val_cannon: i32,
    pub val_rook: i32,
    pub val_king: i32,

    pub pst_pawn: [[i32; 9]; 10],
    pub pst_horse: [[i32; 9]; 10],
    pub pst_cannon: [[i32; 9]; 10],
    pub pst_rook: [[i32; 9]; 10],

    // Search Parameters
    pub score_hash_move: i32,
    pub score_capture_base: i32,
    pub score_killer_move: i32,
    pub score_history_max: i32,
    pub pruning_method: i32, // 0: Dynamic Limiting, 1: LMR, 2: Both
    pub pruning_multiplier: f32,
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

            pst_pawn: PST_PAWN,
            pst_horse: PST_HORSE,
            pst_cannon: PST_CANNON,
            pst_rook: PST_ROOK,

            score_hash_move: 2_000_000,
            score_capture_base: 1_000_000,
            score_killer_move: 900_000,
            score_history_max: 800_000,
            pruning_method: 0, // Default to Dynamic Limiting
            pruning_multiplier: 1.0,
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

    pst_pawn: Option<[[f32; 9]; 10]>,
    pst_horse: Option<[[f32; 9]; 10]>,
    pst_cannon: Option<[[f32; 9]; 10]>,
    pst_rook: Option<[[f32; 9]; 10]>,

    score_hash_move: Option<f32>,
    score_capture_base: Option<f32>,
    score_killer_move: Option<f32>,
    score_history_max: Option<f32>,
    pruning_method: Option<i32>,
    pruning_multiplier: Option<f32>,
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

            pst_pawn: apply_scale_pst(&default.pst_pawn, json_config.pst_pawn.as_ref()),
            pst_horse: apply_scale_pst(&default.pst_horse, json_config.pst_horse.as_ref()),
            pst_cannon: apply_scale_pst(&default.pst_cannon, json_config.pst_cannon.as_ref()),
            pst_rook: apply_scale_pst(&default.pst_rook, json_config.pst_rook.as_ref()),

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
        })
    }
}

#[allow(dead_code)]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn apply_scale(default_val: i32, scale: Option<f32>) -> i32 {
    if let Some(s) = scale {
        (default_val as f32 * s) as i32
    } else {
        default_val
    }
}

#[allow(dead_code)]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn apply_scale_pst(
    default_pst: &[[i32; 9]; 10],
    scale_pst: Option<&[[f32; 9]; 10]>,
) -> [[i32; 9]; 10] {
    if let Some(s_pst) = scale_pst {
        let mut new_pst = [[0; 9]; 10];
        for (r, row) in new_pst.iter_mut().enumerate() {
            for (c, cell) in row.iter_mut().enumerate() {
                if let Some(def_row) = default_pst.get(r) {
                    if let Some(def_val) = def_row.get(c) {
                        if let Some(scale_row) = s_pst.get(r) {
                            if let Some(scale_val) = scale_row.get(c) {
                                *cell = (*def_val as f32 * *scale_val) as i32;
                            }
                        }
                    }
                }
            }
        }
        new_pst
    } else {
        *default_pst
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config_default() {
        let json = "{}";
        let config = EngineConfig::load_from_json(json).unwrap();
        assert_eq!(config.val_pawn, VAL_PAWN);
        assert_eq!(config.score_hash_move, 2_000_000);
    }

    #[test]
    fn test_load_config_scaled() {
        let json = r#"{
            "val_pawn": 1.5,
            "score_hash_move": 0.5
        }"#;
        let config = EngineConfig::load_from_json(json).unwrap();
        assert_eq!(config.val_pawn, (VAL_PAWN as f32 * 1.5) as i32);
        assert_eq!(config.score_hash_move, 1_000_000);
    }

    #[test]
    fn test_load_config_pst() {
        // Test scaling a PST
        // Default PST_PAWN[4][0] is 10.
        let json = r#"{
            "pst_pawn": [
                [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
                [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
                [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
                [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
                [2.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
                [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
                [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
                [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
                [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
                [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]
            ]
        }"#;
        let config = EngineConfig::load_from_json(json).unwrap();
        // Row 4, Col 0 is 10 in default. Scaled by 2.0 should be 20.
        assert_eq!(config.pst_pawn[4][0], 20);
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
            "pruning_multiplier": 2.5
        }"#;
        let config = EngineConfig::load_from_json(json).unwrap();

        assert_eq!(config.val_pawn, (VAL_PAWN as f32 * 1.1) as i32);
        assert_eq!(config.val_advisor, (VAL_ADVISOR as f32 * 1.2) as i32);
        assert_eq!(config.pruning_method, 1);
        assert!((config.pruning_multiplier - 2.5).abs() < f32::EPSILON);
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
}
