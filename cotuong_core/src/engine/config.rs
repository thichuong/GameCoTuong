use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineConfig {
    // Checkmate scoring
    pub mate_score: i32, // Base score for checkmate (higher = stronger preference)

    // Transposition Table
    pub tt_size_mb: usize,

    // NNUE weight / scale percentage (50..=150)
    pub nnue_weight: i32,

    // Model type / precision architecture: "f32" (unquantized), "int16" (default), "int8" (compact)
    pub model_type: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            mate_score: 300_000,
            tt_size_mb: 256,
            nnue_weight: 100,
            model_type: "int16".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EngineConfig::default();
        assert_eq!(config.mate_score, 300_000);
        assert_eq!(config.nnue_weight, 100);
    }

    #[test]
    fn test_deserialize_partial_config() {
        let json = r#"{
            "mate_score": 40000,
            "nnue_weight": 110
        }"#;

        let config: EngineConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.mate_score, 40_000);
        assert_eq!(config.nnue_weight, 110);
    }
}
