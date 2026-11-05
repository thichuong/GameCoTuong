use crate::logic::board::{Board, Color, Piece, PieceType};
use std::sync::OnceLock;

pub const NNUE_MAGIC: &[u8; 4] = b"XNNU";
pub const NNUE_VERSION: u32 = 2;

pub const L1_INPUT_SIZE: usize = 10_530; // 9 king palace squares * 13 piece types * 90 board squares
pub const L1_OUTPUT_SIZE: usize = 192;
pub const L2_INPUT_SIZE: usize = 384; // 2 * L1_OUTPUT_SIZE
pub const L2_OUTPUT_SIZE: usize = 32;
pub const L3_INPUT_SIZE: usize = 32;
pub const L3_OUTPUT_SIZE: usize = 32;

pub const FILE_SIZE: usize = 4
    + 4
    + (L1_INPUT_SIZE * L1_OUTPUT_SIZE * 2)
    + (L1_OUTPUT_SIZE * 2)
    + (L2_INPUT_SIZE * L2_OUTPUT_SIZE)
    + (L2_OUTPUT_SIZE * 4)
    + (L3_INPUT_SIZE * L3_OUTPUT_SIZE)
    + (L3_OUTPUT_SIZE * 4)
    + (L3_OUTPUT_SIZE * 2)
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
    pub w3: Box<[[i8; L3_INPUT_SIZE]; L3_OUTPUT_SIZE]>,
    pub b3: [i32; L3_OUTPUT_SIZE],
    pub w4: [i16; L3_OUTPUT_SIZE],
    pub b4: i32,
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
            w3: vec![[0i8; L3_INPUT_SIZE]; L3_OUTPUT_SIZE]
                .into_boxed_slice()
                .try_into()
                .unwrap_or_else(|_| Box::new([[0i8; L3_INPUT_SIZE]; L3_OUTPUT_SIZE])),
            b3: [0i32; L3_OUTPUT_SIZE],
            w4: [0i16; L3_OUTPUT_SIZE],
            b4: 0,
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

        let mut w3_vec = vec![[0i8; L3_INPUT_SIZE]; L3_OUTPUT_SIZE];
        for row in &mut w3_vec {
            for val in row {
                let b = bytes
                    .get(offset)
                    .ok_or_else(|| "Unexpected EOF while reading W3".to_string())?;
                #[allow(clippy::cast_possible_wrap)]
                {
                    *val = *b as i8;
                }
                offset += 1;
            }
        }
        let w3: Box<[[i8; L3_INPUT_SIZE]; L3_OUTPUT_SIZE]> =
            w3_vec
                .into_boxed_slice()
                .try_into()
                .map_err(|_| "Failed to allocate boxed W3 array".to_string())?;

        let mut b3 = [0i32; L3_OUTPUT_SIZE];
        for val in &mut b3 {
            let slice = bytes
                .get(offset..offset + 4)
                .ok_or_else(|| "Unexpected EOF while reading B3".to_string())?;
            *val = i32::from_le_bytes([
                *slice.first().unwrap_or(&0),
                *slice.get(1).unwrap_or(&0),
                *slice.get(2).unwrap_or(&0),
                *slice.get(3).unwrap_or(&0),
            ]);
            offset += 4;
        }

        let mut w4 = [0i16; L3_OUTPUT_SIZE];
        for val in &mut w4 {
            let slice = bytes
                .get(offset..offset + 2)
                .ok_or_else(|| "Unexpected EOF while reading W4".to_string())?;
            *val = i16::from_le_bytes([*slice.first().unwrap_or(&0), *slice.get(1).unwrap_or(&0)]);
            offset += 2;
        }

        let slice = bytes
            .get(offset..offset + 4)
            .ok_or_else(|| "Unexpected EOF while reading B4".to_string())?;
        let b4 = i32::from_le_bytes([
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
            w4,
            b4,
        })
    }
}

/// Maps square (0..90) to a palace index (0..9).
/// Red palace:
/// row 0, cols 3, 4, 5 -> 3, 4, 5 (0, 1, 2)
/// row 1, cols 3, 4, 5 -> 12, 13, 14 (3, 4, 5)
/// row 2, cols 3, 4, 5 -> 21, 22, 23 (6, 7, 8)
#[inline]
#[must_use]
pub fn palace_index(sq: usize) -> usize {
    match sq {
        3 => 0,
        4 => 1,
        5 => 2,
        12 => 3,
        13 => 4,
        14 => 5,
        21 => 6,
        22 => 7,
        23 => 8,
        _ => 1, // Fallback center palace (row 0, col 4)
    }
}

/// Maps a piece to a HalfKP type index in 0..13:
/// Friendly pieces (1..=6): Advisor=0, Elephant=1, Horse=2, Chariot=3, Cannon=4, Soldier=5.
/// Friendly General is None (the conditioning king).
/// Enemy pieces (0..=6): General=6, Advisor=7, Elephant=8, Horse=9, Chariot=10, Cannon=11, Soldier=12.
#[inline]
#[must_use]
pub fn halfkp_piece_type_idx(is_enemy: bool, piece_type: PieceType) -> Option<usize> {
    if is_enemy {
        Some(6 + piece_type.index())
    } else {
        match piece_type {
            PieceType::General => None,
            PieceType::Advisor => Some(0),
            PieceType::Elephant => Some(1),
            PieceType::Horse => Some(2),
            PieceType::Chariot => Some(3),
            PieceType::Cannon => Some(4),
            PieceType::Soldier => Some(5),
        }
    }
}

#[inline]
#[must_use]
pub fn halfkp_feature_idx_red(
    k_palace_idx: usize,
    is_black_piece: bool,
    piece_type: PieceType,
    sq: usize,
) -> Option<usize> {
    let p_type_idx = halfkp_piece_type_idx(is_black_piece, piece_type)?;
    let sq_clamped = sq.min(89);
    Some(k_palace_idx * 1170 + p_type_idx * 90 + sq_clamped)
}

#[inline]
#[must_use]
pub fn halfkp_feature_idx_black(
    k_palace_idx: usize,
    is_red_piece: bool,
    piece_type: PieceType,
    sq: usize,
) -> Option<usize> {
    let p_type_idx = halfkp_piece_type_idx(is_red_piece, piece_type)?;
    let flipped_sq = 89_usize.saturating_sub(sq.min(89));
    Some(k_palace_idx * 1170 + p_type_idx * 90 + flipped_sq)
}

#[inline]
#[must_use]
pub fn get_red_king_palace_idx(board: &Board) -> usize {
    if board.bitboards[0] != 0 {
        let sq = board.bitboards[0].trailing_zeros() as usize;
        palace_index(sq)
    } else {
        1
    }
}

#[inline]
#[must_use]
pub fn get_black_king_palace_idx(board: &Board) -> usize {
    if board.bitboards[7] != 0 {
        let sq = board.bitboards[7].trailing_zeros() as usize;
        let flipped_sq = 89_usize.saturating_sub(sq);
        palace_index(flipped_sq)
    } else {
        1
    }
}

impl NnueAccumulator {
    pub fn refresh(&mut self, board: &Board, weights: &NnueWeights) {
        let k_red = get_red_king_palace_idx(board);
        let k_black = get_black_king_palace_idx(board);
        self.refresh_red(&board.grid, k_red, weights);
        self.refresh_black(&board.grid, k_black, weights);
    }

    pub fn refresh_red(
        &mut self,
        grid: &[Option<Piece>; 90],
        k_red_palace_idx: usize,
        weights: &NnueWeights,
    ) {
        self.red = weights.b1;
        for (sq, maybe_piece) in grid.iter().enumerate() {
            if let Some(piece) = maybe_piece {
                if let Some(feat) = halfkp_feature_idx_red(
                    k_red_palace_idx,
                    piece.color == Color::Black,
                    piece.piece_type,
                    sq,
                ) {
                    if let Some(row) = weights.w1.get(feat) {
                        for (a, &w) in self.red.iter_mut().zip(row.iter()) {
                            *a = a.saturating_add(w);
                        }
                    }
                }
            }
        }
    }

    pub fn refresh_black(
        &mut self,
        grid: &[Option<Piece>; 90],
        k_black_palace_idx: usize,
        weights: &NnueWeights,
    ) {
        self.black = weights.b1;
        for (sq, maybe_piece) in grid.iter().enumerate() {
            if let Some(piece) = maybe_piece {
                if let Some(feat) = halfkp_feature_idx_black(
                    k_black_palace_idx,
                    piece.color == Color::Red,
                    piece.piece_type,
                    sq,
                ) {
                    if let Some(row) = weights.w1.get(feat) {
                        for (a, &w) in self.black.iter_mut().zip(row.iter()) {
                            *a = a.saturating_add(w);
                        }
                    }
                }
            }
        }
    }

    #[inline]
    pub fn add_piece(
        &mut self,
        piece: Piece,
        sq: usize,
        k_red: usize,
        k_black: usize,
        weights: &NnueWeights,
    ) {
        if let Some(red_feat) =
            halfkp_feature_idx_red(k_red, piece.color == Color::Black, piece.piece_type, sq)
        {
            if let Some(row) = weights.w1.get(red_feat) {
                for (a, &w) in self.red.iter_mut().zip(row.iter()) {
                    *a = a.saturating_add(w);
                }
            }
        }

        if let Some(black_feat) =
            halfkp_feature_idx_black(k_black, piece.color == Color::Red, piece.piece_type, sq)
        {
            if let Some(row) = weights.w1.get(black_feat) {
                for (a, &w) in self.black.iter_mut().zip(row.iter()) {
                    *a = a.saturating_add(w);
                }
            }
        }
    }

    #[inline]
    pub fn remove_piece(
        &mut self,
        piece: Piece,
        sq: usize,
        k_red: usize,
        k_black: usize,
        weights: &NnueWeights,
    ) {
        if let Some(red_feat) =
            halfkp_feature_idx_red(k_red, piece.color == Color::Black, piece.piece_type, sq)
        {
            if let Some(row) = weights.w1.get(red_feat) {
                for (a, &w) in self.red.iter_mut().zip(row.iter()) {
                    *a = a.saturating_sub(w);
                }
            }
        }

        if let Some(black_feat) =
            halfkp_feature_idx_black(k_black, piece.color == Color::Red, piece.piece_type, sq)
        {
            if let Some(row) = weights.w1.get(black_feat) {
                for (a, &w) in self.black.iter_mut().zip(row.iter()) {
                    *a = a.saturating_sub(w);
                }
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
    // 1. Layer 1 activation (CReLU: clamp [0, 256])
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

    // 2. Layer 2: 384 -> 32
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

    // 3. Layer 3: 32 -> 32
    let mut l3_output = [0i32; L3_OUTPUT_SIZE];
    for (j, (out, &b)) in l3_output.iter_mut().zip(weights.b3.iter()).enumerate() {
        if let Some(w_row) = weights.w3.get(j) {
            let mut sum = b;
            for (&inp, &w) in l2_output.iter().zip(w_row.iter()) {
                sum = sum.saturating_add(inp * i32::from(w));
            }
            *out = (sum / 64).clamp(0, 127);
        }
    }

    // 4. Layer 4: 32 -> 1
    let mut final_sum = weights.b4;
    for (&val, &w) in l3_output.iter().zip(weights.w4.iter()) {
        final_sum = final_sum.saturating_add(val * i32::from(w));
    }

    final_sum / 16
}

#[must_use]
pub fn evaluate_nnue_with_weights(board: &Board, weights: &NnueWeights) -> i32 {
    let red_score = evaluate_forward(&board.nnue_acc.red, &board.nnue_acc.black, weights);
    let black_score = evaluate_forward(&board.nnue_acc.black, &board.nnue_acc.red, weights);
    (red_score - black_score) / 2
}

#[must_use]
pub fn evaluate_nnue(board: &Board) -> i32 {
    evaluate_nnue_with_weights(board, NnueWeights::get())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_weights_validity() {
        let weights = NnueWeights::get();
        assert_eq!(weights.b1.len(), L1_OUTPUT_SIZE);
        assert_eq!(weights.b2.len(), L2_OUTPUT_SIZE);
        assert_eq!(weights.b3.len(), L3_OUTPUT_SIZE);
        assert_eq!(weights.w4.len(), L3_OUTPUT_SIZE);
    }

    #[test]
    fn test_palace_mapping() {
        // Red palace squares
        assert_eq!(palace_index(3), 0);
        assert_eq!(palace_index(4), 1);
        assert_eq!(palace_index(5), 2);
        assert_eq!(palace_index(12), 3);
        assert_eq!(palace_index(13), 4);
        assert_eq!(palace_index(14), 5);
        assert_eq!(palace_index(21), 6);
        assert_eq!(palace_index(22), 7);
        assert_eq!(palace_index(23), 8);

        // Black palace squares flipped 89 - sq
        assert_eq!(palace_index(89 - 85), 1); // Row 9, Col 4
        assert_eq!(palace_index(89 - 84), 2); // Row 9, Col 3
        assert_eq!(palace_index(89 - 86), 0); // Row 9, Col 5
    }

    #[test]
    fn test_halfkp_feature_bounds() {
        // Red palace idx 0..8
        for k in 0..9 {
            // Friendly Chariot at sq 0
            let feat = halfkp_feature_idx_red(k, false, PieceType::Chariot, 0).expect("Valid feat");
            assert!(feat < L1_INPUT_SIZE);

            // Enemy General at sq 85
            let feat_eg =
                halfkp_feature_idx_red(k, true, PieceType::General, 85).expect("Valid feat");
            assert!(feat_eg < L1_INPUT_SIZE);
        }
    }

    #[test]
    fn test_accumulator_incremental_vs_refresh() {
        use crate::logic::board::{Board, BoardCoordinate, Color};
        use crate::logic::generator::MoveGenerator;

        let mut board = Board::new();
        let mg = MoveGenerator::new();

        // 1. Check initial accumulator matches fresh refresh
        let mut expected = board.clone();
        expected.refresh_nnue();
        assert_eq!(board.nnue_acc.red, expected.nnue_acc.red);
        assert_eq!(board.nnue_acc.black, expected.nnue_acc.black);

        // 2. Play 10 moves, verifying incremental match after each move
        let mut history = Vec::new();
        let mut turn = Color::Red;

        for _ in 0..10 {
            let moves = mg.generate_moves(&board, turn);
            if moves.is_empty() {
                break;
            }
            let mv = moves[0];
            let captured = board
                .get_piece(BoardCoordinate::new(mv.to_row as usize, mv.to_col as usize).unwrap());
            board.apply_move(&mv, turn);
            history.push((mv, captured, turn));

            let mut fresh = board.clone();
            fresh.refresh_nnue();
            assert_eq!(board.nnue_acc.red, fresh.nnue_acc.red);
            assert_eq!(board.nnue_acc.black, fresh.nnue_acc.black);

            turn = turn.opposite();
        }

        // 3. Undo all moves and verify back to start
        while let Some((mv, cap, t)) = history.pop() {
            board.undo_move(&mv, cap, t);
            let mut fresh = board.clone();
            fresh.refresh_nnue();
            assert_eq!(board.nnue_acc.red, fresh.nnue_acc.red);
            assert_eq!(board.nnue_acc.black, fresh.nnue_acc.black);
        }
    }
}
