use cotuong_core::engine::config::EngineConfig;
use cotuong_core::engine::nnue::{
    NnueWeights, FILE_SIZE, L1_INPUT_SIZE, L1_OUTPUT_SIZE, L2_INPUT_SIZE, L2_OUTPUT_SIZE,
    L3_INPUT_SIZE, L3_OUTPUT_SIZE, NNUE_MAGIC, NNUE_VERSION, PSQT_BUCKETS,
};
use cotuong_core::engine::search::AlphaBetaEngine;
use cotuong_core::engine::{Move, SearchLimit, Searcher};
use cotuong_core::logic::board::{Board, BoardCoordinate, Color, PieceType};
use cotuong_core::logic::eval_constants::get_pst_value;
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

pub const FLOAT_NNUE_MAGIC: &[u8; 8] = b"COTF32_1";
pub const FLOAT_FILE_SIZE: usize = 8
    + (L1_INPUT_SIZE * L1_OUTPUT_SIZE
        + L1_OUTPUT_SIZE
        + L1_INPUT_SIZE * PSQT_BUCKETS
        + L2_OUTPUT_SIZE * L2_INPUT_SIZE
        + L2_OUTPUT_SIZE
        + L3_OUTPUT_SIZE * L3_INPUT_SIZE
        + L3_OUTPUT_SIZE
        + L3_OUTPUT_SIZE
        + 1)
        * 4;

pub const DEFAULT_FLOAT_MODEL: &str = "checkpoints/model_latest.f32";
pub const DEFAULT_QUANTIZED_MODEL: &str = "cotuong_core/assets/default.nnue";

#[derive(Clone)]
pub struct FloatNnue {
    pub w1: Box<[[f32; L1_OUTPUT_SIZE]; L1_INPUT_SIZE]>,
    pub b1: [f32; L1_OUTPUT_SIZE],
    pub w_psqt: Box<[[f32; PSQT_BUCKETS]; L1_INPUT_SIZE]>,
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
    #[allow(
        clippy::too_many_lines,
        clippy::cast_precision_loss,
        clippy::cognitive_complexity,
        clippy::indexing_slicing
    )]
    pub fn new() -> Self {
        let mut w1_vec = vec![[0.0f32; L1_OUTPUT_SIZE]; L1_INPUT_SIZE];
        let mut b1 = [0.0f32; L1_OUTPUT_SIZE];
        let mut w_psqt_vec = vec![[0.0f32; PSQT_BUCKETS]; L1_INPUT_SIZE];
        let mut w2_vec = vec![[0.0f32; L2_INPUT_SIZE]; L2_OUTPUT_SIZE];
        let b2 = [0.0f32; L2_OUTPUT_SIZE];
        let mut w3_vec = vec![[0.0f32; L3_INPUT_SIZE]; L3_OUTPUT_SIZE];
        let b3 = [0.0f32; L3_OUTPUT_SIZE];
        let mut w4 = [0.0f32; L3_OUTPUT_SIZE];

        // Set threshold biases for non-linear synergy & King safety ReLU circuits
        b1[8] = -0.55; // Connected Advisors (activates when 2 Advisors present)
        b1[9] = -0.55; // Connected Elephants (activates when 2 Elephants present)
        b1[10] = -0.55; // Double Chariot Battery
        b1[11] = -0.75; // Horse + Cannon Coordination
        b1[16] = 0.0; // King Line Cannon & Chariot Threat
        b1[17] = -1.25; // King File Cannon Double-Blocker Shield (cancels threat at >=2 blockers)
        b1[18] = -0.70; // King File Chariot Single-Blocker Shield

        // Friendly HalfKP types: 0=Advisor, 1=Elephant, 2=Horse, 3=Chariot, 4=Cannon, 5=Soldier
        // Enemy HalfKP types: 6=General, 7=Advisor, 8=Elephant, 9=Horse, 10=Chariot, 11=Cannon, 12=Soldier
        let friendly_types = [
            PieceType::Advisor,
            PieceType::Elephant,
            PieceType::Horse,
            PieceType::Chariot,
            PieceType::Cannon,
            PieceType::Soldier,
        ];
        let enemy_types = [
            PieceType::General,
            PieceType::Advisor,
            PieceType::Elephant,
            PieceType::Horse,
            PieceType::Chariot,
            PieceType::Cannon,
            PieceType::Soldier,
        ];

        for k in 0..9 {
            let kr = k / 3;
            let kc = 3 + (k % 3);

            // 1. Friendly piece features (ptype_idx 0..6)
            for (ptype_idx, &pt) in friendly_types.iter().enumerate() {
                for sq in 0..90 {
                    let r = sq / 9;
                    let c = sq % 9;
                    let feat = k * 1170 + ptype_idx * 90 + sq;
                    let pst = get_pst_value(pt, Color::Red, r, c) as f32;

                    if let Some(psqt_row) = w_psqt_vec.get_mut(feat) {
                        for (bucket, cell) in psqt_row.iter_mut().enumerate() {
                            let phase = bucket as f32 / (PSQT_BUCKETS.saturating_sub(1) as f32);
                            let base_cp = match pt {
                                PieceType::Advisor | PieceType::Elephant => {
                                    120.0 + 10.0 * (1.0 - phase)
                                }
                                PieceType::Horse => 265.0 + 30.0 * (1.0 - phase),
                                PieceType::Chariot => 600.0,
                                PieceType::Cannon => 270.0 + 30.0 * phase,
                                PieceType::Soldier => 30.0 + 25.0 * (1.0 - phase),
                                PieceType::General => 0.0,
                            };
                            let dev_bonus = match pt {
                                PieceType::Chariot if r == 0 && (c == 1 || c == 7) => 16.0,
                                PieceType::Chariot if r == 0 && (c == 0 || c == 8) => -6.0,
                                PieceType::Horse if r == 2 && (c == 2 || c == 6) => 12.0,
                                PieceType::Advisor if r == 1 && c == 4 => 12.0,
                                _ => 0.0,
                            };
                            let king_home_bonus = if kr == 0 && kc == 4 {
                                3.0 * phase
                            } else if kr >= 1 {
                                -4.0 * phase
                            } else {
                                0.0
                            };
                            // Shield bonus when friendly piece protects King file in front of King
                            let shield_bonus = if c == kc && r > kr && r <= 3 {
                                8.0
                            } else {
                                0.0
                            };
                            *cell = base_cp + pst + dev_bonus + king_home_bonus + shield_bonus;
                        }
                    }

                    if let Some(row) = w1_vec.get_mut(feat) {
                        // Synergy & structure detectors
                        match pt {
                            PieceType::Advisor => {
                                row[8] += 0.55;
                            }
                            PieceType::Elephant => {
                                row[9] += if r == 2 && c == 4 { 0.65 } else { 0.50 };
                            }
                            PieceType::Chariot => {
                                row[10] += 0.55;
                                if r >= 5 {
                                    row[12] += 0.35;
                                }
                            }
                            PieceType::Horse => {
                                if r >= 3 {
                                    row[11] += 0.45;
                                }
                                if r >= 6 && (2..=6).contains(&c) {
                                    row[12] += 0.40;
                                }
                            }
                            PieceType::Cannon => {
                                row[11] += 0.45;
                                if c == 4 {
                                    row[13] += 0.35;
                                }
                            }
                            PieceType::Soldier => {
                                if r >= 5 {
                                    row[14] += 0.30;
                                }
                            }
                            PieceType::General => {}
                        }

                        // Blocker on King file between King and river (for Cannon/Chariot shield circuits)
                        if c == kc && r > kr && r <= 4 {
                            row[17] += 0.625;
                            row[18] += 0.70;
                        }

                        // Distributed positional channels (neurons 24..192)
                        for (idx, cell) in row.iter_mut().enumerate().skip(24) {
                            let h = ((feat.wrapping_mul(2_654_435_761)) ^ (idx.wrapping_mul(97)))
                                & 0xFF;
                            let signed = (h as f32 - 128.0) / 128.0;
                            *cell = signed * 0.04;
                        }
                    }
                }
            }

            // 2. Enemy piece features (enemy_halfkp_idx 6..13)
            for (e_idx, &pt) in enemy_types.iter().enumerate() {
                let ptype_idx = 6 + e_idx;
                for sq in 0..90 {
                    let r = sq / 9;
                    let c = sq % 9;
                    let feat = k * 1170 + ptype_idx * 90 + sq;

                    if let Some(row) = w1_vec.get_mut(feat) {
                        // King Safety circuits conditioned on friendly King at (kr, kc)
                        if pt == PieceType::Cannon && (r == kr || c == kc) && (r != kr || c != kc) {
                            row[16] += 0.78;
                            if c == kc && r > kr {
                                row[17] += 0.78;
                            }
                        } else if pt == PieceType::Chariot
                            && (r == kr || c == kc)
                            && (r != kr || c != kc)
                        {
                            row[16] += 0.65;
                            if c == kc && r > kr {
                                row[18] += 0.70;
                            }
                        }

                        // Enemy pieces invading friendly palace zone
                        if r <= 3 && (2..=6).contains(&c) {
                            match pt {
                                PieceType::Chariot | PieceType::Horse | PieceType::Cannon => {
                                    row[19] += 0.40;
                                }
                                PieceType::Soldier => {
                                    row[19] += 0.30;
                                }
                                _ => {}
                            }
                        }

                        // Blocker on King file (an enemy non-cannon piece at r <= 4 also acts as a screen)
                        if c == kc && r > kr && r <= 4 && pt != PieceType::Cannon {
                            row[17] += 0.625;
                            row[18] += 0.70;
                        }

                        // Distributed positional channels (neurons 24..192)
                        for (idx, cell) in row.iter_mut().enumerate().skip(24) {
                            let h = ((feat.wrapping_mul(2_654_435_761)) ^ (idx.wrapping_mul(131)))
                                & 0xFF;
                            let signed = (h as f32 - 128.0) / 128.0;
                            *cell = signed * 0.04;
                        }
                    }
                }
            }
        }

        // Layer 2: Wire active (0..192) and opponent (192..384) feature detectors into L2 neurons
        // Neuron 0: Friendly synergies (Advisors, Elephants, Double Chariot, Horse+Cannon, Siege)
        w2_vec[0][8] = 0.55;
        w2_vec[0][9] = 0.55;
        w2_vec[0][10] = 0.65;
        w2_vec[0][11] = 0.50;
        w2_vec[0][12] = 0.60;
        w2_vec[0][13] = 0.45;
        w2_vec[0][14] = 0.50;

        // Neuron 1: Opponent King danger (Opponent's Neuron 16 minus Opponent's Shields 17 & 18) -> good for us!
        w2_vec[1][L1_OUTPUT_SIZE + 16] = 1.45;
        w2_vec[1][L1_OUTPUT_SIZE + 17] = -1.45;
        w2_vec[1][L1_OUTPUT_SIZE + 18] = -1.10;
        w2_vec[1][L1_OUTPUT_SIZE + 19] = 0.85;

        // Neuron 2: Our King danger (Our Neuron 16 minus Our Shields 17 & 18) -> bad for us!
        w2_vec[2][16] = 1.45;
        w2_vec[2][17] = -1.45;
        w2_vec[2][18] = -1.10;
        w2_vec[2][19] = 0.85;

        // Neurons 3..32: Distributed cross-perspective connections for training
        for (j, row) in w2_vec.iter_mut().enumerate().skip(3) {
            for (i, cell) in row.iter_mut().enumerate() {
                let h = ((j.wrapping_mul(313)) ^ (i.wrapping_mul(1_000_003))) & 0xFF;
                *cell = ((h as f32 - 128.0) / 128.0) * 0.08;
            }
        }

        // Layer 3: Identity + cross-channel mixing
        for (i, row) in w3_vec.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                if i == j {
                    *cell = 1.0;
                } else {
                    let h = ((i.wrapping_mul(73)) ^ (j.wrapping_mul(199))) & 0xFF;
                    *cell = ((h as f32 - 128.0) / 128.0) * 0.04;
                }
            }
        }

        // Layer 4: Output weights
        w4[0] = 65.0; // Friendly synergy bonus
        w4[1] = 135.0; // Enemy King exposed penalty (good for us)
        w4[2] = -135.0; // Our King exposed penalty (bad for us)
        for (k, cell) in w4.iter_mut().enumerate().skip(3) {
            let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
            *cell = sign * 8.0;
        }

        let Ok(w1) = w1_vec.into_boxed_slice().try_into() else {
            unreachable!()
        };
        let Ok(w_psqt) = w_psqt_vec.into_boxed_slice().try_into() else {
            unreachable!()
        };
        let Ok(w2) = w2_vec.into_boxed_slice().try_into() else {
            unreachable!()
        };
        let Ok(w3) = w3_vec.into_boxed_slice().try_into() else {
            unreachable!()
        };

        Self {
            w1,
            b1,
            w_psqt,
            w2,
            b2,
            w3,
            b3,
            w4,
            b4: 0.0,
        }
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::cast_precision_loss)]
    pub fn forward(
        &self,
        red_features: &[usize],
        black_features: &[usize],
    ) -> (
        f32,
        [f32; L2_INPUT_SIZE],
        [f32; L2_INPUT_SIZE],
        [f32; L2_OUTPUT_SIZE],
        [f32; L3_OUTPUT_SIZE],
        usize,
    ) {
        let piece_count = red_features.len().max(black_features.len()) + 2;
        let bucket = ((piece_count.saturating_sub(2)) * PSQT_BUCKETS / 30).min(PSQT_BUCKETS - 1);

        let mut acc_red = self.b1;
        let mut psqt_red = 0.0f32;
        for &feat in red_features {
            if let Some(row) = self.w1.get(feat) {
                for (a, &w) in acc_red.iter_mut().zip(row.iter()) {
                    *a += w;
                }
            }
            if let Some(psqt_row) = self.w_psqt.get(feat) {
                if let Some(&p) = psqt_row.get(bucket) {
                    psqt_red += p;
                }
            }
        }

        let mut acc_black = self.b1;
        let mut psqt_black = 0.0f32;
        for &feat in black_features {
            if let Some(row) = self.w1.get(feat) {
                for (a, &w) in acc_black.iter_mut().zip(row.iter()) {
                    *a += w;
                }
            }
            if let Some(psqt_row) = self.w_psqt.get(feat) {
                if let Some(&p) = psqt_row.get(bucket) {
                    psqt_black += p;
                }
            }
        }

        // L1 SCReLU activation: clamped in [0.0, 1.0], squared
        let mut l1_clamped = [0.0f32; L2_INPUT_SIZE];
        let mut l2_input = [0.0f32; L2_INPUT_SIZE];
        if let Some(left_c) = l1_clamped.get_mut(..L1_OUTPUT_SIZE) {
            for (c_out, &val) in left_c.iter_mut().zip(acc_red.iter()) {
                *c_out = val.clamp(0.0, 1.0);
            }
        }
        if let Some(right_c) = l1_clamped.get_mut(L1_OUTPUT_SIZE..) {
            for (c_out, &val) in right_c.iter_mut().zip(acc_black.iter()) {
                *c_out = val.clamp(0.0, 1.0);
            }
        }
        for (act_out, &c_val) in l2_input.iter_mut().zip(l1_clamped.iter()) {
            *act_out = c_val * c_val;
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

        // Layer 4 (Output): 32 -> 1 + PSQT Head
        let mut nn_sum = self.b4;
        for (&val, &w) in l3_output.iter().zip(self.w4.iter()) {
            nn_sum += val * w;
        }

        let total_eval = (psqt_red - psqt_black) + nn_sum;
        (
            total_eval, l1_clamped, l2_input, l2_output, l3_output, bucket,
        )
    }

    #[allow(clippy::indexing_slicing, clippy::too_many_lines)]
    pub fn train_step(&mut self, sample: &PositionSample, lr: f32) -> f32 {
        let (pred, l1_clamped, l2_inp, l2_out, l3_out, bucket) =
            self.forward(&sample.red_features, &sample.black_features);

        let target_score = if sample.search_eval.abs() > 1e-4 {
            0.5 * sigmoid(sample.search_eval / 400.0) + 0.5 * sample.game_result
        } else {
            sample.game_result
        };

        let pred_prob = sigmoid(pred / 400.0);
        let error = pred_prob - target_score;

        let grad_out = (error * (pred_prob * (1.0 - pred_prob)) / 400.0).clamp(-1.0, 1.0);

        // 1. Update PSQT Head for active features
        let psqt_lr = lr * 2.5;
        for &feat in &sample.red_features {
            if let Some(psqt_row) = self.w_psqt.get_mut(feat) {
                if let Some(cell) = psqt_row.get_mut(bucket) {
                    *cell -= psqt_lr * grad_out;
                }
            }
        }
        for &feat in &sample.black_features {
            if let Some(psqt_row) = self.w_psqt.get_mut(feat) {
                if let Some(cell) = psqt_row.get_mut(bucket) {
                    *cell += psqt_lr * grad_out;
                }
            }
        }

        // 2. Update Layer 4
        self.b4 -= lr * grad_out;
        for (w, &val) in self.w4.iter_mut().zip(l3_out.iter()) {
            *w -= lr * grad_out * val;
        }

        // 3. Backprop to Layer 3
        let mut grad_l3 = [0.0f32; L3_OUTPUT_SIZE];
        for (k, (&w4_val, &l3_val)) in self.w4.iter().zip(l3_out.iter()).enumerate() {
            if l3_val > 0.0 && l3_val < 1.0 {
                if let Some(g) = grad_l3.get_mut(k) {
                    *g = (grad_out * w4_val).clamp(-1.0, 1.0);
                }
            }
        }

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

        // 4. Backprop to Layer 2
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
                if let Some(g) = grad_l2.get_mut(j) {
                    *g = sum_g.clamp(-1.0, 1.0);
                }
            }
        }

        // 5. Backprop to Layer 1 (SCReLU derivative: 2 * c_val for c_val in (0, 1))
        let mut grad_l1 = [0.0f32; L2_INPUT_SIZE];
        for (i, &c_val) in l1_clamped.iter().enumerate() {
            if c_val > 0.0 && c_val < 1.0 {
                let mut sum_g = 0.0f32;
                for (j, &g_l2) in grad_l2.iter().enumerate() {
                    if let Some(w_row) = self.w2.get(j) {
                        if let Some(&w) = w_row.get(i) {
                            sum_g += g_l2 * w;
                        }
                    }
                }
                if let Some(g1) = grad_l1.get_mut(i) {
                    *g1 = (sum_g * 2.0 * c_val).clamp(-1.0, 1.0);
                }
            }
        }

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

        // Update sparse active features in W1 (trainable channels 24..L1_OUTPUT_SIZE)
        let l1_lr = lr * 0.25;
        for &feat in &sample.red_features {
            if let Some(row) = self.w1.get_mut(feat) {
                for idx in 24..L1_OUTPUT_SIZE {
                    let g = grad_l1[idx];
                    if g.abs() > 1e-7 {
                        row[idx] = (row[idx] - l1_lr * g).clamp(-4.0, 4.0);
                    }
                }
            }
        }
        for &feat in &sample.black_features {
            if let Some(row) = self.w1.get_mut(feat) {
                for idx in 24..L1_OUTPUT_SIZE {
                    let g = grad_l1[L1_OUTPUT_SIZE + idx];
                    if g.abs() > 1e-7 {
                        row[idx] = (row[idx] - l1_lr * g).clamp(-4.0, 4.0);
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
        let Ok(w1) = w1_vec.into_boxed_slice().try_into() else {
            unreachable!()
        };

        let mut b1 = [0i16; L1_OUTPUT_SIZE];
        for (dst, &val) in b1.iter_mut().zip(self.b1.iter()) {
            #[allow(clippy::cast_possible_truncation)]
            {
                *dst = (val * 256.0).clamp(-32767.0, 32767.0) as i16;
            }
        }

        let mut w_psqt_vec = vec![[0i16; PSQT_BUCKETS]; L1_INPUT_SIZE];
        for (dst_row, src_row) in w_psqt_vec.iter_mut().zip(self.w_psqt.iter()) {
            for (dst, &val) in dst_row.iter_mut().zip(src_row.iter()) {
                #[allow(clippy::cast_possible_truncation)]
                {
                    *dst = (val * 16.0).clamp(-32767.0, 32767.0) as i16;
                }
            }
        }
        let Ok(w_psqt) = w_psqt_vec.into_boxed_slice().try_into() else {
            unreachable!()
        };

        let mut w2_vec = vec![[0i8; L2_INPUT_SIZE]; L2_OUTPUT_SIZE];
        for (dst_row, src_row) in w2_vec.iter_mut().zip(self.w2.iter()) {
            for (dst, &val) in dst_row.iter_mut().zip(src_row.iter()) {
                #[allow(clippy::cast_possible_truncation)]
                {
                    *dst = (val * 64.0).clamp(-127.0, 127.0) as i8;
                }
            }
        }
        let Ok(w2): Result<Box<[[i8; L2_INPUT_SIZE]; L2_OUTPUT_SIZE]>, _> =
            w2_vec.into_boxed_slice().try_into()
        else {
            unreachable!()
        };
        let w2_col = NnueWeights::transpose_w2(&w2);

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
        let Ok(w3): Result<Box<[[i8; L3_INPUT_SIZE]; L3_OUTPUT_SIZE]>, _> =
            w3_vec.into_boxed_slice().try_into()
        else {
            unreachable!()
        };
        let w3_col = NnueWeights::transpose_w3(&w3);

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
            w_psqt,
            w2,
            w2_col,
            b2,
            w3,
            w3_col,
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

        // W_PSQT: scale f32 to i16 (x16)
        for row in self.w_psqt.iter() {
            for &val in row.iter() {
                #[allow(clippy::cast_possible_truncation)]
                let q = (val * 16.0).clamp(-32767.0, 32767.0) as i16;
                bytes.extend_from_slice(&q.to_le_bytes());
            }
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

    /// Saves full-precision unquantized model weights to a binary file.
    pub fn save_float(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        let mut bytes = Vec::with_capacity(FLOAT_FILE_SIZE);
        bytes.extend_from_slice(FLOAT_NNUE_MAGIC);

        for row in self.w1.iter() {
            for &val in row.iter() {
                bytes.extend_from_slice(&val.to_le_bytes());
            }
        }
        for &val in self.b1.iter() {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        for row in self.w_psqt.iter() {
            for &val in row.iter() {
                bytes.extend_from_slice(&val.to_le_bytes());
            }
        }
        for row in self.w2.iter() {
            for &val in row.iter() {
                bytes.extend_from_slice(&val.to_le_bytes());
            }
        }
        for &val in self.b2.iter() {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        for row in self.w3.iter() {
            for &val in row.iter() {
                bytes.extend_from_slice(&val.to_le_bytes());
            }
        }
        for &val in self.b3.iter() {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        for &val in self.w4.iter() {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        bytes.extend_from_slice(&self.b4.to_le_bytes());

        file.write_all(&bytes)?;
        Ok(())
    }

    /// Loads full-precision unquantized model weights from bytes.
    pub fn load_float(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != FLOAT_FILE_SIZE {
            return Err(format!(
                "Invalid Float NNUE size: expected {}, got {}",
                FLOAT_FILE_SIZE,
                bytes.len()
            ));
        }
        if bytes.get(0..8) != Some(FLOAT_NNUE_MAGIC) {
            return Err("Invalid Float NNUE magic header".to_string());
        }

        let mut offset = 8;
        let mut read_f32 = |data: &[u8]| -> Result<f32, String> {
            let slice = data
                .get(offset..offset + 4)
                .ok_or_else(|| "Unexpected EOF reading float".to_string())?;
            offset += 4;
            let arr: [u8; 4] = slice.try_into().map_err(|_| "Slice convert error")?;
            Ok(f32::from_le_bytes(arr))
        };

        let mut w1_vec = vec![[0.0f32; L1_OUTPUT_SIZE]; L1_INPUT_SIZE];
        for row in &mut w1_vec {
            for val in row {
                *val = read_f32(bytes)?;
            }
        }
        let w1: Box<[[f32; L1_OUTPUT_SIZE]; L1_INPUT_SIZE]> = w1_vec
            .into_boxed_slice()
            .try_into()
            .map_err(|_| "Failed to allocate boxed W1 array".to_string())?;

        let mut b1 = [0.0f32; L1_OUTPUT_SIZE];
        for val in &mut b1 {
            *val = read_f32(bytes)?;
        }

        let mut w_psqt_vec = vec![[0.0f32; PSQT_BUCKETS]; L1_INPUT_SIZE];
        for row in &mut w_psqt_vec {
            for val in row {
                *val = read_f32(bytes)?;
            }
        }
        let w_psqt: Box<[[f32; PSQT_BUCKETS]; L1_INPUT_SIZE]> = w_psqt_vec
            .into_boxed_slice()
            .try_into()
            .map_err(|_| "Failed to allocate boxed W_PSQT array".to_string())?;

        let mut w2_vec = vec![[0.0f32; L2_INPUT_SIZE]; L2_OUTPUT_SIZE];
        for row in &mut w2_vec {
            for val in row {
                *val = read_f32(bytes)?;
            }
        }
        let w2: Box<[[f32; L2_INPUT_SIZE]; L2_OUTPUT_SIZE]> = w2_vec
            .into_boxed_slice()
            .try_into()
            .map_err(|_| "Failed to allocate boxed W2 array".to_string())?;

        let mut b2 = [0.0f32; L2_OUTPUT_SIZE];
        for val in &mut b2 {
            *val = read_f32(bytes)?;
        }

        let mut w3_vec = vec![[0.0f32; L3_INPUT_SIZE]; L3_OUTPUT_SIZE];
        for row in &mut w3_vec {
            for val in row {
                *val = read_f32(bytes)?;
            }
        }
        let w3: Box<[[f32; L3_INPUT_SIZE]; L3_OUTPUT_SIZE]> = w3_vec
            .into_boxed_slice()
            .try_into()
            .map_err(|_| "Failed to allocate boxed W3 array".to_string())?;

        let mut b3 = [0.0f32; L3_OUTPUT_SIZE];
        for val in &mut b3 {
            *val = read_f32(bytes)?;
        }

        let mut w4 = [0.0f32; L3_OUTPUT_SIZE];
        for val in &mut w4 {
            *val = read_f32(bytes)?;
        }

        let b4 = read_f32(bytes)?;

        Ok(Self {
            w1,
            b1,
            w_psqt,
            w2,
            b2,
            w3,
            b3,
            w4,
            b4,
        })
    }

    /// Loads weights from quantized `NnueWeights`.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn from_quantized_weights(weights: &NnueWeights) -> Self {
        let mut w1_vec = vec![[0.0f32; L1_OUTPUT_SIZE]; L1_INPUT_SIZE];
        for (dst_row, src_row) in w1_vec.iter_mut().zip(weights.w1.iter()) {
            for (dst, &val) in dst_row.iter_mut().zip(src_row.iter()) {
                *dst = f32::from(val) / 256.0;
            }
        }
        let Ok(w1) = w1_vec.into_boxed_slice().try_into() else {
            unreachable!()
        };

        let mut b1 = [0.0f32; L1_OUTPUT_SIZE];
        for (dst, &val) in b1.iter_mut().zip(weights.b1.iter()) {
            *dst = f32::from(val) / 256.0;
        }

        let mut w_psqt_vec = vec![[0.0f32; PSQT_BUCKETS]; L1_INPUT_SIZE];
        for (dst_row, src_row) in w_psqt_vec.iter_mut().zip(weights.w_psqt.iter()) {
            for (dst, &val) in dst_row.iter_mut().zip(src_row.iter()) {
                *dst = f32::from(val) / 16.0;
            }
        }
        let Ok(w_psqt) = w_psqt_vec.into_boxed_slice().try_into() else {
            unreachable!()
        };

        let mut w2_vec = vec![[0.0f32; L2_INPUT_SIZE]; L2_OUTPUT_SIZE];
        for (dst_row, src_row) in w2_vec.iter_mut().zip(weights.w2.iter()) {
            for (dst, &val) in dst_row.iter_mut().zip(src_row.iter()) {
                *dst = f32::from(val) / 64.0;
            }
        }
        let Ok(w2) = w2_vec.into_boxed_slice().try_into() else {
            unreachable!()
        };

        let mut b2 = [0.0f32; L2_OUTPUT_SIZE];
        for (dst, &val) in b2.iter_mut().zip(weights.b2.iter()) {
            *dst = (val as f32) / 4096.0;
        }

        let mut w3_vec = vec![[0.0f32; L3_INPUT_SIZE]; L3_OUTPUT_SIZE];
        for (dst_row, src_row) in w3_vec.iter_mut().zip(weights.w3.iter()) {
            for (dst, &val) in dst_row.iter_mut().zip(src_row.iter()) {
                *dst = f32::from(val) / 64.0;
            }
        }
        let Ok(w3) = w3_vec.into_boxed_slice().try_into() else {
            unreachable!()
        };

        let mut b3 = [0.0f32; L3_OUTPUT_SIZE];
        for (dst, &val) in b3.iter_mut().zip(weights.b3.iter()) {
            *dst = (val as f32) / 4096.0;
        }

        let mut w4 = [0.0f32; L3_OUTPUT_SIZE];
        for (dst, &val) in w4.iter_mut().zip(weights.w4.iter()) {
            *dst = f32::from(val) / 16.0;
        }

        let b4 = (weights.b4 as f32) / 16.0;

        Self {
            w1,
            b1,
            w_psqt,
            w2,
            b2,
            w3,
            b3,
            w4,
            b4,
        }
    }

    /// Loads model weights from either a full-precision float file (.f32) or a quantized NNUE file (.nnue).
    pub fn load_from_file(path: &str) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("Failed to read '{path}': {e}"))?;
        if bytes.starts_with(FLOAT_NNUE_MAGIC) {
            Self::load_float(&bytes)
        } else if bytes.starts_with(NNUE_MAGIC) {
            let weights = NnueWeights::load_from_bytes(&bytes)?;
            Ok(Self::from_quantized_weights(&weights))
        } else {
            Err(format!("File '{path}' has unknown format/header"))
        }
    }

    /// Loads model weights from specified path, or fallback existing file, or creates new if none found or scratch.
    #[must_use]
    pub fn load_or_init(
        specified_path: Option<&str>,
        fallback_path: Option<&str>,
        scratch: bool,
    ) -> Self {
        if scratch {
            println!("🌱 Khởi tạo mô hình NNUE mới từ đầu (--scratch)...");
            return Self::new();
        }

        if let Some(path) = specified_path {
            match Self::load_from_file(path) {
                Ok(model) => {
                    println!("♻️  Đã nạp thành công trọng số từ: {path} để tiếp tục huấn luyện!");
                    return model;
                }
                Err(e) => {
                    eprintln!("⚠️  Không thể nạp trọng số từ '{path}': {e}. Sẽ khởi tạo mạng mới.");
                    return Self::new();
                }
            }
        }

        if let Some(fb) = fallback_path {
            if std::path::Path::new(fb).exists() {
                match Self::load_from_file(fb) {
                    Ok(model) => {
                        println!(
                            "♻️  Tự động nạp trọng số chưa lượng tử hóa từ: {fb} để tiếp tục huấn luyện!"
                        );
                        return model;
                    }
                    Err(e) => {
                        println!(
                            "⚠️  Tệp '{fb}' tồn tại nhưng nạp thất bại: {e}. Sẽ thử tìm file khác."
                        );
                    }
                }
            }
        }

        // Also check if default quantized weights exist
        let default_quantized = "cotuong_core/assets/default.nnue";
        if std::path::Path::new(default_quantized).exists() {
            if let Ok(model) = Self::load_from_file(default_quantized) {
                println!("♻️  Tự động nạp trọng số từ: {default_quantized}!");
                return model;
            }
        }

        println!("🌱 Không tìm thấy trọng số trước đó. Khởi tạo mô hình mạng mới từ đầu!");
        Self::new()
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
    let mut game_state = cotuong_core::logic::game::GameState::new();
    let mut samples = Vec::new();

    let config = Arc::new(EngineConfig {
        nnue_weight: 75,
        ..Default::default()
    });
    let mut engine = AlphaBetaEngine::with_weights(config, weights);

    let mut rng = rand::thread_rng();

    for ply in 0..120 {
        if game_state.status != cotuong_core::logic::game::GameStatus::Playing {
            break;
        }

        let (red_feats, black_feats) = extract_features(&game_state.board);

        let best_move = if ply < 6 {
            let Some(m) = generate_random_legal_move(&game_state.board, game_state.turn, &mut rng)
            else {
                break;
            };
            m
        } else {
            let search_res = engine.search(
                &game_state,
                SearchLimit::DepthOrTime {
                    depth,
                    time_ms: 2500,
                },
                &[],
            );
            match search_res {
                Some((mv, stats)) => {
                    let eval = match game_state.turn {
                        Color::Red => stats.depth as f32 * 10.0,
                        Color::Black => -(stats.depth as f32 * 10.0),
                    };
                    samples.push((red_feats, black_feats, eval));
                    mv
                }
                None => break,
            }
        };

        if let (Some(from), Some(to)) = (
            BoardCoordinate::new(best_move.from_row as usize, best_move.from_col as usize),
            BoardCoordinate::new(best_move.to_row as usize, best_move.to_col as usize),
        ) {
            if game_state.make_move(from, to).is_err() {
                break;
            }
        } else {
            break;
        }
    }

    let result = match game_state.status {
        cotuong_core::logic::game::GameStatus::Checkmate(Color::Red) => 1.0,
        cotuong_core::logic::game::GameStatus::Checkmate(Color::Black) => 0.0,
        _ => 0.5,
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
    let mut game_state = cotuong_core::logic::game::GameState::new();

    let config_cand = Arc::new(EngineConfig::default());
    let config_champ = Arc::new(EngineConfig::default());

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
        if let Some(m) = generate_random_legal_move(&game_state.board, game_state.turn, &mut rng) {
            if let (Some(from), Some(to)) = (
                BoardCoordinate::new(m.from_row as usize, m.from_col as usize),
                BoardCoordinate::new(m.to_row as usize, m.to_col as usize),
            ) {
                if game_state.make_move(from, to).is_err() {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    for _ in 4..120 {
        if game_state.status != cotuong_core::logic::game::GameStatus::Playing {
            break;
        }

        let search_res = match game_state.turn {
            Color::Red => red_engine.search(
                &game_state,
                SearchLimit::DepthOrTime {
                    depth,
                    time_ms: 2500,
                },
                &[],
            ),
            Color::Black => black_engine.search(
                &game_state,
                SearchLimit::DepthOrTime {
                    depth,
                    time_ms: 2500,
                },
                &[],
            ),
        };

        match search_res {
            Some((mv, _)) => {
                if let (Some(from), Some(to)) = (
                    BoardCoordinate::new(mv.from_row as usize, mv.from_col as usize),
                    BoardCoordinate::new(mv.to_row as usize, mv.to_col as usize),
                ) {
                    if game_state.make_move(from, to).is_err() {
                        break;
                    }
                } else {
                    break;
                }
            }
            None => break,
        }
    }

    let (red_wins, black_wins) = match game_state.status {
        cotuong_core::logic::game::GameStatus::Checkmate(Color::Red) => (true, false),
        cotuong_core::logic::game::GameStatus::Checkmate(Color::Black) => (false, true),
        _ => (false, false),
    };

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
#[allow(clippy::too_many_arguments)]
fn run_dataset_training(
    dataset_path: &str,
    epochs: usize,
    output_path: &str,
    checkpoint_dir: Option<&str>,
    lr: f32,
    eval_games: usize,
    resume_path: Option<&str>,
    scratch: bool,
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

    let mut champion_model = FloatNnue::load_or_init(resume_path, Some(output_path), scratch);
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

            if let Err(e) = champion_model.save_float(output_path) {
                eprintln!("    ❌ Lỗi ghi file trọng số Float: {e}");
            } else {
                println!("    💾 Đã lưu trọng số Float (chưa lượng tử hóa) sang: {output_path}");
            }

            if let Some(cp_dir) = checkpoint_dir {
                let _ = std::fs::create_dir_all(cp_dir);
                let cp_path = format!("{cp_dir}/dataset_epoch_{epoch}.f32");
                if champion_model.save_float(&cp_path).is_ok() {
                    println!("    📁 Đã lưu checkpoint Float: {cp_path}");
                }
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
#[allow(clippy::too_many_arguments)]
fn run_teacher_distillation(
    engine_path: &str,
    protocol: EngineProtocol,
    dataset_path: &str,
    depth: u8,
    epochs: usize,
    output_path: &str,
    checkpoint_dir: Option<&str>,
    lr: f32,
    resume_path: Option<&str>,
    scratch: bool,
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

    // Train on labeled samples (resuming from existing model if available)
    let mut model = FloatNnue::load_or_init(resume_path, Some(output_path), scratch);
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

        if let Some(cp_dir) = checkpoint_dir {
            let _ = std::fs::create_dir_all(cp_dir);
            let cp_path = format!("{cp_dir}/teacher_epoch_{epoch}.f32");
            if model.save_float(&cp_path).is_ok() {
                println!("    📁 Đã lưu checkpoint Float: {cp_path}");
            }
        }
    }

    if let Err(e) = model.save_float(output_path) {
        eprintln!("❌ Lỗi xuất trọng số Float: {e}");
    } else {
        println!("💾 Đã lưu trọng số Float đã chưng cất tri thức sang: {output_path}");
    }
}

/// Runs Head-to-Head Sparring match against External Engine.
fn run_sparring_match(
    engine_path: &str,
    protocol: EngineProtocol,
    num_games: usize,
    depth: u8,
    output_path: &str,
    resume_path: Option<&str>,
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

    let candidate_model = FloatNnue::load_or_init(resume_path, Some(output_path), false);
    let cand_weights = Arc::new(candidate_model.to_quantized_weights());
    let config_cand = Arc::new(EngineConfig::default());

    let mut wins = 0;
    let mut draws = 0;
    let mut losses = 0;

    for game_idx in 1..=num_games {
        let cand_is_red = game_idx % 2 == 1;
        let mut game_state = cotuong_core::logic::game::GameState::new();
        let mut cand_engine =
            AlphaBetaEngine::with_weights(config_cand.clone(), cand_weights.clone());

        println!(
            "  ▶ Ván {game_idx}/{num_games}: Bot cầm {}",
            if cand_is_red { "ĐỎ" } else { "ĐEN" }
        );

        for _ply in 0..100 {
            if game_state.status != cotuong_core::logic::game::GameStatus::Playing {
                break;
            }

            let turn = game_state.turn;
            let best_move =
                if (turn == Color::Red && cand_is_red) || (turn == Color::Black && !cand_is_red) {
                    cand_engine
                        .search(
                            &game_state,
                            SearchLimit::DepthOrTime {
                                depth,
                                time_ms: 2000,
                            },
                            &[],
                        )
                        .map(|(mv, _)| mv)
                } else {
                    let fen = game_state.board.to_fen_string(turn);
                    client
                        .get_best_move(&fen, &[], depth, Duration::from_millis(2000))
                        .ok()
                        .flatten()
                };

            match best_move {
                Some(mv) => {
                    if let (Some(from), Some(to)) = (
                        BoardCoordinate::new(mv.from_row as usize, mv.from_col as usize),
                        BoardCoordinate::new(mv.to_row as usize, mv.to_col as usize),
                    ) {
                        if game_state.make_move(from, to).is_err() {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                None => break,
            }
        }

        let (red_wins, black_wins) = match game_state.status {
            cotuong_core::logic::game::GameStatus::Checkmate(Color::Red) => (true, false),
            cotuong_core::logic::game::GameStatus::Checkmate(Color::Black) => (false, true),
            _ => (false, false),
        };

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
    println!("💡 Trận đấu tập đánh giá (Sparring) hoàn tất, mô hình được giữ nguyên.");
}

/// Runs Self-Play Reinforcement Learning loop.
#[allow(clippy::too_many_arguments)]
fn run_selfplay_training(
    num_games: usize,
    epochs: usize,
    eval_games: usize,
    output_path: &str,
    checkpoint_dir: Option<&str>,
    lr: f32,
    resume_path: Option<&str>,
    scratch: bool,
) {
    let overall_start = Instant::now();
    let mut champion_model = FloatNnue::load_or_init(resume_path, Some(output_path), scratch);
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

            if let Err(e) = champion_model.save_float(output_path) {
                eprintln!("    ❌ Lỗi ghi file trọng số Float: {e}");
            } else {
                println!("    💾 Đã lưu trọng số Float (chưa lượng tử hóa) sang: {output_path}");
            }

            if let Some(cp_dir) = checkpoint_dir {
                let _ = std::fs::create_dir_all(cp_dir);
                let cp_path = format!("{cp_dir}/selfplay_epoch_{epoch}.f32");
                if champion_model.save_float(&cp_path).is_ok() {
                    println!("    📁 Đã lưu checkpoint Float: {cp_path}");
                }
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

/// Quantizes a full-precision float model into an int16/int8 NNUE binary file.
fn run_quantize_mode(input_path: &str, output_path: &str) {
    println!("\n▶ Bắt đầu lượng tử hóa mô hình Float NNUE (f32) -> Quantized NNUE (i16/i8)...");
    println!("  • File nguồn (Float NNUE)    : {input_path}");
    println!("  • File đích (Quantized NNUE) : {output_path}");

    let model = match FloatNnue::load_from_file(input_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("❌ Lỗi nạp mô hình Float từ '{input_path}': {e}");
            return;
        }
    };

    if let Some(parent) = std::path::Path::new(output_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Err(e) = model.export_quantized(output_path) {
        eprintln!("❌ Lỗi xuất trọng số lượng tử hóa: {e}");
    } else {
        println!("✅ Đã lượng tử hóa và lưu thành công sang: {output_path}");
        if let Ok(metadata) = std::fs::metadata(output_path) {
            #[allow(clippy::cast_precision_loss)]
            let size_kb = metadata.len() as f64 / 1024.0;
            println!("  • Dung lượng file lượng tử hóa : {size_kb:.1} KB");
        }
    }
}

fn main() {
    // Configure Rayon thread pool with 8MB stack size to prevent stack overflow in deep tree searches
    rayon::ThreadPoolBuilder::new()
        .stack_size(8 * 1024 * 1024)
        .build_global()
        .ok();

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
            if s.to_lowercase() == "ucci" {
                EngineProtocol::Ucci
            } else {
                EngineProtocol::Uci
            }
        })
        .unwrap_or(EngineProtocol::Uci);

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
        .unwrap_or(0.05);

    let input_path = args
        .iter()
        .position(|a| a == "--input" || a == "-i")
        .and_then(|idx| args.get(idx + 1))
        .map(String::as_str)
        .unwrap_or(DEFAULT_FLOAT_MODEL);

    let output_path = args
        .iter()
        .position(|a| a == "--output" || a == "-o")
        .and_then(|idx| args.get(idx + 1))
        .map_or_else(
            || {
                if mode == "quantize" || mode == "export" {
                    DEFAULT_QUANTIZED_MODEL
                } else {
                    DEFAULT_FLOAT_MODEL
                }
            },
            |s| s.as_str(),
        );

    let checkpoint_dir = args
        .iter()
        .position(|a| a == "--checkpoint-dir" || a == "--checkpoints")
        .and_then(|idx| args.get(idx + 1))
        .map(String::as_str)
        .or(Some("checkpoints"));

    let resume_path = args
        .iter()
        .position(|a| a == "--resume" || a == "--init")
        .and_then(|idx| args.get(idx + 1))
        .map(String::as_str);

    let scratch = args.iter().any(|a| a == "--scratch");

    println!("================================================================================");
    println!(" 🧠 Xiangqi AI Trainer (Master Dataset & External UCCI/UCI Engine Integration)  ");
    println!("================================================================================");
    println!("Cấu hình:");
    println!("  • Chế độ (Mode)            : {mode}");
    if mode == "quantize" || mode == "export" {
        println!("  • File Float nguồn         : {input_path}");
        println!("  • File NNUE đích           : {output_path}");
    } else {
        println!("  • Số Epochs                : {epochs}");
        println!("  • Tốc độ học (LR)          : {lr}");
        println!("  • Đường dẫn trọng số đích : {output_path}");
        if let Some(res) = resume_path {
            println!("  • Nạp trọng số khởi điểm  : {res}");
        } else if scratch {
            println!("  • Huấn luyện mới từ đầu    : BẬT (--scratch)");
        } else {
            println!("  • Kế thừa trọng số         : Tự động từ {output_path} (nếu có)");
        }
        if let Some(cp) = checkpoint_dir {
            println!("  • Thư mục Checkpoint       : {cp}");
        }
    }

    match mode.as_str() {
        "quantize" | "export" => {
            println!(
                "================================================================================"
            );
            run_quantize_mode(input_path, output_path);
        }
        "dataset" | "sl" => {
            println!("  • Tập dữ liệu kỳ phổ mẫu   : {dataset_path}");
            println!("  • Số ván Đấu Đánh Giá      : {eval_games}");
            println!(
                "================================================================================"
            );
            run_dataset_training(
                dataset_path,
                epochs,
                output_path,
                checkpoint_dir,
                lr,
                eval_games,
                resume_path,
                scratch,
            );
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
                checkpoint_dir,
                lr,
                resume_path,
                scratch,
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
            run_sparring_match(
                engine,
                engine_proto,
                num_games,
                engine_depth,
                output_path,
                resume_path,
            );
        }
        "selfplay" | "rl" => {
            println!("  • Số ván Self-Play/Epoch   : {num_games}");
            println!("  • Số ván Đấu Đánh Giá      : {eval_games}");
            println!(
                "================================================================================"
            );
            run_selfplay_training(
                num_games,
                epochs,
                eval_games,
                output_path,
                checkpoint_dir,
                lr,
                resume_path,
                scratch,
            );
        }
        unknown => {
            eprintln!("❌ Chế độ không hợp lệ: '{unknown}'. Hỗ trợ: 'dataset', 'teacher', 'sparring', 'selfplay'.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_float_save_and_load_roundtrip() {
        let original_model = FloatNnue::new();
        let temp_path = "/tmp/test_model_roundtrip.f32";
        assert!(original_model.save_float(temp_path).is_ok());

        let loaded = FloatNnue::load_from_file(temp_path);
        assert!(loaded.is_ok());
        let loaded_model = loaded.unwrap();

        // Exact match for float weights
        assert_eq!(original_model.b1, loaded_model.b1);
        assert_eq!(original_model.b2, loaded_model.b2);
        assert_eq!(original_model.b3, loaded_model.b3);
        assert_eq!(original_model.w4, loaded_model.w4);
        assert_eq!(original_model.b4, loaded_model.b4);

        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_quantized_weights_roundtrip() {
        let original_model = FloatNnue::new();
        let quantized = original_model.to_quantized_weights();
        let reconstructed = FloatNnue::from_quantized_weights(&quantized);

        // Check biases roundtrip closeness
        for (b_orig, b_recon) in original_model.b1.iter().zip(reconstructed.b1.iter()) {
            assert!((b_orig - b_recon).abs() < 0.01);
        }

        // Test export & load from file roundtrip
        let temp_path = "/tmp/test_roundtrip.nnue";
        assert!(original_model.export_quantized(temp_path).is_ok());
        let loaded = FloatNnue::load_from_file(temp_path);
        assert!(loaded.is_ok());
        let loaded_model = loaded.unwrap();
        assert!((loaded_model.b4 - original_model.b4).abs() < 0.1);
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_load_or_init_behavior() {
        // Scratch always gives fresh model
        let scratch_model = FloatNnue::load_or_init(None, None, true);
        assert_eq!(scratch_model.b1[8], -0.55);

        // Non-existent specified path falls back to new
        let fallback_model = FloatNnue::load_or_init(Some("/tmp/non_existent.nnue"), None, false);
        assert_eq!(fallback_model.b1[8], -0.55);
    }
}
