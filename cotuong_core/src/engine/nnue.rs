use crate::logic::board::{Board, Color, Piece, PieceType};
use std::sync::OnceLock;

pub const NNUE_MAGIC: &[u8; 4] = b"XNNU";
pub const NNUE_VERSION: u32 = 1;

pub const L1_INPUT_SIZE: usize = 1260;
pub const L1_OUTPUT_SIZE: usize = 256;
pub const L2_INPUT_SIZE: usize = 512;
pub const L2_OUTPUT_SIZE: usize = 32;

pub const FILE_SIZE: usize = 4
    + 4
    + (L1_INPUT_SIZE * L1_OUTPUT_SIZE * 2)
    + (L1_OUTPUT_SIZE * 2)
    + (L2_INPUT_SIZE * L2_OUTPUT_SIZE)
    + (L2_OUTPUT_SIZE * 4)
    + (L2_OUTPUT_SIZE * 2)
    + 4;

const EMBEDDED_WEIGHTS: &[u8] = include_bytes!("../../assets/default.nnue");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NnueAccumulator {
    pub red: [i16; L1_OUTPUT_SIZE],
    pub black: [i16; L1_OUTPUT_SIZE],
}

impl Default for NnueAccumulator {
    fn default() -> Self {
        Self {
            red: [0; L1_OUTPUT_SIZE],
            black: [0; L1_OUTPUT_SIZE],
        }
    }
}

pub struct NnueWeights {
    pub w1: Box<[[i16; L1_OUTPUT_SIZE]; L1_INPUT_SIZE]>,
    pub b1: [i16; L1_OUTPUT_SIZE],
    pub w2: Box<[[i8; L2_INPUT_SIZE]; L2_OUTPUT_SIZE]>,
    pub b2: [i32; L2_OUTPUT_SIZE],
    pub w3: [i16; L2_OUTPUT_SIZE],
    pub b3: i32,
}

static NNUE_INSTANCE: OnceLock<NnueWeights> = OnceLock::new();

impl NnueWeights {
    #[must_use]
    pub fn get() -> &'static Self {
        NNUE_INSTANCE.get_or_init(|| {
            Self::load_from_bytes(EMBEDDED_WEIGHTS).unwrap_or_else(|_| Self::empty())
        })
    }

    #[must_use]
    pub fn empty() -> Self {
        Self {
            w1: vec![[0i16; L1_OUTPUT_SIZE]; L1_INPUT_SIZE]
                .into_boxed_slice()
                .try_into()
                .unwrap_or_else(|_| Box::new([[0i16; L1_OUTPUT_SIZE]; L1_INPUT_SIZE])),
            b1: [0i16; L1_OUTPUT_SIZE],
            w2: vec![[0i8; L2_INPUT_SIZE]; L2_OUTPUT_SIZE]
                .into_boxed_slice()
                .try_into()
                .unwrap_or_else(|_| Box::new([[0i8; L2_INPUT_SIZE]; L2_OUTPUT_SIZE])),
            b2: [0i32; L2_OUTPUT_SIZE],
            w3: [0i16; L2_OUTPUT_SIZE],
            b3: 0,
        }
    }

    pub fn load_from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != FILE_SIZE {
            return Err(format!(
                "Invalid NNUE file size: expected {}, got {}",
                FILE_SIZE,
                bytes.len()
            ));
        }

        let magic = bytes
            .get(0..4)
            .ok_or_else(|| "Failed to read magic".to_string())?;
        if magic != NNUE_MAGIC {
            return Err("Invalid NNUE magic header".to_string());
        }

        let ver_bytes = bytes
            .get(4..8)
            .ok_or_else(|| "Failed to read version".to_string())?;
        let ver = u32::from_le_bytes([
            *ver_bytes.first().unwrap_or(&0),
            *ver_bytes.get(1).unwrap_or(&0),
            *ver_bytes.get(2).unwrap_or(&0),
            *ver_bytes.get(3).unwrap_or(&0),
        ]);
        if ver != NNUE_VERSION {
            return Err(format!("Unsupported NNUE version: {ver}"));
        }

        let mut offset = 8;
        let mut w1_vec = vec![[0i16; L1_OUTPUT_SIZE]; L1_INPUT_SIZE];
        for row in &mut w1_vec {
            for val in row {
                let slice = bytes
                    .get(offset..offset + 2)
                    .ok_or_else(|| "Unexpected EOF while reading W1".to_string())?;
                *val =
                    i16::from_le_bytes([*slice.first().unwrap_or(&0), *slice.get(1).unwrap_or(&0)]);
                offset += 2;
            }
        }
        let w1: Box<[[i16; L1_OUTPUT_SIZE]; L1_INPUT_SIZE]> = w1_vec
            .into_boxed_slice()
            .try_into()
            .map_err(|_| "Failed to allocate boxed W1 array".to_string())?;

        let mut b1 = [0i16; L1_OUTPUT_SIZE];
        for val in &mut b1 {
            let slice = bytes
                .get(offset..offset + 2)
                .ok_or_else(|| "Unexpected EOF while reading B1".to_string())?;
            *val = i16::from_le_bytes([*slice.first().unwrap_or(&0), *slice.get(1).unwrap_or(&0)]);
            offset += 2;
        }

        let mut w2_vec = vec![[0i8; L2_INPUT_SIZE]; L2_OUTPUT_SIZE];
        for row in &mut w2_vec {
            for val in row {
                let b = bytes
                    .get(offset)
                    .ok_or_else(|| "Unexpected EOF while reading W2".to_string())?;
                #[allow(clippy::cast_possible_wrap)]
                {
                    *val = *b as i8;
                }
                offset += 1;
            }
        }
        let w2: Box<[[i8; L2_INPUT_SIZE]; L2_OUTPUT_SIZE]> =
            w2_vec
                .into_boxed_slice()
                .try_into()
                .map_err(|_| "Failed to allocate boxed W2 array".to_string())?;

        let mut b2 = [0i32; L2_OUTPUT_SIZE];
        for val in &mut b2 {
            let slice = bytes
                .get(offset..offset + 4)
                .ok_or_else(|| "Unexpected EOF while reading B2".to_string())?;
            *val = i32::from_le_bytes([
                *slice.first().unwrap_or(&0),
                *slice.get(1).unwrap_or(&0),
                *slice.get(2).unwrap_or(&0),
                *slice.get(3).unwrap_or(&0),
            ]);
            offset += 4;
        }

        let mut w3 = [0i16; L2_OUTPUT_SIZE];
        for val in &mut w3 {
            let slice = bytes
                .get(offset..offset + 2)
                .ok_or_else(|| "Unexpected EOF while reading W3".to_string())?;
            *val = i16::from_le_bytes([*slice.first().unwrap_or(&0), *slice.get(1).unwrap_or(&0)]);
            offset += 2;
        }

        let slice = bytes
            .get(offset..offset + 4)
            .ok_or_else(|| "Unexpected EOF while reading B3".to_string())?;
        let b3 = i32::from_le_bytes([
            *slice.first().unwrap_or(&0),
            *slice.get(1).unwrap_or(&0),
            *slice.get(2).unwrap_or(&0),
            *slice.get(3).unwrap_or(&0),
        ]);

        Ok(Self {
            w1,
            b1,
            w2,
            b2,
            w3,
            b3,
        })
    }
}

#[inline]
#[must_use]
pub fn piece_feature_idx_red(color: Color, piece_type: PieceType, sq: usize) -> usize {
    let color_offset = match color {
        Color::Red => 0,
        Color::Black => 7,
    };
    let piece_idx = color_offset + piece_type.index();
    piece_idx * 90 + sq.min(89)
}

#[inline]
#[must_use]
pub fn piece_feature_idx_black(color: Color, piece_type: PieceType, sq: usize) -> usize {
    let color_offset = match color {
        Color::Black => 0,
        Color::Red => 7,
    };
    let piece_idx = color_offset + piece_type.index();
    let flipped_sq = 89_usize.saturating_sub(sq.min(89));
    piece_idx * 90 + flipped_sq
}

impl NnueAccumulator {
    pub fn refresh(&mut self, grid: &[Option<Piece>; 90], weights: &NnueWeights) {
        self.red = weights.b1;
        self.black = weights.b1;

        for (sq, maybe_piece) in grid.iter().enumerate() {
            if let Some(piece) = maybe_piece {
                self.add_piece(*piece, sq, weights);
            }
        }
    }

    #[inline]
    pub fn add_piece(&mut self, piece: Piece, sq: usize, weights: &NnueWeights) {
        let red_idx = piece_feature_idx_red(piece.color, piece.piece_type, sq);
        if let Some(row) = weights.w1.get(red_idx) {
            for (acc_val, &w) in self.red.iter_mut().zip(row.iter()) {
                *acc_val = acc_val.saturating_add(w);
            }
        }

        let black_idx = piece_feature_idx_black(piece.color, piece.piece_type, sq);
        if let Some(row) = weights.w1.get(black_idx) {
            for (acc_val, &w) in self.black.iter_mut().zip(row.iter()) {
                *acc_val = acc_val.saturating_add(w);
            }
        }
    }

    #[inline]
    pub fn remove_piece(&mut self, piece: Piece, sq: usize, weights: &NnueWeights) {
        let red_idx = piece_feature_idx_red(piece.color, piece.piece_type, sq);
        if let Some(row) = weights.w1.get(red_idx) {
            for (acc_val, &w) in self.red.iter_mut().zip(row.iter()) {
                *acc_val = acc_val.saturating_sub(w);
            }
        }

        let black_idx = piece_feature_idx_black(piece.color, piece.piece_type, sq);
        if let Some(row) = weights.w1.get(black_idx) {
            for (acc_val, &w) in self.black.iter_mut().zip(row.iter()) {
                *acc_val = acc_val.saturating_sub(w);
            }
        }
    }
}

#[must_use]
pub fn evaluate_forward(
    active_acc: &[i16; L1_OUTPUT_SIZE],
    opp_acc: &[i16; L1_OUTPUT_SIZE],
    weights: &NnueWeights,
) -> i32 {
    let mut l2_input = [0i32; L2_INPUT_SIZE];
    if let Some(left) = l2_input.get_mut(..L1_OUTPUT_SIZE) {
        for (out, &val) in left.iter_mut().zip(active_acc.iter()) {
            *out = i32::from(val.clamp(0, 256));
        }
    }
    if let Some(right) = l2_input.get_mut(L1_OUTPUT_SIZE..) {
        for (out, &val) in right.iter_mut().zip(opp_acc.iter()) {
            *out = i32::from(val.clamp(0, 256));
        }
    }

    let mut l2_output = [0i32; L2_OUTPUT_SIZE];
    for (j, (out, &b)) in l2_output.iter_mut().zip(weights.b2.iter()).enumerate() {
        if let Some(w_row) = weights.w2.get(j) {
            let mut sum = b;
            for (&inp, &w) in l2_input.iter().zip(w_row.iter()) {
                sum = sum.saturating_add(inp * i32::from(w));
            }
            *out = (sum / 64).clamp(0, 127);
        }
    }

    let mut final_sum = weights.b3;
    for (&val, &w) in l2_output.iter().zip(weights.w3.iter()) {
        final_sum = final_sum.saturating_add(val * i32::from(w));
    }

    final_sum / 16
}

#[must_use]
pub fn evaluate_nnue(board: &Board) -> i32 {
    let weights = NnueWeights::get();
    let red_score = evaluate_forward(&board.nnue_acc.red, &board.nnue_acc.black, weights);
    let black_score = evaluate_forward(&board.nnue_acc.black, &board.nnue_acc.red, weights);
    (red_score - black_score) / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_weights_validity() {
        let weights = NnueWeights::get();
        assert_eq!(weights.b1.len(), L1_OUTPUT_SIZE);
        assert_eq!(weights.b2.len(), L2_OUTPUT_SIZE);
        assert_eq!(weights.w3.len(), L2_OUTPUT_SIZE);
    }

    #[test]
    fn test_feature_symmetry() {
        // Red king at bottom (row 0, col 4 -> sq = 4)
        let red_k_feat = piece_feature_idx_red(Color::Red, PieceType::General, 4);
        assert_eq!(red_k_feat, 4);

        // Black king at top (row 9, col 4 -> sq = 85)
        // From Black perspective: friendly general at flipped sq = 89 - 85 = 4
        let black_k_feat = piece_feature_idx_black(Color::Black, PieceType::General, 85);
        assert_eq!(black_k_feat, 4);
    }

    #[test]
    fn test_incremental_accumulator_exactness() {
        let mut board = Board::new();
        let weights = NnueWeights::get();

        // Check starting position
        let mut expected = NnueAccumulator::default();
        expected.refresh(&board.grid, weights);
        assert_eq!(board.nnue_acc, expected);

        // Move 1: Red Cannon (row 2, col 1) to (row 2, col 4)
        let mv1 = crate::engine::Move {
            from_row: 2,
            from_col: 1,
            to_row: 2,
            to_col: 4,
            score: 0,
        };
        board.apply_move(&mv1, Color::Red);
        expected.refresh(&board.grid, weights);
        assert_eq!(board.nnue_acc, expected);

        // Move 2: Black Horse (row 9, col 1) to (row 7, col 2)
        let mv2 = crate::engine::Move {
            from_row: 9,
            from_col: 1,
            to_row: 7,
            to_col: 2,
            score: 0,
        };
        board.apply_move(&mv2, Color::Black);
        expected.refresh(&board.grid, weights);
        assert_eq!(board.nnue_acc, expected);

        // Move 3: Red Cannon captures Black Soldier at (row 6, col 4)
        let captured =
            board.get_piece(crate::logic::board::BoardCoordinate::new(6, 4).expect("Valid square"));
        let mv3 = crate::engine::Move {
            from_row: 2,
            from_col: 4,
            to_row: 6,
            to_col: 4,
            score: 0,
        };
        board.apply_move(&mv3, Color::Red);
        expected.refresh(&board.grid, weights);
        assert_eq!(board.nnue_acc, expected);

        // Undo Move 3 (Capture)
        board.undo_move(&mv3, captured, Color::Red);
        expected.refresh(&board.grid, weights);
        assert_eq!(board.nnue_acc, expected);

        // Undo Move 2
        board.undo_move(&mv2, None, Color::Black);
        expected.refresh(&board.grid, weights);
        assert_eq!(board.nnue_acc, expected);

        // Undo Move 1
        board.undo_move(&mv1, None, Color::Red);
        expected.refresh(&board.grid, weights);
        assert_eq!(board.nnue_acc, expected);
    }
}
