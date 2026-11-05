use cotuong_core::engine::config::EngineConfig;
use cotuong_core::engine::nnue::{
    get_black_king_palace_idx, get_red_king_palace_idx, halfkp_feature_idx_black,
    halfkp_feature_idx_red, NnueWeights, FILE_SIZE, L1_INPUT_SIZE, L1_OUTPUT_SIZE, L2_INPUT_SIZE,
    L2_OUTPUT_SIZE, L3_INPUT_SIZE, L3_OUTPUT_SIZE, NNUE_MAGIC, NNUE_VERSION,
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
use std::time::Instant;

#[derive(Clone)]
pub struct PositionSample {
    pub red_features: Vec<usize>,
    pub black_features: Vec<usize>,
    pub search_eval: f32, // Centipawns from Red's perspective
    pub game_result: f32, // 1.0 (Red win), 0.5 (Draw), 0.0 (Black win)
}

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
        // [Advisor: 12, Elephant: 12, Horse: 27, Chariot: 60, Cannon: 28.5, Soldier: 3, Enemy General: 600]
        let piece_values = [12.0f32, 12.0, 27.0, 60.0, 28.5, 3.0, 600.0];
        for k in 0..9 {
            for (ptype_idx, &val) in piece_values.iter().enumerate() {
                // Friendly (ptype_idx 0..6)
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
                // Enemy (ptype_idx 0..7 => 6..12 in HalfKP)
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

        // Layer 2: connect friendly accumulator (0..192) positively, enemy accumulator (192..384) negatively
        if let Some(row) = w2_vec.first_mut() {
            if let Some(cell) = row.first_mut() {
                *cell = 1.0;
            }
            if let Some(cell) = row.get_mut(192 + 4) {
                *cell = -1.0;
            }
        }

        // Layer 3: identity on diagonal
        for (i, row) in w3_vec.iter_mut().enumerate() {
            if let Some(cell) = row.get_mut(i) {
                *cell = 1.0;
            }
        }

        // Layer 4: scale up to centipawns
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

    pub fn train_step(&mut self, sample: &PositionSample, lr: f32) {
        let (pred, l2_inp, l2_out, l3_out) =
            self.forward(&sample.red_features, &sample.black_features);

        let target_score = 0.5 * sigmoid(sample.search_eval / 400.0) + 0.5 * sample.game_result;
        let pred_prob = sigmoid(pred / 400.0);
        let error = pred_prob - target_score;

        let grad_l4 = (error * (pred_prob * (1.0 - pred_prob)) / 400.0).clamp(-1.0, 1.0);

        // Update Layer 4 (W4, B4)
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

        // Update Layer 3 (W3, B3)
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

        // Update Layer 2 (W2, B2)
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

fn extract_features(board: &Board) -> (Vec<usize>, Vec<usize>) {
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

    // Use current active weights in Hybrid/NNUE mode for Self-play
    let config = Arc::new(EngineConfig {
        eval_mode: 1, // Hybrid evaluation during self-play
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

        // First 6 plies: exploration with random legal moves
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

/// Plays a single head-to-head game between Candidate and Champion.
/// Returns Candidate's score: 1.0 (Win), 0.5 (Draw), 0.0 (Loss).
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
        eval_mode: 2, // Pure NNUE evaluation for tournament testing
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

    // 4 plies of random opening for diverse tournament games
    for _ in 0..4 {
        if let Some(m) = generate_random_legal_move(&board, turn, &mut rng) {
            board.apply_move(&m, turn);
            turn = turn.opposite();
        } else {
            break;
        }
    }

    // Play until game ends or 120 plies
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

    // Determine winner
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
        .unwrap_or(5);

    let eval_games = args
        .iter()
        .position(|a| a == "--eval-games")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);

    let output_path = args
        .iter()
        .position(|a| a == "--output")
        .and_then(|idx| args.get(idx + 1))
        .map_or("cotuong_core/assets/default.nnue", |s| s.as_str());

    println!("================================================================================");
    println!(" 🧠 Xiangqi Iterative RL Trainer (HalfKP NNUE + Self-Play + Gating Tournament) ");
    println!("================================================================================");
    println!("Cấu hình:");
    println!("  • Kiến trúc mạng           : HalfKP(10.530 -> 192) x 2 -> 32 -> 32 -> 1");
    println!("  • Số Epochs (Generations)  : {epochs}");
    println!("  • Số ván Self-Play/Epoch   : {num_games}");
    println!("  • Số ván Đấu Đánh Giá      : {eval_games} (Tráo đổi màu quân 50/50)");
    println!("  • Đường dẫn trọng số đích : {output_path}");
    println!("================================================================================");

    let overall_start = Instant::now();

    // Initialize Champion model
    let mut champion_model = FloatNnue::new();
    let mut champion_weights = Arc::new(champion_model.to_quantized_weights());

    // Replay buffer for sliding window of past training samples
    let mut replay_buffer: Vec<PositionSample> = Vec::new();
    let max_buffer_size = 6000;

    let lr = 0.01;

    for epoch in 1..=epochs {
        println!(
            "\n--------------------------------------------------------------------------------"
        );
        println!(" 🏆 EPOCH {epoch}/{epochs} BẮT ĐẦU");
        println!(
            "--------------------------------------------------------------------------------"
        );
        let epoch_start = Instant::now();

        // 1. Self-Play data generation using current champion weights
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

        // Append to replay buffer with sliding window
        replay_buffer.extend(new_samples);
        if replay_buffer.len() > max_buffer_size {
            let drain_count = replay_buffer.len() - max_buffer_size;
            replay_buffer.drain(0..drain_count);
        }
        println!(
            "    Tổng số mẫu trong Replay Buffer: {}",
            replay_buffer.len()
        );

        // 2. Train Candidate model
        print!("  ▶ [2/3] Huấn luyện Candidate Model (SGD/Adam)... ");
        std::io::stdout().flush().ok();
        let train_start = Instant::now();

        let mut candidate_model = champion_model.clone();
        for sample in &replay_buffer {
            candidate_model.train_step(sample, lr);
        }
        println!("xong! ({:.2?})", train_start.elapsed());

        // 3. Evaluation Tournament (Candidate vs Champion)
        print!("  ▶ [3/3] Giải đấu Đánh giá: Candidate vs Champion ({eval_games} ván)... ");
        std::io::stdout().flush().ok();
        let eval_start = Instant::now();

        let cand_weights = Arc::new(candidate_model.to_quantized_weights());

        // Play eval_games in parallel with alternating colors
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

        // Gating threshold: Candidate promoted if winrate >= 50%
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
        "🎉 TOÀN BỘ TIẾN TRÌNH RL HOÀN TẤT TRONG: {:.2?}",
        overall_start.elapsed()
    );
    println!("================================================================================");
}
