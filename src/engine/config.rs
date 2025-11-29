use crate::engine::eval_constants::*;
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

            pst_pawn: apply_scale_pst(default.pst_pawn, json_config.pst_pawn),
            pst_horse: apply_scale_pst(default.pst_horse, json_config.pst_horse),
            pst_cannon: apply_scale_pst(default.pst_cannon, json_config.pst_cannon),
            pst_rook: apply_scale_pst(default.pst_rook, json_config.pst_rook),

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
        })
    }
}

#[allow(dead_code)]
fn apply_scale(default_val: i32, scale: Option<f32>) -> i32 {
    if let Some(s) = scale {
        (default_val as f32 * s) as i32
    } else {
        default_val
    }
}

#[allow(dead_code)]
fn apply_scale_pst(
    default_pst: [[i32; 9]; 10],
    scale_pst: Option<[[f32; 9]; 10]>,
) -> [[i32; 9]; 10] {
    if let Some(s_pst) = scale_pst {
        let mut new_pst = [[0; 9]; 10];
        for r in 0..10 {
            for c in 0..9 {
                // For PST, the user said "scale 0 to 1".
                // But PST values can be negative (penalties).
                // And they are additive to the base value.
                // If the user provides a PST in JSON, is it a Multiplier map? Or absolute values?
                // "Bảng điểm vị trí Tốt: Tinh chỉnh điểm thưởng... Mảng 10x9"
                // "mặc định các tỉ số hiện tại là 1 khi nhập JSON"
                // If I have a PST value of 10, and JSON says 1.5, it becomes 15.
                // If I have -10, and JSON says 1.5, it becomes -15.
                // This seems consistent with "multiplier".
                // However, if the user provides a full 10x9 array in JSON, they probably want to SET the values, not multiply them?
                // But the instruction "scale 0 to 1 ... sau đó nhân lên trong code theo tỉ lệ" applies to "đầu vào là JSON".
                // If the input is a 10x9 array of floats, are those floats multipliers for the default PST?
                // Or are they normalized values where 1.0 = some max score?
                // Given "mặc định các tỉ số hiện tại là 1", it strongly implies multipliers.
                // So if the user wants to change a specific square from 10 to 20, they put 2.0 in that square.
                // If they want to change 0 to something else... 0 * x = 0.
                // This is a problem. Multipliers can't change 0 to non-zero.
                //
                // Alternative interpretation:
                // The "scale 0 to 1" applies to the *parameters* (scalars).
                // For arrays (PST), maybe the user provides the actual values but scaled?
                // Or maybe the user provides a single scalar for the *entire* PST?
                // No, the table says "pst_pawn ... [[i32;9];10]".
                // So the JSON input is a 2D array.
                // If I use multipliers, I can't tune 0 values.
                //
                // Let's look at the user request again.
                // "mặc định các tỉ số hiện tại là 1 khi nhập JSON"
                // This might refer to the SCALARS.
                // For PSTs, maybe they are just raw values?
                // But the user said "Để mô hình RL nên dùng scale 0 đến 1 cho đầu vào là JSON".
                // If I output a PST from RL, it will be 0-1 floats.
                // How do I map 0-1 float to a PST value like -10 or 20?
                // Maybe 0.5 is 0?
                //
                // Let's stick to the simplest interpretation that works for RL:
                // The JSON values are MULTIPLIERS for the *Magnitude*?
                // Or maybe the user provides the *weights* for the PST?
                //
                // Actually, "mặc định các tỉ số hiện tại là 1" is the key.
                // If I have a PST with values [10, 20, ...], and I want to keep it, I pass [1, 1, ...].
                // If I want to double the bonus, I pass [2, 2, ...].
                // If I want to make a 0 become 10... I can't.
                //
                // Maybe the user accepts that 0s remain 0s?
                // "pst_pawn ... Tinh chỉnh điểm thưởng khi qua sông."
                // "PST_HORSE ... Khuyến khích Mã chiếm trung lộ... Phạt nặng khi Mã ở biên"
                // The structure of the PST (where the bonuses/penalties are) is likely fixed by the "shape" of the default PST.
                // The RL only tunes the *magnitude* of these bonuses/penalties.
                // This makes sense for a "tuning" task where you don't want to learn the game from scratch, but tune existing heuristics.
                //
                // So I will implement it as element-wise multiplication.
                // `new_val = default_val * json_val`.
                new_pst[r][c] = (default_pst[r][c] as f32 * s_pst[r][c]) as i32;
            }
        }
        new_pst
    } else {
        default_pst
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
}
