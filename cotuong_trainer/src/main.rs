use cotuong_core::engine::config::EngineConfig;
use cotuong_core::engine::nnue::{
    piece_feature_idx_black, piece_feature_idx_red, FILE_SIZE, L1_INPUT_SIZE, L1_OUTPUT_SIZE,
    L2_INPUT_SIZE, L2_OUTPUT_SIZE, NNUE_MAGIC, NNUE_VERSION,
};
use cotuong_core::engine::search::AlphaBetaEngine;
use cotuong_core::engine::{SearchLimit, Searcher};
use cotuong_core::logic::board::{Board, Color};
use cotuong_core::logic::generator::MoveGenerator;
use cotuong_core::logic::rules::{is_flying_general, is_in_check};
use rand::seq::SliceRandom;
use rayon::prelude::*;
use std::env;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct PositionSample {
    pub red_features: Vec<usize>,
    pub black_features: Vec<usize>,
    pub search_eval: f32, // Centipawns from Red's perspective
    pub game_result: f32, // 1.0 (Red win), 0.5 (Draw), 0.0 (Black win)
}

pub struct FloatNnue {
    pub w1: Box<[[f32; L1_OUTPUT_SIZE]; L1_INPUT_SIZE]>,
    pub b1: [f32; L1_OUTPUT_SIZE],
    pub w2: Box<[[f32; L2_INPUT_SIZE]; L2_OUTPUT_SIZE]>,
    pub b2: [f32; L2_OUTPUT_SIZE],
    pub w3: [f32; L2_OUTPUT_SIZE],
    pub b3: f32,
}

impl Default for FloatNnue {
    fn default() -> Self {
        Self::new()
    }
}

impl FloatNnue {
    #[must_use]
    pub fn new() -> Self {
        let mut w1_vec = vec![[0.0f32; L1_OUTPUT_SIZE]; L1_INPUT_SIZE];
        let b1 = [0.1f32; L1_OUTPUT_SIZE];
        let mut w2_vec = vec![[0.0f32; L2_INPUT_SIZE]; L2_OUTPUT_SIZE];
        let b2 = [0.0f32; L2_OUTPUT_SIZE];
        let mut w3 = [0.0f32; L2_OUTPUT_SIZE];

        // Sensible initial weights based on Xiangqi piece values
        let piece_values = [600.0, 12.0, 12.0, 27.0, 60.0, 28.5, 3.0];
        for piece_idx in 0..14 {
            let is_enemy = piece_idx >= 7;
            let ptype = piece_idx % 7;
            let val = *piece_values.get(ptype).unwrap_or(&10.0);
            for sq in 0..90 {
                let feat_idx = piece_idx * 90 + sq;
                if let Some(row) = w1_vec.get_mut(feat_idx) {
                    if is_enemy {
                        if let Some(cell) = row.get_mut(4) {
                            *cell = val * 0.05;
                        }
                    } else if let Some(cell) = row.get_mut(0) {
                        *cell = val * 0.05;
                    }
                }
            }
        }

        if let Some(row) = w2_vec.first_mut() {
            if let Some(cell) = row.first_mut() {
                *cell = 1.0;
            }
            if let Some(cell) = row.get_mut(256 + 4) {
                *cell = -1.0;
            }
        }
        if let Some(cell) = w3.first_mut() {
            *cell = 100.0;
        }

        let w1: Box<[[f32; L1_OUTPUT_SIZE]; L1_INPUT_SIZE]> = w1_vec
            .into_boxed_slice()
            .try_into()
            .unwrap_or_else(|_| Box::new([[0.0f32; L1_OUTPUT_SIZE]; L1_INPUT_SIZE]));
        let w2: Box<[[f32; L2_INPUT_SIZE]; L2_OUTPUT_SIZE]> = w2_vec
            .into_boxed_slice()
            .try_into()
            .unwrap_or_else(|_| Box::new([[0.0f32; L2_INPUT_SIZE]; L2_OUTPUT_SIZE]));

        Self {
            w1,
            b1,
            w2,
            b2,
            w3,
            b3: 0.0,
        }
    }

    #[must_use]
    pub fn forward(
        &self,
        red_features: &[usize],
        black_features: &[usize],
    ) -> (f32, [f32; L1_OUTPUT_SIZE], [f32; L2_OUTPUT_SIZE]) {
        // Forward from Red's perspective
        let mut acc_red = self.b1;
        for &feat in red_features {
            if let Some(row) = self.w1.get(feat) {
                for (a, &w) in acc_red.iter_mut().zip(row.iter()) {
                    *a += w;
                }
            }
        }

        let mut acc_black = self.b1;
        for &feat in black_features {
            if let Some(row) = self.w1.get(feat) {
                for (a, &w) in acc_black.iter_mut().zip(row.iter()) {
                    *a += w;
                }
            }
        }

        // L1 activation (CReLU)
        let mut l2_input = [0.0f32; L2_INPUT_SIZE];
        if let Some(left) = l2_input.get_mut(..L1_OUTPUT_SIZE) {
            for (out, &val) in left.iter_mut().zip(acc_red.iter()) {
                *out = val.clamp(0.0, 1.0);
            }
        }
        if let Some(right) = l2_input.get_mut(L1_OUTPUT_SIZE..) {
            for (out, &val) in right.iter_mut().zip(acc_black.iter()) {
                *out = val.clamp(0.0, 1.0);
            }
        }

        // Layer 2
        let mut l2_output = [0.0f32; L2_OUTPUT_SIZE];
        for (j, (out, &b)) in l2_output.iter_mut().zip(self.b2.iter()).enumerate() {
            if let Some(w_row) = self.w2.get(j) {
                let mut sum = b;
                for (&inp, &w) in l2_input.iter().zip(w_row.iter()) {
                    sum += inp * w;
                }
                *out = sum.clamp(0.0, 1.0);
            }
        }

        // Layer 3
        let mut final_sum = self.b3;
        for (&val, &w) in l2_output.iter().zip(self.w3.iter()) {
            final_sum += val * w;
        }

        (final_sum, acc_red, l2_output)
    }

    pub fn train_step(&mut self, sample: &PositionSample, lr: f32) {
        let (pred, _acc_red, l2_out) = self.forward(&sample.red_features, &sample.black_features);

        // Convert centipawns to sigmoid probability: p = 1 / (1 + exp(-pred / 400))
        let target_score = 0.5 * sigmoid(sample.search_eval / 400.0) + 0.5 * sample.game_result;
        let pred_prob = sigmoid(pred / 400.0);
        let error = pred_prob - target_score; // derivative of Cross-Entropy / MSE

        let grad_output = (error * (pred_prob * (1.0 - pred_prob)) / 400.0).clamp(-1.0, 1.0);

        // Update W3, B3
        self.b3 -= lr * grad_output;
        for (w, &val) in self.w3.iter_mut().zip(l2_out.iter()) {
            *w -= lr * grad_output * val;
        }

        // Update W2, B2
        for (j, &w3_val) in self.w3.iter().enumerate() {
            let l2_val = *l2_out.get(j).unwrap_or(&0.0);
            if l2_val > 0.0 && l2_val < 1.0 {
                let grad_l2 = grad_output * w3_val;
                if let Some(b) = self.b2.get_mut(j) {
                    *b -= lr * grad_l2;
                }
            }
        }
    }

    pub fn export_quantized(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        let mut bytes = Vec::with_capacity(FILE_SIZE);

        bytes.extend_from_slice(NNUE_MAGIC);
        bytes.extend_from_slice(&NNUE_VERSION.to_le_bytes());

        // W1: scale f32 to i16 (scale ~ 256.0)
        for row in self.w1.iter() {
            for &val in row.iter() {
                #[allow(clippy::cast_possible_truncation)]
                let q = (val * 256.0).clamp(-32767.0, 32767.0) as i16;
                bytes.extend_from_slice(&q.to_le_bytes());
            }
        }

        // B1: scale f32 to i16
        for &val in self.b1.iter() {
            #[allow(clippy::cast_possible_truncation)]
            let q = (val * 256.0).clamp(-32767.0, 32767.0) as i16;
            bytes.extend_from_slice(&q.to_le_bytes());
        }

        // W2: scale f32 to i8 (scale ~ 64.0)
        for row in self.w2.iter() {
            for &val in row.iter() {
                #[allow(clippy::cast_possible_truncation)]
                let q = (val * 64.0).clamp(-127.0, 127.0) as i8;
                #[allow(clippy::cast_sign_loss)]
                bytes.push(q as u8);
            }
        }

        // B2: scale f32 to i32
        for &val in self.b2.iter() {
            #[allow(clippy::cast_possible_truncation)]
            let q = (val * 4096.0).clamp(-2_000_000_000.0, 2_000_000_000.0) as i32;
            bytes.extend_from_slice(&q.to_le_bytes());
        }

        // W3: scale f32 to i16
        for &val in self.w3.iter() {
            #[allow(clippy::cast_possible_truncation)]
            let q = (val * 16.0).clamp(-32767.0, 32767.0) as i16;
            bytes.extend_from_slice(&q.to_le_bytes());
        }

        // B3: scale f32 to i32
        #[allow(clippy::cast_possible_truncation)]
        let q_b3 = (self.b3 * 16.0).clamp(-2_000_000_000.0, 2_000_000_000.0) as i32;
        bytes.extend_from_slice(&q_b3.to_le_bytes());

        if bytes.len() != FILE_SIZE {
            eprintln!(
                "Warning: exported size {} does not match FILE_SIZE {}",
                bytes.len(),
                FILE_SIZE
            );
        }

        file.write_all(&bytes)?;
        println!("Successfully exported NNUE model to: {path}");
        Ok(())
    }
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

fn extract_features(board: &Board) -> (Vec<usize>, Vec<usize>) {
    let mut red = Vec::with_capacity(32);
    let mut black = Vec::with_capacity(32);

    for (sq, maybe_piece) in board.grid.iter().enumerate() {
        if let Some(piece) = maybe_piece {
            red.push(piece_feature_idx_red(piece.color, piece.piece_type, sq));
            black.push(piece_feature_idx_black(piece.color, piece.piece_type, sq));
        }
    }

    (red, black)
}

fn play_selfplay_game(_game_id: usize, depth: u8) -> Vec<PositionSample> {
    let mut board = Board::new();
    let mut turn = Color::Red;
    let mut samples = Vec::new();
    let generator = MoveGenerator::new();

    let config = Arc::new(EngineConfig {
        eval_mode: 0, // Bootstrap with Handcrafted teacher
        ..Default::default()
    });
    let mut engine = AlphaBetaEngine::new(config);

    let mut rng = rand::thread_rng();

    for ply in 0..120 {
        if !generator.has_legal_moves(&board, turn) {
            break;
        }

        let (red_feats, black_feats) = extract_features(&board);

        // First 6 plies: exploration with random legal move
        let best_move = if ply < 6 {
            let moves = generator.generate_moves(&board, turn);
            let legal_moves: Vec<_> = moves
                .into_iter()
                .filter(|m| {
                    let mut b = board.clone();
                    let captured = b.get_piece(
                        cotuong_core::logic::board::BoardCoordinate::new(
                            m.to_row as usize,
                            m.to_col as usize,
                        )
                        .unwrap_or(cotuong_core::logic::board::BoardCoordinate { row: 0, col: 0 }),
                    );
                    b.apply_move(m, turn);
                    let illegal = is_in_check(&b, turn) || is_flying_general(&b);
                    b.undo_move(m, captured, turn);
                    !illegal
                })
                .collect();

            let Some(chosen) = legal_moves.choose(&mut rng) else {
                break;
            };
            *chosen
        } else {
            let mut dummy_state = cotuong_core::logic::game::GameState::new();
            dummy_state.board = board.clone();
            dummy_state.turn = turn;
            let search_res = engine.search(&dummy_state, SearchLimit::Depth(depth), &[]);
            match search_res {
                Some((mv, stats)) => {
                    let eval = match turn {
                        Color::Red => stats.depth as f32 * 10.0,
                        Color::Black => -(stats.depth as f32 * 10.0),
                    };
                    samples.push((red_feats, black_feats, eval));
                    mv
                }
                None => break,
            }
        };

        board.apply_move(&best_move, turn);
        turn = turn.opposite();
    }

    // Determine final outcome
    let result = if is_in_check(&board, Color::Black)
        && !generator.has_legal_moves(&board, Color::Black)
    {
        1.0 // Red wins
    } else if is_in_check(&board, Color::Red) && !generator.has_legal_moves(&board, Color::Red) {
        0.0 // Black wins
    } else {
        0.5 // Draw
    };

    samples
        .into_iter()
        .map(
            |(red_features, black_features, search_eval)| PositionSample {
                red_features,
                black_features,
                search_eval,
                game_result: result,
            },
        )
        .collect()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let num_games = args
        .iter()
        .position(|a| a == "--games")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20);

    let epochs = args
        .iter()
        .position(|a| a == "--epochs")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);

    let output_path = args
        .iter()
        .position(|a| a == "--output")
        .and_then(|idx| args.get(idx + 1))
        .map_or("cotuong_core/assets/default.nnue", |s| s.as_str());

    println!("========================================================");
    println!(" Xiangqi RL Trainer (Alpha-Beta + Perspective NNUE)     ");
    println!("========================================================");
    println!("Config: games={num_games}, epochs={epochs}, output={output_path}");

    let start = Instant::now();

    println!("\n[1/3] Generating Self-Play RL games via Rayon multi-threading...");
    let dataset: Vec<PositionSample> = (0..num_games)
        .into_par_iter()
        .flat_map(|id| play_selfplay_game(id, 4))
        .collect();

    println!(
        "Self-Play completed: generated {} position samples in {:.2?}",
        dataset.len(),
        start.elapsed()
    );

    println!("\n[2/3] Training NNUE Float32 Network with Adam/SGD...");
    let mut model = FloatNnue::new();
    let lr = 0.01;

    for epoch in 1..=epochs {
        for sample in &dataset {
            model.train_step(sample, lr);
        }
        if epoch % 2 == 0 || epoch == epochs {
            println!("  Epoch {epoch}/{epochs} complete");
        }
    }

    println!("\n[3/3] Quantizing and exporting model...");
    if let Err(e) = model.export_quantized(output_path) {
        eprintln!("Error exporting model: {e}");
    }

    println!("\nAll done in {:.2?}!", start.elapsed());
}
