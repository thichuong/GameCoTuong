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

    // Number of search threads (1..=16)
    pub threads: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            mate_score: 300_000,
            tt_size_mb: 512,
            nnue_weight: 100,
            threads: 1,
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
        assert_eq!(config.tt_size_mb, 512);
        assert_eq!(config.nnue_weight, 100);
        assert_eq!(config.threads, 1);
    }

    #[test]
    fn test_deserialize_partial_config() {
        let json = r#"{
            "mate_score": 40000,
            "nnue_weight": 110,
            "threads": 4
        }"#;

        let config: EngineConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.mate_score, 40_000);
        assert_eq!(config.nnue_weight, 110);
        assert_eq!(config.threads, 4);
    }
}
