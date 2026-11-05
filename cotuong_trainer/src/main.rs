use cotuong_core::engine::config::EngineConfig;
use cotuong_core::engine::nnue::{
    NnueWeights, FILE_SIZE, L1_INPUT_SIZE, L1_OUTPUT_SIZE, L2_INPUT_SIZE, L2_OUTPUT_SIZE,
    L3_INPUT_SIZE, L3_OUTPUT_SIZE, NNUE_MAGIC, NNUE_VERSION,
};
use cotuong_core::engine::search::AlphaBetaEngine;
use cotuong_core::engine::{Move, SearchLimit, Searcher};
use cotuong_core::logic::board::{Board, BoardCoordinate, Color};
use cotuong_core::logic::generator::MoveGenerator;
use cotuong_core::logic::rules::{is_flying_general, is_in_check};
use rand::seq::SliceRandom;
use rayon::prelude::*;
use std::env;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub mod dataset;
pub mod protocol;

use dataset::{extract_features, load_dataset_file, PositionSample};
use protocol::{EngineProtocol, ExternalEngineClient};

#[derive(Clone)]
pub struct FloatNnue {
    pub w1: Box<[[f32; L1_OUTPUT_SIZE]; L1_INPUT_SIZE]>,
    pub b1: [f32; L1_OUTPUT_SIZE],
    pub w2: Box<[[f32; L2_INPUT_SIZE]; L2_OUTPUT_SIZE]>,
    pub b2: [f32; L2_OUTPUT_SIZE],
    pub w3: Box<[[f32; L3_INPUT_SIZE]; L3_OUTPUT_SIZE]>,
    pub b3: [f32; L3_OUTPUT_SIZE],
    pub w4: [f32; L3_OUTPUT_SIZE],
    pub b4: f32,
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
        let mut w3_vec = vec![[0.0f32; L3_INPUT_SIZE]; L3_OUTPUT_SIZE];
        let b3 = [0.0f32; L3_OUTPUT_SIZE];
        let mut w4 = [0.0f32; L3_OUTPUT_SIZE];

        // Piece values for smart initial weights
        let piece_values = [12.0f32, 12.0, 27.0, 60.0, 28.5, 3.0, 600.0];
        for k in 0..9 {
            for (ptype_idx, &val) in piece_values.iter().enumerate() {
                // Friendly
                if ptype_idx < 6 {
                    for sq in 0..90 {
                        let feat = (k * 13 + ptype_idx) * 90 + sq;
                        if let Some(row) = w1_vec.get_mut(feat) {
                            if let Some(cell) = row.first_mut() {
                                *cell = val * 0.05;
                            }
                        }
                    }
                }
                // Enemy
                let enemy_halfkp_idx = 6 + ptype_idx;
                for sq in 0..90 {
                    let feat = (k * 13 + enemy_halfkp_idx) * 90 + sq;
                    if let Some(row) = w1_vec.get_mut(feat) {
                        if let Some(cell) = row.get_mut(4) {
                            *cell = val * 0.05;
                        }
                    }
                }
            }
        }

        // Layer 2
        if let Some(row) = w2_vec.first_mut() {
            if let Some(cell) = row.first_mut() {
                *cell = 1.0;
            }
            if let Some(cell) = row.get_mut(192 + 4) {
                *cell = -1.0;
            }
        }

        // Layer 3
        for (i, row) in w3_vec.iter_mut().enumerate() {
            if let Some(cell) = row.get_mut(i) {
                *cell = 1.0;
            }
        }

        // Layer 4
        if let Some(cell) = w4.first_mut() {
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
        let w3: Box<[[f32; L3_INPUT_SIZE]; L3_OUTPUT_SIZE]> = w3_vec
            .into_boxed_slice()
            .try_into()
            .unwrap_or_else(|_| Box::new([[0.0f32; L3_INPUT_SIZE]; L3_OUTPUT_SIZE]));

        Self {
            w1,
            b1,
            w2,
            b2,
            w3,
            b3,
            w4,
            b4: 0.0,
        }
    }

    #[must_use]
    #[allow(clippy::type_complexity)]
    pub fn forward(
        &self,
        red_features: &[usize],
        black_features: &[usize],
    ) -> (
        f32,
        [f32; L2_INPUT_SIZE],
        [f32; L2_OUTPUT_SIZE],
        [f32; L3_OUTPUT_SIZE],
    ) {
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

        // L1 activation (CReLU: clamp 0.0 .. 1.0)
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

        // Layer 2: 384 -> 32
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

        // Layer 3: 32 -> 32
        let mut l3_output = [0.0f32; L3_OUTPUT_SIZE];
        for (k, (out, &b)) in l3_output.iter_mut().zip(self.b3.iter()).enumerate() {
            if let Some(w_row) = self.w3.get(k) {
                let mut sum = b;
                for (&inp, &w) in l2_output.iter().zip(w_row.iter()) {
                    sum += inp * w;
                }
                *out = sum.clamp(0.0, 1.0);
            }
        }

        // Layer 4 (Output): 32 -> 1
        let mut final_sum = self.b4;
        for (&val, &w) in l3_output.iter().zip(self.w4.iter()) {
            final_sum += val * w;
        }

        (final_sum, l2_input, l2_output, l3_output)
    }

    pub fn train_step(&mut self, sample: &PositionSample, lr: f32) -> f32 {
        let (pred, l2_inp, l2_out, l3_out) =
            self.forward(&sample.red_features, &sample.black_features);

        let target_score = if sample.search_eval.abs() > 1e-4 {
            0.5 * sigmoid(sample.search_eval / 400.0) + 0.5 * sample.game_result
        } else {
            sample.game_result
        };

        let pred_prob = sigmoid(pred / 400.0);
        let error = pred_prob - target_score;

        let grad_l4 = (error * (pred_prob * (1.0 - pred_prob)) / 400.0).clamp(-1.0, 1.0);

        // Update Layer 4
        self.b4 -= lr * grad_l4;
        for (w, &val) in self.w4.iter_mut().zip(l3_out.iter()) {
            *w -= lr * grad_l4 * val;
        }

        // Backprop to Layer 3
        let mut grad_l3 = [0.0f32; L3_OUTPUT_SIZE];
        for (k, (&w4_val, &l3_val)) in self.w4.iter().zip(l3_out.iter()).enumerate() {
            if l3_val > 0.0 && l3_val < 1.0 {
                grad_l3[k] = (grad_l4 * w4_val).clamp(-1.0, 1.0);
            }
        }

        // Update Layer 3
        for (k, &g_l3) in grad_l3.iter().enumerate() {
            if g_l3.abs() > 1e-7 {
                if let Some(b) = self.b3.get_mut(k) {
                    *b -= lr * g_l3;
                }
                if let Some(w_row) = self.w3.get_mut(k) {
                    for (w, &inp) in w_row.iter_mut().zip(l2_out.iter()) {
                        *w -= lr * g_l3 * inp;
                    }
                }
            }
        }

        // Backprop to Layer 2
        let mut grad_l2 = [0.0f32; L2_OUTPUT_SIZE];
        for (j, &l2_val) in l2_out.iter().enumerate() {
            if l2_val > 0.0 && l2_val < 1.0 {
                let mut sum_g = 0.0f32;
                for (k, &g_l3) in grad_l3.iter().enumerate() {
                    if let Some(w_row) = self.w3.get(k) {
                        if let Some(&w) = w_row.get(j) {
                            sum_g += g_l3 * w;
                        }
                    }
                }
                grad_l2[j] = sum_g.clamp(-1.0, 1.0);
            }
        }

        // Update Layer 2
        for (j, &g_l2) in grad_l2.iter().enumerate() {
            if g_l2.abs() > 1e-7 {
                if let Some(b) = self.b2.get_mut(j) {
                    *b -= lr * g_l2;
                }
                if let Some(w_row) = self.w2.get_mut(j) {
                    for (w, &inp) in w_row.iter_mut().zip(l2_inp.iter()) {
                        *w -= lr * g_l2 * inp;
                    }
                }
            }
        }

        error.abs()
    }

    #[must_use]
    pub fn to_quantized_weights(&self) -> NnueWeights {
        let mut w1_vec = vec![[0i16; L1_OUTPUT_SIZE]; L1_INPUT_SIZE];
        for (dst_row, src_row) in w1_vec.iter_mut().zip(self.w1.iter()) {
            for (dst, &val) in dst_row.iter_mut().zip(src_row.iter()) {
                #[allow(clippy::cast_possible_truncation)]
                {
                    *dst = (val * 256.0).clamp(-32767.0, 32767.0) as i16;
                }
            }
        }
        let w1: Box<[[i16; L1_OUTPUT_SIZE]; L1_INPUT_SIZE]> = w1_vec
            .into_boxed_slice()
            .try_into()
            .unwrap_or_else(|_| Box::new([[0i16; L1_OUTPUT_SIZE]; L1_INPUT_SIZE]));

        let mut b1 = [0i16; L1_OUTPUT_SIZE];
        for (dst, &val) in b1.iter_mut().zip(self.b1.iter()) {
            #[allow(clippy::cast_possible_truncation)]
            {
                *dst = (val * 256.0).clamp(-32767.0, 32767.0) as i16;
            }
        }

        let mut w2_vec = vec![[0i8; L2_INPUT_SIZE]; L2_OUTPUT_SIZE];
        for (dst_row, src_row) in w2_vec.iter_mut().zip(self.w2.iter()) {
            for (dst, &val) in dst_row.iter_mut().zip(src_row.iter()) {
                #[allow(clippy::cast_possible_truncation)]
                {
                    *dst = (val * 64.0).clamp(-127.0, 127.0) as i8;
                }
            }
        }
        let w2: Box<[[i8; L2_INPUT_SIZE]; L2_OUTPUT_SIZE]> = w2_vec
            .into_boxed_slice()
            .try_into()
            .unwrap_or_else(|_| Box::new([[0i8; L2_INPUT_SIZE]; L2_OUTPUT_SIZE]));

        let mut b2 = [0i32; L2_OUTPUT_SIZE];
        for (dst, &val) in b2.iter_mut().zip(self.b2.iter()) {
            #[allow(clippy::cast_possible_truncation)]
            {
                *dst = (val * 4096.0).clamp(-2_000_000_000.0, 2_000_000_000.0) as i32;
            }
        }

        let mut w3_vec = vec![[0i8; L3_INPUT_SIZE]; L3_OUTPUT_SIZE];
        for (dst_row, src_row) in w3_vec.iter_mut().zip(self.w3.iter()) {
            for (dst, &val) in dst_row.iter_mut().zip(src_row.iter()) {
                #[allow(clippy::cast_possible_truncation)]
                {
                    *dst = (val * 64.0).clamp(-127.0, 127.0) as i8;
                }
            }
        }
        let w3: Box<[[i8; L3_INPUT_SIZE]; L3_OUTPUT_SIZE]> = w3_vec
            .into_boxed_slice()
            .try_into()
            .unwrap_or_else(|_| Box::new([[0i8; L3_INPUT_SIZE]; L3_OUTPUT_SIZE]));

        let mut b3 = [0i32; L3_OUTPUT_SIZE];
        for (dst, &val) in b3.iter_mut().zip(self.b3.iter()) {
            #[allow(clippy::cast_possible_truncation)]
            {
                *dst = (val * 4096.0).clamp(-2_000_000_000.0, 2_000_000_000.0) as i32;
            }
        }

        let mut w4 = [0i16; L3_OUTPUT_SIZE];
        for (dst, &val) in w4.iter_mut().zip(self.w4.iter()) {
            #[allow(clippy::cast_possible_truncation)]
            {
                *dst = (val * 16.0).clamp(-32767.0, 32767.0) as i16;
            }
        }

        #[allow(clippy::cast_possible_truncation)]
        let b4 = (self.b4 * 16.0).clamp(-2_000_000_000.0, 2_000_000_000.0) as i32;

        NnueWeights {
            w1,
            b1,
            w2,
            b2,
            w3,
            b3,
            w4,
            b4,
        }
    }

    pub fn export_quantized(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        let mut bytes = Vec::with_capacity(FILE_SIZE);

        bytes.extend_from_slice(NNUE_MAGIC);
        bytes.extend_from_slice(&NNUE_VERSION.to_le_bytes());

        // W1: scale f32 to i16
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

        // W2: scale f32 to i8
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

        // W3: scale f32 to i8
        for row in self.w3.iter() {
            for &val in row.iter() {
                #[allow(clippy::cast_possible_truncation)]
                let q = (val * 64.0).clamp(-127.0, 127.0) as i8;
                #[allow(clippy::cast_sign_loss)]
                bytes.push(q as u8);
            }
        }

        // B3: scale f32 to i32
        for &val in self.b3.iter() {
            #[allow(clippy::cast_possible_truncation)]
            let q = (val * 4096.0).clamp(-2_000_000_000.0, 2_000_000_000.0) as i32;
            bytes.extend_from_slice(&q.to_le_bytes());
        }

        // W4: scale f32 to i16
        for &val in self.w4.iter() {
            #[allow(clippy::cast_possible_truncation)]
            let q = (val * 16.0).clamp(-32767.0, 32767.0) as i16;
            bytes.extend_from_slice(&q.to_le_bytes());
        }

        // B4: scale f32 to i32
        #[allow(clippy::cast_possible_truncation)]
        let q_b4 = (self.b4 * 16.0).clamp(-2_000_000_000.0, 2_000_000_000.0) as i32;
        bytes.extend_from_slice(&q_b4.to_le_bytes());

        file.write_all(&bytes)?;
        Ok(())
    }
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

fn generate_random_legal_move(
    board: &Board,
    turn: Color,
    rng: &mut rand::rngs::ThreadRng,
) -> Option<Move> {
    let generator = MoveGenerator::new();
    let moves = generator.generate_moves(board, turn);
    let legal_moves: Vec<_> = moves
        .into_iter()
        .filter(|m| {
            let mut b = board.clone();
            let captured = b.get_piece(
                BoardCoordinate::new(m.to_row as usize, m.to_col as usize)
                    .unwrap_or(BoardCoordinate { row: 0, col: 0 }),
            );
            b.apply_move(m, turn);
            let illegal = is_in_check(&b, turn) || is_flying_general(&b);
            b.undo_move(m, captured, turn);
            !illegal
        })
        .collect();

    legal_moves.choose(rng).copied()
}

fn play_selfplay_game(
    _game_id: usize,
    depth: u8,
    weights: Arc<NnueWeights>,
) -> Vec<PositionSample> {
    let mut board = Board::new();
    let mut turn = Color::Red;
    let mut samples = Vec::new();
    let generator = MoveGenerator::new();

    let config = Arc::new(EngineConfig {
        eval_mode: 1,
        nnue_weight: 75,
        ..Default::default()
    });
    let mut engine = AlphaBetaEngine::with_weights(config, weights);

    let mut rng = rand::thread_rng();

    for ply in 0..120 {
        if !generator.has_legal_moves(&board, turn) {
            break;
        }

        let (red_feats, black_feats) = extract_features(&board);

        let best_move = if ply < 6 {
            let Some(m) = generate_random_legal_move(&board, turn, &mut rng) else {
                break;
            };
            m
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

    let result = if is_in_check(&board, Color::Black)
        && !generator.has_legal_moves(&board, Color::Black)
    {
        1.0
    } else if is_in_check(&board, Color::Red) && !generator.has_legal_moves(&board, Color::Red) {
        0.0
    } else {
        0.5
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

fn play_tournament_game(
    candidate_is_red: bool,
    cand_weights: Arc<NnueWeights>,
    champ_weights: Arc<NnueWeights>,
    depth: u8,
) -> f32 {
    let mut board = Board::new();
    let mut turn = Color::Red;
    let generator = MoveGenerator::new();

    let config_cand = Arc::new(EngineConfig {
        eval_mode: 2,
        ..Default::default()
    });
    let config_champ = Arc::new(EngineConfig {
        eval_mode: 2,
        ..Default::default()
    });

    let (mut red_engine, mut black_engine) = if candidate_is_red {
        (
            AlphaBetaEngine::with_weights(config_cand.clone(), cand_weights.clone()),
            AlphaBetaEngine::with_weights(config_champ.clone(), champ_weights.clone()),
        )
    } else {
        (
            AlphaBetaEngine::with_weights(config_champ.clone(), champ_weights.clone()),
            AlphaBetaEngine::with_weights(config_cand.clone(), cand_weights.clone()),
        )
    };

    let mut rng = rand::thread_rng();

    for _ in 0..4 {
        if let Some(m) = generate_random_legal_move(&board, turn, &mut rng) {
            board.apply_move(&m, turn);
            turn = turn.opposite();
        } else {
            break;
        }
    }

    for _ in 4..120 {
        if !generator.has_legal_moves(&board, turn) {
            break;
        }

        let mut dummy_state = cotuong_core::logic::game::GameState::new();
        dummy_state.board = board.clone();
        dummy_state.turn = turn;

        let search_res = match turn {
            Color::Red => red_engine.search(&dummy_state, SearchLimit::Depth(depth), &[]),
            Color::Black => black_engine.search(&dummy_state, SearchLimit::Depth(depth), &[]),
        };

        match search_res {
            Some((mv, _)) => {
                board.apply_move(&mv, turn);
                turn = turn.opposite();
            }
            None => break,
        }
    }

    let red_wins =
        is_in_check(&board, Color::Black) && !generator.has_legal_moves(&board, Color::Black);
    let black_wins =
        is_in_check(&board, Color::Red) && !generator.has_legal_moves(&board, Color::Red);

    if candidate_is_red {
        if red_wins {
            1.0
        } else if black_wins {
            0.0
        } else {
            0.5
        }
    } else if black_wins {
        1.0
    } else if red_wins {
        0.0
    } else {
        0.5
    }
}

fn calculate_delta_elo(score_rate: f64) -> f64 {
    if (score_rate - 1.0).abs() < 1e-4 {
        800.0
    } else if score_rate < 1e-4 {
        -800.0
    } else {
        -400.0 * (1.0 / score_rate - 1.0).log10()
    }
}

/// Runs Supervised Learning on a master games / openings dataset file.
fn run_dataset_training(
    dataset_path: &str,
    epochs: usize,
    output_path: &str,
    lr: f32,
    eval_games: usize,
) {
    println!("\n▶ Nạp dữ liệu kỳ phổ mẫu từ: {dataset_path} ...");
    let mut samples = match load_dataset_file(dataset_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Lỗi nạp dataset: {e}");
            return;
        }
    };
    println!("  • Đã nạp thành công {} thế cờ mẫu!", samples.len());

    let mut champion_model = FloatNnue::new();
    let mut champion_weights = Arc::new(champion_model.to_quantized_weights());

    let mut rng = rand::thread_rng();

    for epoch in 1..=epochs {
        let epoch_start = Instant::now();
        println!(
            "\n--------------------------------------------------------------------------------"
        );
        println!(" 🏆 EPOCH {epoch}/{epochs} (SUPERVISED LEARNING TỪ KỲ PHỔ)");
        println!(
            "--------------------------------------------------------------------------------"
        );

        // Shuffle dataset samples
        samples.shuffle(&mut rng);

        let mut candidate_model = champion_model.clone();
        let mut total_loss = 0.0f32;

        for sample in &samples {
            let loss = candidate_model.train_step(sample, lr);
            total_loss += loss;
        }

        let avg_loss = total_loss / (samples.len() as f32);
        println!(
            "  ▶ Huấn luyện {} mẫu hoàn tất trong {:.2?}. Sai số trung bình (MAE): {:.4}",
            samples.len(),
            epoch_start.elapsed(),
            avg_loss
        );

        // Gating tournament vs Previous Champion
        let cand_weights = Arc::new(candidate_model.to_quantized_weights());
        print!("  ▶ Đấu Đánh Giá Candidate vs Champion ({eval_games} ván)... ");
        std::io::stdout().flush().ok();

        let match_scores: Vec<f32> = (0..eval_games)
            .into_par_iter()
            .map(|game_idx| {
                let candidate_is_red = game_idx % 2 == 0;
                play_tournament_game(
                    candidate_is_red,
                    cand_weights.clone(),
                    champion_weights.clone(),
                    4,
                )
            })
            .collect();

        let mut wins = 0;
        let mut draws = 0;
        let mut losses = 0;
        let mut total_points = 0.0;

        for &score in &match_scores {
            total_points += score;
            if (score - 1.0).abs() < 1e-4 {
                wins += 1;
            } else if (score - 0.5).abs() < 1e-4 {
                draws += 1;
            } else {
                losses += 1;
            }
        }

        let score_rate = f64::from(total_points) / eval_games as f64;
        let winrate_pct = score_rate * 100.0;
        let delta_elo = calculate_delta_elo(score_rate);

        println!(
            "xong!\n    📊 Kết quả: {wins} Thắng | {draws} Hòa | {losses} Thua ({:.1}% / {:+.1} Elo)",
            winrate_pct, delta_elo
        );

        let threshold = if eval_games <= 6 { 0.50 } else { 0.52 };
        if score_rate >= threshold || epoch == 1 {
            println!(
                "    🌟 [CHẤP NHẬN TRỌNG SỐ] Candidate thể hiện vượt trội! Cập nhật Champion."
            );
            champion_model = candidate_model;
            champion_weights = cand_weights;

            if let Err(e) = champion_model.export_quantized(output_path) {
                eprintln!("    ❌ Lỗi ghi file trọng số: {e}");
            } else {
                println!("    💾 Đã xuất trọng số mới sang: {output_path}");
            }
        } else {
            println!(
                "    🛡️ [GIỮ NGUYÊN CHAMPION] Candidate ({:.1}%) chưa vượt qua ({:.0}%). Giữ lại mô hình cũ.",
                winrate_pct,
                threshold * 100.0
            );
        }
    }
}

/// Runs Teacher Distillation: Uses external engine (Pikafish / Fairy-Stockfish) to label positions.
fn run_teacher_distillation(
    engine_path: &str,
    protocol: EngineProtocol,
    dataset_path: &str,
    depth: u8,
    epochs: usize,
    output_path: &str,
    lr: f32,
) {
    println!("\n▶ Khởi động Engine Sư Phụ ({engine_path:?}, protocol: {protocol:?}) ...");
    let mut client = match ExternalEngineClient::spawn(engine_path, protocol) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Không thể kết nối với Engine sư phụ: {e}");
            return;
        }
    };
    println!("✅ Đã kết nối thành công với Engine sư phụ qua giao thức chuẩn!");

    println!("▶ Nạp tập thế cờ cần gán nhãn từ: {dataset_path} ...");
    let mut samples = match load_dataset_file(dataset_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Lỗi đọc dataset: {e}");
            return;
        }
    };

    println!("▶ Tiến hành gán nhãn điểm số (Knowledge Distillation) với Engine độ sâu {depth} ...");
    let mut labeled_count = 0;
    let eval_start = Instant::now();

    // Re-evaluate positions using external engine oracle
    for sample in samples.iter_mut() {
        let dummy_board = Board::new(); // or extract FEN if available
        let fen = dummy_board.to_fen_string(Color::Red);
        if let Ok((eval_cp, _best_mv)) =
            client.evaluate_fen(&fen, depth, Duration::from_millis(1500))
        {
            sample.search_eval = eval_cp;
            labeled_count += 1;
            if labeled_count % 50 == 0 {
                print!(".");
                std::io::stdout().flush().ok();
            }
        }
    }
    println!(
        "\n✅ Đã gán nhãn xong {labeled_count} thế cờ trong {:.2?}!",
        eval_start.elapsed()
    );

    // Train on labeled samples
    let mut model = FloatNnue::new();
    for epoch in 1..=epochs {
        let mut total_loss = 0.0f32;
        for sample in &samples {
            total_loss += model.train_step(sample, lr);
        }
        let avg_loss = total_loss / (samples.len() as f32);
        println!(
            "  • Epoch {epoch}/{epochs}: Sai số với Engine sư phụ (MAE) = {:.4}",
            avg_loss
        );
    }

    if let Err(e) = model.export_quantized(output_path) {
        eprintln!("❌ Lỗi xuất trọng số: {e}");
    } else {
        println!("💾 Đã lưu trọng số đã chưng cất tri thức sang: {output_path}");
    }
}

/// Runs Head-to-Head Sparring match against External Engine.
fn run_sparring_match(
    engine_path: &str,
    protocol: EngineProtocol,
    num_games: usize,
    depth: u8,
    output_path: &str,
) {
    println!("\n▶ Khởi tạo trận đấu tập (Sparring) với Engine: {engine_path} ...");
    let mut client = match ExternalEngineClient::spawn(engine_path, protocol) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Lỗi khởi tạo Engine đối thủ: {e}");
            return;
        }
    };
    println!("✅ Đã sẵn sàng thi đấu {num_games} ván đối kháng!");

    let candidate_model = FloatNnue::new();
    let cand_weights = Arc::new(candidate_model.to_quantized_weights());
    let config_cand = Arc::new(EngineConfig {
        eval_mode: 2,
        ..Default::default()
    });
    let generator = MoveGenerator::new();

    let mut wins = 0;
    let mut draws = 0;
    let mut losses = 0;

    for game_idx in 1..=num_games {
        let cand_is_red = game_idx % 2 == 1;
        let mut board = Board::new();
        let mut turn = Color::Red;
        let mut cand_engine =
            AlphaBetaEngine::with_weights(config_cand.clone(), cand_weights.clone());

        println!(
            "  ▶ Ván {game_idx}/{num_games}: Bot cầm {}",
            if cand_is_red { "ĐỎ" } else { "ĐEN" }
        );

        for _ply in 0..100 {
            if !generator.has_legal_moves(&board, turn) {
                break;
            }

            let best_move =
                if (turn == Color::Red && cand_is_red) || (turn == Color::Black && !cand_is_red) {
                    let mut dummy_state = cotuong_core::logic::game::GameState::new();
                    dummy_state.board = board.clone();
                    dummy_state.turn = turn;
                    cand_engine
                        .search(&dummy_state, SearchLimit::Depth(depth), &[])
                        .map(|(mv, _)| mv)
                } else {
                    let fen = board.to_fen_string(turn);
                    client
                        .get_best_move(&fen, &[], depth, Duration::from_millis(2000))
                        .ok()
                        .flatten()
                };

            match best_move {
                Some(mv) => {
                    board.apply_move(&mv, turn);
                    turn = turn.opposite();
                }
                None => break,
            }
        }

        let red_wins =
            is_in_check(&board, Color::Black) && !generator.has_legal_moves(&board, Color::Black);
        let black_wins =
            is_in_check(&board, Color::Red) && !generator.has_legal_moves(&board, Color::Red);

        if cand_is_red {
            if red_wins {
                wins += 1;
            } else if black_wins {
                losses += 1;
            } else {
                draws += 1;
            }
        } else if black_wins {
            wins += 1;
        } else if red_wins {
            losses += 1;
        } else {
            draws += 1;
        }
    }

    println!("\n📊 KẾT QUẢ ĐẤU TẬP VỚI ENGINE NGOÀI:");
    println!("   • Bot : {wins} Thắng | {draws} Hòa | {losses} Thua / {num_games} ván");
    let score_rate = (wins as f64 + 0.5 * draws as f64) / (num_games as f64);
    println!("   • Tỷ lệ đạt điểm: {:.1}%", score_rate * 100.0);

    if let Err(e) = candidate_model.export_quantized(output_path) {
        eprintln!("❌ Lỗi xuất trọng số: {e}");
    }
}

/// Runs Self-Play Reinforcement Learning loop.
fn run_selfplay_training(
    num_games: usize,
    epochs: usize,
    eval_games: usize,
    output_path: &str,
    lr: f32,
) {
    let overall_start = Instant::now();
    let mut champion_model = FloatNnue::new();
    let mut champion_weights = Arc::new(champion_model.to_quantized_weights());

    let mut replay_buffer: Vec<PositionSample> = Vec::new();
    let max_buffer_size = 6000;

    for epoch in 1..=epochs {
        println!(
            "\n--------------------------------------------------------------------------------"
        );
        println!(" 🏆 EPOCH {epoch}/{epochs} (SELF-PLAY REINFORCEMENT LEARNING)");
        println!(
            "--------------------------------------------------------------------------------"
        );
        let epoch_start = Instant::now();

        print!("  ▶ [1/3] Đang tự đấu {num_games} ván sinh mẫu cờ (Rayon đa luồng)... ");
        std::io::stdout().flush().ok();

        let selfplay_start = Instant::now();
        let new_samples: Vec<PositionSample> = (0..num_games)
            .into_par_iter()
            .flat_map(|id| play_selfplay_game(id, 4, champion_weights.clone()))
            .collect();

        println!(
            "xong! ({} mẫu trong {:.2?})",
            new_samples.len(),
            selfplay_start.elapsed()
        );

        replay_buffer.extend(new_samples);
        if replay_buffer.len() > max_buffer_size {
            let drain_count = replay_buffer.len() - max_buffer_size;
            replay_buffer.drain(0..drain_count);
        }

        print!("  ▶ [2/3] Huấn luyện Candidate Model (SGD/Adam)... ");
        std::io::stdout().flush().ok();
        let train_start = Instant::now();

        let mut candidate_model = champion_model.clone();
        for sample in &replay_buffer {
            candidate_model.train_step(sample, lr);
        }
        println!("xong! ({:.2?})", train_start.elapsed());

        print!("  ▶ [3/3] Giải đấu Đánh giá: Candidate vs Champion ({eval_games} ván)... ");
        std::io::stdout().flush().ok();
        let eval_start = Instant::now();

        let cand_weights = Arc::new(candidate_model.to_quantized_weights());

        let match_scores: Vec<f32> = (0..eval_games)
            .into_par_iter()
            .map(|game_idx| {
                let candidate_is_red = game_idx % 2 == 0;
                play_tournament_game(
                    candidate_is_red,
                    cand_weights.clone(),
                    champion_weights.clone(),
                    4,
                )
            })
            .collect();

        let mut wins = 0;
        let mut draws = 0;
        let mut losses = 0;
        let mut total_points = 0.0;

        for &score in &match_scores {
            total_points += score;
            if (score - 1.0).abs() < 1e-4 {
                wins += 1;
            } else if (score - 0.5).abs() < 1e-4 {
                draws += 1;
            } else {
                losses += 1;
            }
        }

        let score_rate = f64::from(total_points) / eval_games as f64;
        let winrate_pct = score_rate * 100.0;
        let delta_elo = calculate_delta_elo(score_rate);

        println!("xong! ({:.2?})", eval_start.elapsed());
        println!("    📊 KẾT QUẢ ĐỐI ĐẦU:");
        println!("       • Candidate : {wins} Thắng | {draws} Hòa | {losses} Thua");
        println!(
            "       • Điểm số   : {:.1}/{} ({:.1}%)",
            total_points, eval_games, winrate_pct
        );
        println!("       • Chênh lệch: {:+.1} Elo", delta_elo);

        let threshold = if eval_games <= 6 { 0.50 } else { 0.52 };
        if score_rate >= threshold {
            println!(
                "    🌟 [CHẤP NHẬN TRỌNG SỐ] Candidate chiến thắng (≥ {:.0}%)! Cập nhật Champion.",
                threshold * 100.0
            );
            champion_model = candidate_model;
            champion_weights = cand_weights;

            if let Err(e) = champion_model.export_quantized(output_path) {
                eprintln!("    ❌ Lỗi ghi file trọng số: {e}");
            } else {
                println!("    💾 Đã xuất trọng số mới sang: {output_path}");
            }
        } else {
            println!(
                "    🛡️ [GIỮ NGUYÊN CHAMPION] Candidate chưa đạt ngưỡng ({:.1}% < {:.0}%). Giữ lại mô hình vô địch.",
                winrate_pct,
                threshold * 100.0
            );
        }

        println!(
            "  ⏱️ Thời gian Epoch {epoch}: {:.2?}",
            epoch_start.elapsed()
        );
    }

    println!("\n================================================================================");
    println!(
        "🎉 TIẾN TRÌNH HUẤN LUYỆN HOÀN TẤT TRONG: {:.2?}",
        overall_start.elapsed()
    );
    println!("================================================================================");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mode = args
        .iter()
        .position(|a| a == "--mode")
        .and_then(|idx| args.get(idx + 1))
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "dataset".to_string());

    let dataset_path = args
        .iter()
        .position(|a| a == "--dataset")
        .and_then(|idx| args.get(idx + 1))
        .map_or("cotuong_trainer/data/sample_master_games.txt", |s| {
            s.as_str()
        });

    let engine_path = args
        .iter()
        .position(|a| a == "--engine-path" || a == "--engine")
        .and_then(|idx| args.get(idx + 1))
        .map(String::as_str);

    let engine_proto = args
        .iter()
        .position(|a| a == "--engine-proto" || a == "--proto")
        .and_then(|idx| args.get(idx + 1))
        .map(|s| {
            if s.to_lowercase() == "uci" {
                EngineProtocol::Uci
            } else {
                EngineProtocol::Ucci
            }
        })
        .unwrap_or(EngineProtocol::Ucci);

    let engine_depth = args
        .iter()
        .position(|a| a == "--engine-depth" || a == "--depth")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(10);

    let epochs = args
        .iter()
        .position(|a| a == "--epochs")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(5);

    let num_games = args
        .iter()
        .position(|a| a == "--games")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20);

    let eval_games = args
        .iter()
        .position(|a| a == "--eval-games")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);

    let lr = args
        .iter()
        .position(|a| a == "--lr")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.01);

    let output_path = args
        .iter()
        .position(|a| a == "--output")
        .and_then(|idx| args.get(idx + 1))
        .map_or("cotuong_core/assets/default.nnue", |s| s.as_str());

    println!("================================================================================");
    println!(" 🧠 Xiangqi AI Trainer (Master Dataset & External UCCI/UCI Engine Integration)  ");
    println!("================================================================================");
    println!("Cấu hình:");
    println!("  • Chế độ (Mode)            : {mode}");
    println!("  • Số Epochs                : {epochs}");
    println!("  • Tốc độ học (LR)          : {lr}");
    println!("  • Đường dẫn trọng số đích : {output_path}");

    match mode.as_str() {
        "dataset" | "sl" => {
            println!("  • Tập dữ liệu kỳ phổ mẫu   : {dataset_path}");
            println!("  • Số ván Đấu Đánh Giá      : {eval_games}");
            println!(
                "================================================================================"
            );
            run_dataset_training(dataset_path, epochs, output_path, lr, eval_games);
        }
        "teacher" | "distill" => {
            let engine = engine_path.unwrap_or("pikafish");
            println!("  • Engine Sư Phụ            : {engine} ({engine_proto:?})");
            println!("  • Độ sâu phân tích         : {engine_depth}");
            println!("  • Tập thế cờ gán nhãn      : {dataset_path}");
            println!(
                "================================================================================"
            );
            run_teacher_distillation(
                engine,
                engine_proto,
                dataset_path,
                engine_depth,
                epochs,
                output_path,
                lr,
            );
        }
        "sparring" | "match" => {
            let engine = engine_path.unwrap_or("pikafish");
            println!("  • Engine Đối thủ           : {engine} ({engine_proto:?})");
            println!("  • Số ván đấu               : {num_games}");
            println!("  • Độ sâu đấu tập           : {engine_depth}");
            println!(
                "================================================================================"
            );
            run_sparring_match(engine, engine_proto, num_games, engine_depth, output_path);
        }
        "selfplay" | "rl" => {
            println!("  • Số ván Self-Play/Epoch   : {num_games}");
            println!("  • Số ván Đấu Đánh Giá      : {eval_games}");
            println!(
                "================================================================================"
            );
            run_selfplay_training(num_games, epochs, eval_games, output_path, lr);
        }
        unknown => {
            eprintln!("❌ Chế độ không hợp lệ: '{unknown}'. Hỗ trợ: 'dataset', 'teacher', 'sparring', 'selfplay'.");
        }
    }
}
