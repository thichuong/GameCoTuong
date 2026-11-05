use cotuong_core::engine::nnue::{
    get_black_king_palace_idx, get_red_king_palace_idx, halfkp_feature_idx_black,
    halfkp_feature_idx_red,
};
use cotuong_core::engine::Move;
use cotuong_core::logic::board::{Board, Color};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionSample {
    pub red_features: Vec<usize>,
    pub black_features: Vec<usize>,
    pub search_eval: f32, // Centipawns from Red's perspective
    pub game_result: f32, // 1.0 (Red win), 0.5 (Draw), 0.0 (Black win)
}

/// Extracts HalfKP active feature indices for Red and Black accumulator sides.
#[must_use]
pub fn extract_features(board: &Board) -> (Vec<usize>, Vec<usize>) {
    let mut red = Vec::with_capacity(32);
    let mut black = Vec::with_capacity(32);

    let k_red = get_red_king_palace_idx(board);
    let k_black = get_black_king_palace_idx(board);

    for (sq, maybe_piece) in board.grid.iter().enumerate() {
        if let Some(piece) = maybe_piece {
            if let Some(feat) =
                halfkp_feature_idx_red(k_red, piece.color == Color::Black, piece.piece_type, sq)
            {
                red.push(feat);
            }
            if let Some(feat) =
                halfkp_feature_idx_black(k_black, piece.color == Color::Red, piece.piece_type, sq)
            {
                black.push(feat);
            }
        }
    }

    (red, black)
}

/// Parses a result string ("1-0", "0-1", "1/2-1/2", "draw", "win", "loss") into a float score.
#[must_use]
pub fn parse_result_score(res_str: &str) -> f32 {
    let lower = res_str.trim().to_lowercase();
    if lower.contains("1-0") || lower == "1" || lower == "red_win" || lower == "win" {
        1.0
    } else if lower.contains("0-1") || lower == "0" || lower == "black_win" || lower == "loss" {
        0.0
    } else {
        0.5 // Default draw
    }
}

/// Loads training samples from a file (supporting game move sequences, FEN records, or JSONL).
pub fn load_dataset_file<P: AsRef<Path>>(path: P) -> Result<Vec<PositionSample>, String> {
    let path_ref = path.as_ref();
    let file = File::open(path_ref)
        .map_err(|e| format!("Failed to open dataset file {}: {e}", path_ref.display()))?;
    let reader = BufReader::new(file);

    let mut samples = Vec::new();
    let mut line_num = 0;

    for line_res in reader.lines() {
        line_num += 1;
        let line = match line_res {
            Ok(l) => l,
            Err(_) => continue,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }

        // 1. Try parsing as JSONL line if starts with '{'
        if trimmed.starts_with('{') {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if let Some(fen_str) = val.get("fen").and_then(|v| v.as_str()) {
                    if let Ok((board, _turn)) = Board::from_fen(fen_str) {
                        let (red_feats, black_feats) = extract_features(&board);
                        let result = val
                            .get("result")
                            .and_then(|v| v.as_f64())
                            .map_or(0.5, |f| f as f32);
                        let eval = val
                            .get("eval")
                            .and_then(|v| v.as_f64())
                            .map_or(0.0, |f| f as f32);
                        samples.push(PositionSample {
                            red_features: red_feats,
                            black_features: black_feats,
                            search_eval: eval,
                            game_result: result,
                        });
                        continue;
                    }
                }

                if let Some(moves_arr) = val.get("moves").and_then(|v| v.as_array()) {
                    let res_val = val
                        .get("result")
                        .and_then(|v| v.as_str())
                        .map_or(0.5, parse_result_score);

                    let mut board = Board::new();
                    let mut turn = Color::Red;

                    for m_val in moves_arr {
                        if let Some(m_str) = m_val.as_str() {
                            if let Ok(mv) = Move::from_uci(m_str) {
                                let (red_feats, black_feats) = extract_features(&board);
                                samples.push(PositionSample {
                                    red_features: red_feats,
                                    black_features: black_feats,
                                    search_eval: 0.0,
                                    game_result: res_val,
                                });
                                board.apply_move(&mv, turn);
                                turn = turn.opposite();
                            }
                        }
                    }
                    continue;
                }
            }
        }

        // 2. Try parsing as Move Game Record:
        // Format: "moves h2e2 h9g7 b0c2 i9h9 ... | result: 1-0"
        // Or "h2e2 h9g7 b0c2 i9h9 ... [1-0]"
        if trimmed.starts_with("moves ") || (trimmed.len() >= 4 && trimmed.contains(' ')) {
            let mut result_score = 0.5f32;
            let mut moves_part = trimmed;

            if let Some(pipe_idx) = trimmed.find('|') {
                moves_part = trimmed[..pipe_idx].trim();
                let meta_part = trimmed[pipe_idx + 1..].trim();
                if let Some(res_idx) = meta_part.find("result:") {
                    let sub = meta_part[res_idx + 7..].trim();
                    result_score = parse_result_score(sub);
                }
            } else if trimmed.ends_with(']') {
                if let Some(bracket_idx) = trimmed.rfind('[') {
                    let res_str = &trimmed[bracket_idx + 1..trimmed.len() - 1];
                    result_score = parse_result_score(res_str);
                    moves_part = trimmed[..bracket_idx].trim();
                }
            }

            if moves_part.starts_with("moves ") {
                moves_part = &moves_part[6..];
            }

            let mut board = Board::new();
            let mut turn = Color::Red;
            let mut valid_game = true;

            let move_tokens: Vec<&str> = moves_part.split_whitespace().collect();
            // Games should have at least 2 moves
            if move_tokens.len() >= 2 && move_tokens.iter().all(|t| t.len() == 4) {
                for token in move_tokens {
                    match Move::from_uci(token) {
                        Ok(mv) => {
                            let (red_feats, black_feats) = extract_features(&board);
                            samples.push(PositionSample {
                                red_features: red_feats,
                                black_features: black_feats,
                                search_eval: 0.0,
                                game_result: result_score,
                            });
                            board.apply_move(&mv, turn);
                            turn = turn.opposite();
                        }
                        Err(_) => {
                            valid_game = false;
                            break;
                        }
                    }
                }
                if valid_game {
                    continue;
                }
            }
        }

        // 3. Try parsing as FEN Record:
        // Format: "<FEN> | result: 1.0 | eval: 45"
        // Or "<FEN>"
        let fen_part = if let Some(pipe_idx) = trimmed.find('|') {
            &trimmed[..pipe_idx]
        } else {
            trimmed
        };

        if let Ok((board, _turn)) = Board::from_fen(fen_part.trim()) {
            let mut result_score = 0.5f32;
            let mut eval_cp = 0.0f32;

            if let Some(pipe_idx) = trimmed.find('|') {
                let rest = &trimmed[pipe_idx + 1..];
                for part in rest.split('|') {
                    let p = part.trim();
                    if let Some(res_str) = p.strip_prefix("result:") {
                        result_score = parse_result_score(res_str);
                    } else if let Some(eval_str) = p.strip_prefix("eval:") {
                        if let Ok(val) = eval_str.trim().parse::<f32>() {
                            eval_cp = val;
                        }
                    }
                }
            }

            let (red_feats, black_feats) = extract_features(&board);
            samples.push(PositionSample {
                red_features: red_feats,
                black_features: black_feats,
                search_eval: eval_cp,
                game_result: result_score,
            });
        }
    }

    if samples.is_empty() {
        return Err(format!(
            "No valid training samples could be extracted from '{}' (parsed {} lines)",
            path_ref.display(),
            line_num
        ));
    }

    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_result_score() {
        assert_eq!(parse_result_score("1-0"), 1.0);
        assert_eq!(parse_result_score("0-1"), 0.0);
        assert_eq!(parse_result_score("1/2-1/2"), 0.5);
        assert_eq!(parse_result_score("draw"), 0.5);
    }

    #[test]
    fn test_load_sample_master_games() {
        let samples = load_dataset_file("data/sample_master_games.txt")
            .or_else(|_| load_dataset_file("cotuong_trainer/data/sample_master_games.txt"))
            .expect("Should load sample master games");

        assert!(
            samples.len() >= 50,
            "Expected at least 50 samples from master games dataset, got {}",
            samples.len()
        );

        // Verify feature counts for valid xiangqi boards
        for sample in &samples {
            assert!(
                !sample.red_features.is_empty(),
                "Red features should not be empty"
            );
            assert!(
                !sample.black_features.is_empty(),
                "Black features should not be empty"
            );
            assert!(
                sample.game_result >= 0.0 && sample.game_result <= 1.0,
                "Result score must be between 0.0 and 1.0"
            );
        }
    }
}
