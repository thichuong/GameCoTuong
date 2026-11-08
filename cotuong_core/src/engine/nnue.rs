use crate::logic::board::{Board, Color, Piece, PieceType};
use std::sync::OnceLock;

pub const NNUE_MAGIC: &[u8; 4] = b"XNNU";
pub const NNUE_VERSION: u32 = 3;
pub const FLOAT_NNUE_MAGIC: &[u8; 8] = b"COTF32_1";

pub const L1_INPUT_SIZE: usize = 10_530; // 9 king palace squares * 13 piece types * 90 board squares
pub const L1_OUTPUT_SIZE: usize = 192;
pub const PSQT_BUCKETS: usize = 8; // 8 game-phase material & king-relative positional buckets (Pikafish style)
pub const L2_INPUT_SIZE: usize = 384; // 2 * L1_OUTPUT_SIZE
pub const L2_OUTPUT_SIZE: usize = 32;
pub const L3_INPUT_SIZE: usize = 32;
pub const L3_OUTPUT_SIZE: usize = 32;

pub const FILE_SIZE: usize = 4
    + 4
    + (L1_INPUT_SIZE * L1_OUTPUT_SIZE * 2)
    + (L1_OUTPUT_SIZE * 2)
    + (L1_INPUT_SIZE * PSQT_BUCKETS * 2)
    + (L2_INPUT_SIZE * L2_OUTPUT_SIZE)
    + (L2_OUTPUT_SIZE * 4)
    + (L3_INPUT_SIZE * L3_OUTPUT_SIZE)
    + (L3_OUTPUT_SIZE * 4)
    + (L3_OUTPUT_SIZE * 2)
    + 4;

pub const FLOAT_FILE_SIZE: usize = 8
    + (L1_INPUT_SIZE * L1_OUTPUT_SIZE * 4)
    + (L1_OUTPUT_SIZE * 4)
    + (L1_INPUT_SIZE * PSQT_BUCKETS * 4)
    + (L2_INPUT_SIZE * L2_OUTPUT_SIZE * 4)
    + (L2_OUTPUT_SIZE * 4)
    + (L3_INPUT_SIZE * L3_OUTPUT_SIZE * 4)
    + (L3_OUTPUT_SIZE * 4)
    + (L3_OUTPUT_SIZE * 4)
    + 4;

const EMBEDDED_WEIGHTS: &[u8] = include_bytes!("../../assets/default.nnue");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NnueAccumulator {
    pub red: [i16; L1_OUTPUT_SIZE],
    pub black: [i16; L1_OUTPUT_SIZE],
    pub psqt_red: [i32; PSQT_BUCKETS],
    pub psqt_black: [i32; PSQT_BUCKETS],
}

impl Default for NnueAccumulator {
    fn default() -> Self {
        Self {
            red: [0; L1_OUTPUT_SIZE],
            black: [0; L1_OUTPUT_SIZE],
            psqt_red: [0; PSQT_BUCKETS],
            psqt_black: [0; PSQT_BUCKETS],
        }
    }
}

pub struct NnueWeights {
    pub w1: Box<[[i16; L1_OUTPUT_SIZE]; L1_INPUT_SIZE]>,
    pub b1: [i16; L1_OUTPUT_SIZE],
    pub w_psqt: Box<[[i16; PSQT_BUCKETS]; L1_INPUT_SIZE]>,
    pub w2: Box<[[i8; L2_INPUT_SIZE]; L2_OUTPUT_SIZE]>,
    /// Pre-transposed column-major W2 (`[L2_INPUT_SIZE][L2_OUTPUT_SIZE]`) for sparse SIMD-friendly forward pass
    pub w2_col: Box<[[i8; L2_OUTPUT_SIZE]; L2_INPUT_SIZE]>,
    pub b2: [i32; L2_OUTPUT_SIZE],
    pub w3: Box<[[i8; L3_INPUT_SIZE]; L3_OUTPUT_SIZE]>,
    pub w3_col: Box<[[i8; L3_OUTPUT_SIZE]; L3_INPUT_SIZE]>,
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
        let Ok(w1) = vec![[0i16; L1_OUTPUT_SIZE]; L1_INPUT_SIZE]
            .into_boxed_slice()
            .try_into()
        else {
            unreachable!()
        };
        let Ok(w_psqt) = vec![[0i16; PSQT_BUCKETS]; L1_INPUT_SIZE]
            .into_boxed_slice()
            .try_into()
        else {
            unreachable!()
        };
        let Ok(w2) = vec![[0i8; L2_INPUT_SIZE]; L2_OUTPUT_SIZE]
            .into_boxed_slice()
            .try_into()
        else {
            unreachable!()
        };
        let Ok(w2_col) = vec![[0i8; L2_OUTPUT_SIZE]; L2_INPUT_SIZE]
            .into_boxed_slice()
            .try_into()
        else {
            unreachable!()
        };
        let Ok(w3) = vec![[0i8; L3_INPUT_SIZE]; L3_OUTPUT_SIZE]
            .into_boxed_slice()
            .try_into()
        else {
            unreachable!()
        };
        let Ok(w3_col) = vec![[0i8; L3_OUTPUT_SIZE]; L3_INPUT_SIZE]
            .into_boxed_slice()
            .try_into()
        else {
            unreachable!()
        };
        Self {
            w1,
            b1: [0i16; L1_OUTPUT_SIZE],
            w_psqt,
            w2,
            w2_col,
            b2: [0i32; L2_OUTPUT_SIZE],
            w3,
            w3_col,
            b3: [0i32; L3_OUTPUT_SIZE],
            w4: [0i16; L3_OUTPUT_SIZE],
            b4: 0,
        }
    }

    #[must_use]
    pub fn transpose_w2(
        w2: &[[i8; L2_INPUT_SIZE]; L2_OUTPUT_SIZE],
    ) -> Box<[[i8; L2_OUTPUT_SIZE]; L2_INPUT_SIZE]> {
        let mut w2_col_vec = vec![[0i8; L2_OUTPUT_SIZE]; L2_INPUT_SIZE];
        for (j, row) in w2.iter().enumerate() {
            for (i, &val) in row.iter().enumerate() {
                if let Some(col_slice) = w2_col_vec.get_mut(i) {
                    if let Some(cell) = col_slice.get_mut(j) {
                        *cell = val;
                    }
                }
            }
        }
        let Ok(boxed) = w2_col_vec.into_boxed_slice().try_into() else {
            unreachable!()
        };
        boxed
    }

    #[must_use]
    pub fn transpose_w3(
        w3: &[[i8; L3_INPUT_SIZE]; L3_OUTPUT_SIZE],
    ) -> Box<[[i8; L3_OUTPUT_SIZE]; L3_INPUT_SIZE]> {
        let mut w3_col_vec = vec![[0i8; L3_OUTPUT_SIZE]; L3_INPUT_SIZE];
        for (k, row) in w3.iter().enumerate() {
            for (j, &val) in row.iter().enumerate() {
                if let Some(col_slice) = w3_col_vec.get_mut(j) {
                    if let Some(cell) = col_slice.get_mut(k) {
                        *cell = val;
                    }
                }
            }
        }
        let Ok(boxed) = w3_col_vec.into_boxed_slice().try_into() else {
            unreachable!()
        };
        boxed
    }

    pub fn load_from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.starts_with(FLOAT_NNUE_MAGIC) {
            Self::load_float(bytes)
        } else {
            Self::load_quantized(bytes)
        }
    }

    fn load_float(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != FLOAT_FILE_SIZE {
            return Err(format!(
                "Invalid Float NNUE file size: expected {}, got {}",
                FLOAT_FILE_SIZE,
                bytes.len()
            ));
        }

        let mut offset = 8;
        let mut w1_vec = vec![[0i16; L1_OUTPUT_SIZE]; L1_INPUT_SIZE];
        for row in &mut w1_vec {
            for val in row {
                let slice = bytes
                    .get(offset..offset + 4)
                    .ok_or_else(|| "Unexpected EOF while reading Float W1".to_string())?;
                let f = f32::from_le_bytes([
                    *slice.first().unwrap_or(&0),
                    *slice.get(1).unwrap_or(&0),
                    *slice.get(2).unwrap_or(&0),
                    *slice.get(3).unwrap_or(&0),
                ]);
                #[allow(clippy::cast_possible_truncation)]
                {
                    *val = (f * 256.0).clamp(-32767.0, 32767.0) as i16;
                }
                offset += 4;
            }
        }
        let w1: Box<[[i16; L1_OUTPUT_SIZE]; L1_INPUT_SIZE]> = w1_vec
            .into_boxed_slice()
            .try_into()
            .map_err(|_| "Failed to allocate boxed W1 array".to_string())?;

        let mut b1 = [0i16; L1_OUTPUT_SIZE];
        for val in &mut b1 {
            let slice = bytes
                .get(offset..offset + 4)
                .ok_or_else(|| "Unexpected EOF while reading Float B1".to_string())?;
            let f = f32::from_le_bytes([
                *slice.first().unwrap_or(&0),
                *slice.get(1).unwrap_or(&0),
                *slice.get(2).unwrap_or(&0),
                *slice.get(3).unwrap_or(&0),
            ]);
            #[allow(clippy::cast_possible_truncation)]
            {
                *val = (f * 256.0).clamp(-32767.0, 32767.0) as i16;
            }
            offset += 4;
        }

        let mut w_psqt_vec = vec![[0i16; PSQT_BUCKETS]; L1_INPUT_SIZE];
        for row in &mut w_psqt_vec {
            for val in row {
                let slice = bytes
                    .get(offset..offset + 4)
                    .ok_or_else(|| "Unexpected EOF while reading Float W_PSQT".to_string())?;
                let f = f32::from_le_bytes([
                    *slice.first().unwrap_or(&0),
                    *slice.get(1).unwrap_or(&0),
                    *slice.get(2).unwrap_or(&0),
                    *slice.get(3).unwrap_or(&0),
                ]);
                #[allow(clippy::cast_possible_truncation)]
                {
                    *val = (f * 16.0).clamp(-32767.0, 32767.0) as i16;
                }
                offset += 4;
            }
        }
        let w_psqt: Box<[[i16; PSQT_BUCKETS]; L1_INPUT_SIZE]> = w_psqt_vec
            .into_boxed_slice()
            .try_into()
            .map_err(|_| "Failed to allocate boxed W_PSQT array".to_string())?;

        let mut w2_vec = vec![[0i8; L2_INPUT_SIZE]; L2_OUTPUT_SIZE];
        for row in &mut w2_vec {
            for val in row {
                let slice = bytes
                    .get(offset..offset + 4)
                    .ok_or_else(|| "Unexpected EOF while reading Float W2".to_string())?;
                let f = f32::from_le_bytes([
                    *slice.first().unwrap_or(&0),
                    *slice.get(1).unwrap_or(&0),
                    *slice.get(2).unwrap_or(&0),
                    *slice.get(3).unwrap_or(&0),
                ]);
                #[allow(clippy::cast_possible_truncation)]
                {
                    *val = (f * 64.0).clamp(-127.0, 127.0) as i8;
                }
                offset += 4;
            }
        }
        let w2: Box<[[i8; L2_INPUT_SIZE]; L2_OUTPUT_SIZE]> =
            w2_vec
                .into_boxed_slice()
                .try_into()
                .map_err(|_| "Failed to allocate boxed W2 array".to_string())?;
        let w2_col = Self::transpose_w2(&w2);

        let mut b2 = [0i32; L2_OUTPUT_SIZE];
        for val in &mut b2 {
            let slice = bytes
                .get(offset..offset + 4)
                .ok_or_else(|| "Unexpected EOF while reading Float B2".to_string())?;
            let f = f32::from_le_bytes([
                *slice.first().unwrap_or(&0),
                *slice.get(1).unwrap_or(&0),
                *slice.get(2).unwrap_or(&0),
                *slice.get(3).unwrap_or(&0),
            ]);
            #[allow(clippy::cast_possible_truncation)]
            {
                *val = (f * 16384.0).clamp(-2_000_000_000.0, 2_000_000_000.0) as i32;
            }
            offset += 4;
        }

        let mut w3_vec = vec![[0i8; L3_INPUT_SIZE]; L3_OUTPUT_SIZE];
        for row in &mut w3_vec {
            for val in row {
                let slice = bytes
                    .get(offset..offset + 4)
                    .ok_or_else(|| "Unexpected EOF while reading Float W3".to_string())?;
                let f = f32::from_le_bytes([
                    *slice.first().unwrap_or(&0),
                    *slice.get(1).unwrap_or(&0),
                    *slice.get(2).unwrap_or(&0),
                    *slice.get(3).unwrap_or(&0),
                ]);
                #[allow(clippy::cast_possible_truncation)]
                {
                    *val = (f * 64.0).clamp(-127.0, 127.0) as i8;
                }
                offset += 4;
            }
        }
        let w3: Box<[[i8; L3_INPUT_SIZE]; L3_OUTPUT_SIZE]> =
            w3_vec
                .into_boxed_slice()
                .try_into()
                .map_err(|_| "Failed to allocate boxed W3 array".to_string())?;
        let w3_col = Self::transpose_w3(&w3);

        let mut b3 = [0i32; L3_OUTPUT_SIZE];
        for val in &mut b3 {
            let slice = bytes
                .get(offset..offset + 4)
                .ok_or_else(|| "Unexpected EOF while reading Float B3".to_string())?;
            let f = f32::from_le_bytes([
                *slice.first().unwrap_or(&0),
                *slice.get(1).unwrap_or(&0),
                *slice.get(2).unwrap_or(&0),
                *slice.get(3).unwrap_or(&0),
            ]);
            #[allow(clippy::cast_possible_truncation)]
            {
                *val = (f * 8192.0).clamp(-2_000_000_000.0, 2_000_000_000.0) as i32;
            }
            offset += 4;
        }

        let mut w4 = [0i16; L3_OUTPUT_SIZE];
        for val in &mut w4 {
            let slice = bytes
                .get(offset..offset + 4)
                .ok_or_else(|| "Unexpected EOF while reading Float W4".to_string())?;
            let f = f32::from_le_bytes([
                *slice.first().unwrap_or(&0),
                *slice.get(1).unwrap_or(&0),
                *slice.get(2).unwrap_or(&0),
                *slice.get(3).unwrap_or(&0),
            ]);
            #[allow(clippy::cast_possible_truncation)]
            {
                *val = (f * 16.0).clamp(-32767.0, 32767.0) as i16;
            }
            offset += 4;
        }

        let slice = bytes
            .get(offset..offset + 4)
            .ok_or_else(|| "Unexpected EOF while reading Float B4".to_string())?;
        let f_b4 = f32::from_le_bytes([
            *slice.first().unwrap_or(&0),
            *slice.get(1).unwrap_or(&0),
            *slice.get(2).unwrap_or(&0),
            *slice.get(3).unwrap_or(&0),
        ]);
        #[allow(clippy::cast_possible_truncation)]
        let b4 = (f_b4 * 2048.0).clamp(-2_000_000_000.0, 2_000_000_000.0) as i32;

        Ok(Self {
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
        })
    }

    fn load_quantized(bytes: &[u8]) -> Result<Self, String> {
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

        let mut w_psqt_vec = vec![[0i16; PSQT_BUCKETS]; L1_INPUT_SIZE];
        for row in &mut w_psqt_vec {
            for val in row {
                let slice = bytes
                    .get(offset..offset + 2)
                    .ok_or_else(|| "Unexpected EOF while reading W_PSQT".to_string())?;
                *val =
                    i16::from_le_bytes([*slice.first().unwrap_or(&0), *slice.get(1).unwrap_or(&0)]);
                offset += 2;
            }
        }
        let w_psqt: Box<[[i16; PSQT_BUCKETS]; L1_INPUT_SIZE]> = w_psqt_vec
            .into_boxed_slice()
            .try_into()
            .map_err(|_| "Failed to allocate boxed W_PSQT array".to_string())?;

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
        let w2_col = Self::transpose_w2(&w2);

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
        let w3_col = Self::transpose_w3(&w3);

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
            w_psqt,
            w2,
            w2_col,
            b2,
            w3,
            w3_col,
            b3,
            w4,
            b4,
        })
    }
}

/// Maps square (0..90) to a palace index (0..9).
#[inline]
#[must_use]
pub const fn palace_index(sq: usize) -> usize {
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
pub const fn halfkp_piece_type_idx(is_enemy: bool, piece_type: PieceType) -> Option<usize> {
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
pub const fn get_red_king_palace_idx(board: &Board) -> usize {
    if board.bitboards[0] != 0 {
        let sq = board.bitboards[0].trailing_zeros() as usize;
        palace_index(sq)
    } else {
        1
    }
}

#[inline]
#[must_use]
pub const fn get_black_king_palace_idx(board: &Board) -> usize {
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
        self.psqt_red = [0; PSQT_BUCKETS];
        for (sq, maybe_piece) in grid.iter().enumerate() {
            if let Some(piece) = maybe_piece {
                self.add_piece_red(*piece, sq, k_red_palace_idx, weights);
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
        self.psqt_black = [0; PSQT_BUCKETS];
        for (sq, maybe_piece) in grid.iter().enumerate() {
            if let Some(piece) = maybe_piece {
                self.add_piece_black(*piece, sq, k_black_palace_idx, weights);
            }
        }
    }

    #[inline]
    pub fn add_piece_red(&mut self, piece: Piece, sq: usize, k_red: usize, weights: &NnueWeights) {
        if let Some(red_feat) =
            halfkp_feature_idx_red(k_red, piece.color == Color::Black, piece.piece_type, sq)
        {
            if let Some(w_row) = weights.w1.get(red_feat) {
                for (a, &w) in self.red.iter_mut().zip(w_row.iter()) {
                    *a = a.saturating_add(w);
                }
            }
            if let Some(psqt_row) = weights.w_psqt.get(red_feat) {
                for (p, &w) in self.psqt_red.iter_mut().zip(psqt_row.iter()) {
                    *p = p.saturating_add(i32::from(w));
                }
            }
        }
    }

    #[inline]
    pub fn remove_piece_red(
        &mut self,
        piece: Piece,
        sq: usize,
        k_red: usize,
        weights: &NnueWeights,
    ) {
        if let Some(red_feat) =
            halfkp_feature_idx_red(k_red, piece.color == Color::Black, piece.piece_type, sq)
        {
            if let Some(w_row) = weights.w1.get(red_feat) {
                for (a, &w) in self.red.iter_mut().zip(w_row.iter()) {
                    *a = a.saturating_sub(w);
                }
            }
            if let Some(psqt_row) = weights.w_psqt.get(red_feat) {
                for (p, &w) in self.psqt_red.iter_mut().zip(psqt_row.iter()) {
                    *p = p.saturating_sub(i32::from(w));
                }
            }
        }
    }

    #[inline]
    pub fn add_piece_black(
        &mut self,
        piece: Piece,
        sq: usize,
        k_black: usize,
        weights: &NnueWeights,
    ) {
        if let Some(black_feat) =
            halfkp_feature_idx_black(k_black, piece.color == Color::Red, piece.piece_type, sq)
        {
            if let Some(w_row) = weights.w1.get(black_feat) {
                for (a, &w) in self.black.iter_mut().zip(w_row.iter()) {
                    *a = a.saturating_add(w);
                }
            }
            if let Some(psqt_row) = weights.w_psqt.get(black_feat) {
                for (p, &w) in self.psqt_black.iter_mut().zip(psqt_row.iter()) {
                    *p = p.saturating_add(i32::from(w));
                }
            }
        }
    }

    #[inline]
    pub fn remove_piece_black(
        &mut self,
        piece: Piece,
        sq: usize,
        k_black: usize,
        weights: &NnueWeights,
    ) {
        if let Some(black_feat) =
            halfkp_feature_idx_black(k_black, piece.color == Color::Red, piece.piece_type, sq)
        {
            if let Some(w_row) = weights.w1.get(black_feat) {
                for (a, &w) in self.black.iter_mut().zip(w_row.iter()) {
                    *a = a.saturating_sub(w);
                }
            }
            if let Some(psqt_row) = weights.w_psqt.get(black_feat) {
                for (p, &w) in self.psqt_black.iter_mut().zip(psqt_row.iter()) {
                    *p = p.saturating_sub(i32::from(w));
                }
            }
        }
    }

    #[inline]
    pub fn move_piece_quiet_red(
        &mut self,
        piece: Piece,
        from_sq: usize,
        to_sq: usize,
        k_red: usize,
        weights: &NnueWeights,
    ) {
        let is_black = piece.color == Color::Black;
        let (Some(sub_feat), Some(add_feat)) = (
            halfkp_feature_idx_red(k_red, is_black, piece.piece_type, from_sq),
            halfkp_feature_idx_red(k_red, is_black, piece.piece_type, to_sq),
        ) else {
            return;
        };

        if let (Some(add_row), Some(sub_row)) = (weights.w1.get(add_feat), weights.w1.get(sub_feat))
        {
            for ((a, &w_add), &w_sub) in self.red.iter_mut().zip(add_row.iter()).zip(sub_row.iter())
            {
                *a = a.saturating_add(w_add).saturating_sub(w_sub);
            }
        }
        if let (Some(add_p), Some(sub_p)) =
            (weights.w_psqt.get(add_feat), weights.w_psqt.get(sub_feat))
        {
            for ((p, &w_add), &w_sub) in
                self.psqt_red.iter_mut().zip(add_p.iter()).zip(sub_p.iter())
            {
                *p = p
                    .saturating_add(i32::from(w_add))
                    .saturating_sub(i32::from(w_sub));
            }
        }
    }

    #[inline]
    pub fn move_piece_quiet_black(
        &mut self,
        piece: Piece,
        from_sq: usize,
        to_sq: usize,
        k_black: usize,
        weights: &NnueWeights,
    ) {
        let is_red = piece.color == Color::Red;
        let (Some(sub_feat), Some(add_feat)) = (
            halfkp_feature_idx_black(k_black, is_red, piece.piece_type, from_sq),
            halfkp_feature_idx_black(k_black, is_red, piece.piece_type, to_sq),
        ) else {
            return;
        };

        if let (Some(add_row), Some(sub_row)) = (weights.w1.get(add_feat), weights.w1.get(sub_feat))
        {
            for ((a, &w_add), &w_sub) in self
                .black
                .iter_mut()
                .zip(add_row.iter())
                .zip(sub_row.iter())
            {
                *a = a.saturating_add(w_add).saturating_sub(w_sub);
            }
        }
        if let (Some(add_p), Some(sub_p)) =
            (weights.w_psqt.get(add_feat), weights.w_psqt.get(sub_feat))
        {
            for ((p, &w_add), &w_sub) in self
                .psqt_black
                .iter_mut()
                .zip(add_p.iter())
                .zip(sub_p.iter())
            {
                *p = p
                    .saturating_add(i32::from(w_add))
                    .saturating_sub(i32::from(w_sub));
            }
        }
    }

    #[inline]
    pub fn move_piece_capture_red(
        &mut self,
        piece: Piece,
        from_sq: usize,
        to_sq: usize,
        captured: Piece,
        k_red: usize,
        weights: &NnueWeights,
    ) {
        let is_black = piece.color == Color::Black;
        let (Some(sub_feat), Some(add_feat)) = (
            halfkp_feature_idx_red(k_red, is_black, piece.piece_type, from_sq),
            halfkp_feature_idx_red(k_red, is_black, piece.piece_type, to_sq),
        ) else {
            self.remove_piece_red(captured, to_sq, k_red, weights);
            return;
        };

        let Some(cap_feat) = halfkp_feature_idx_red(
            k_red,
            captured.color == Color::Black,
            captured.piece_type,
            to_sq,
        ) else {
            self.move_piece_quiet_red(piece, from_sq, to_sq, k_red, weights);
            return;
        };

        if let (Some(add_row), Some(sub_row), Some(cap_row)) = (
            weights.w1.get(add_feat),
            weights.w1.get(sub_feat),
            weights.w1.get(cap_feat),
        ) {
            for (((a, &w_add), &w_sub), &w_cap) in self
                .red
                .iter_mut()
                .zip(add_row.iter())
                .zip(sub_row.iter())
                .zip(cap_row.iter())
            {
                *a = a
                    .saturating_sub(w_cap)
                    .saturating_sub(w_sub)
                    .saturating_add(w_add);
            }
        }
        if let (Some(add_p), Some(sub_p), Some(cap_p)) = (
            weights.w_psqt.get(add_feat),
            weights.w_psqt.get(sub_feat),
            weights.w_psqt.get(cap_feat),
        ) {
            for (((p, &w_add), &w_sub), &w_cap) in self
                .psqt_red
                .iter_mut()
                .zip(add_p.iter())
                .zip(sub_p.iter())
                .zip(cap_p.iter())
            {
                *p = p
                    .saturating_sub(i32::from(w_cap))
                    .saturating_sub(i32::from(w_sub))
                    .saturating_add(i32::from(w_add));
            }
        }
    }

    #[inline]
    pub fn move_piece_capture_black(
        &mut self,
        piece: Piece,
        from_sq: usize,
        to_sq: usize,
        captured: Piece,
        k_black: usize,
        weights: &NnueWeights,
    ) {
        let is_red = piece.color == Color::Red;
        let (Some(sub_feat), Some(add_feat)) = (
            halfkp_feature_idx_black(k_black, is_red, piece.piece_type, from_sq),
            halfkp_feature_idx_black(k_black, is_red, piece.piece_type, to_sq),
        ) else {
            self.remove_piece_black(captured, to_sq, k_black, weights);
            return;
        };

        let Some(cap_feat) = halfkp_feature_idx_black(
            k_black,
            captured.color == Color::Red,
            captured.piece_type,
            to_sq,
        ) else {
            self.move_piece_quiet_black(piece, from_sq, to_sq, k_black, weights);
            return;
        };

        if let (Some(add_row), Some(sub_row), Some(cap_row)) = (
            weights.w1.get(add_feat),
            weights.w1.get(sub_feat),
            weights.w1.get(cap_feat),
        ) {
            for (((a, &w_add), &w_sub), &w_cap) in self
                .black
                .iter_mut()
                .zip(add_row.iter())
                .zip(sub_row.iter())
                .zip(cap_row.iter())
            {
                *a = a
                    .saturating_sub(w_cap)
                    .saturating_sub(w_sub)
                    .saturating_add(w_add);
            }
        }
        if let (Some(add_p), Some(sub_p), Some(cap_p)) = (
            weights.w_psqt.get(add_feat),
            weights.w_psqt.get(sub_feat),
            weights.w_psqt.get(cap_feat),
        ) {
            for (((p, &w_add), &w_sub), &w_cap) in self
                .psqt_black
                .iter_mut()
                .zip(add_p.iter())
                .zip(sub_p.iter())
                .zip(cap_p.iter())
            {
                *p = p
                    .saturating_sub(i32::from(w_cap))
                    .saturating_sub(i32::from(w_sub))
                    .saturating_add(i32::from(w_add));
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
        self.add_piece_red(piece, sq, k_red, weights);
        self.add_piece_black(piece, sq, k_black, weights);
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
        self.remove_piece_red(piece, sq, k_red, weights);
        self.remove_piece_black(piece, sq, k_black, weights);
    }
}

/// Evaluates both Red and Black forward passes simultaneously with shared SCReLU activations,
/// zero-skipping, and register-friendly separated L2 accumulation.
#[inline]
#[must_use]
pub fn evaluate_dual_forward(
    red_acc: &[i16; L1_OUTPUT_SIZE],
    black_acc: &[i16; L1_OUTPUT_SIZE],
    weights: &NnueWeights,
) -> (i32, i32) {
    let mut red_acts = [(0u16, 0i16); L1_OUTPUT_SIZE];
    let mut red_count = 0;

    for (i, &val) in red_acc.iter().enumerate() {
        if val >= 16 {
            let clamped = i32::from(val.min(256));
            #[allow(clippy::cast_possible_truncation)]
            let act = ((clamped * clamped) >> 8) as i16;
            if let Some(slot) = red_acts.get_mut(red_count) {
                #[allow(clippy::cast_possible_truncation)]
                {
                    *slot = (i as u16, act);
                }
                red_count += 1;
            }
        }
    }

    let mut black_acts = [(0u16, 0i16); L1_OUTPUT_SIZE];
    let mut black_count = 0;
    for (i, &val) in black_acc.iter().enumerate() {
        if val >= 16 {
            let clamped = i32::from(val.min(256));
            #[allow(clippy::cast_possible_truncation)]
            let act = ((clamped * clamped) >> 8) as i16;
            if let Some(slot) = black_acts.get_mut(black_count) {
                #[allow(clippy::cast_possible_truncation)]
                {
                    *slot = (i as u16, act);
                }
                black_count += 1;
            }
        }
    }

    let red_slice = red_acts.get(..red_count).unwrap_or(&[]);
    let black_slice = black_acts.get(..black_count).unwrap_or(&[]);

    // 1a. Layer 1 -> Layer 2 for Red perspective (sequential column access, fits in registers)
    let mut l2_red = weights.b2;
    for &(i, act) in red_slice {
        let act32 = i32::from(act);
        if let Some(col) = weights.w2_col.get(usize::from(i)) {
            for (out, &w) in l2_red.iter_mut().zip(col.iter()) {
                *out += act32 * i32::from(w);
            }
        }
    }
    for &(i, act) in black_slice {
        let act32 = i32::from(act);
        if let Some(col) = weights.w2_col.get(L1_OUTPUT_SIZE + usize::from(i)) {
            for (out, &w) in l2_red.iter_mut().zip(col.iter()) {
                *out += act32 * i32::from(w);
            }
        }
    }

    // 1b. Layer 1 -> Layer 2 for Black perspective (sequential column access, fits in registers)
    let mut l2_black = weights.b2;
    for &(i, act) in black_slice {
        let act32 = i32::from(act);
        if let Some(col) = weights.w2_col.get(usize::from(i)) {
            for (out, &w) in l2_black.iter_mut().zip(col.iter()) {
                *out += act32 * i32::from(w);
            }
        }
    }
    for &(i, act) in red_slice {
        let act32 = i32::from(act);
        if let Some(col) = weights.w2_col.get(L1_OUTPUT_SIZE + usize::from(i)) {
            for (out, &w) in l2_black.iter_mut().zip(col.iter()) {
                *out += act32 * i32::from(w);
            }
        }
    }

    // 2. Layer 2 -> Layer 3 (32 -> 32)
    let mut l3_red = weights.b3;
    for (j, &sum) in l2_red.iter().enumerate() {
        let act = (sum >> 7).clamp(0, 128);
        if act != 0 {
            if let Some(col) = weights.w3_col.get(j) {
                for (out, &w) in l3_red.iter_mut().zip(col.iter()) {
                    *out += act * i32::from(w);
                }
            }
        }
    }

    let mut l3_black = weights.b3;
    for (j, &sum) in l2_black.iter().enumerate() {
        let act = (sum >> 7).clamp(0, 128);
        if act != 0 {
            if let Some(col) = weights.w3_col.get(j) {
                for (out, &w) in l3_black.iter_mut().zip(col.iter()) {
                    *out += act * i32::from(w);
                }
            }
        }
    }

    // 3. Layer 3 -> Layer 4 (32 -> 1)
    let mut final_red = weights.b4;
    for (&sum, &w) in l3_red.iter().zip(weights.w4.iter()) {
        let act = (sum >> 6).clamp(0, 128);
        final_red += act * i32::from(w);
    }

    let mut final_black = weights.b4;
    for (&sum, &w) in l3_black.iter().zip(weights.w4.iter()) {
        let act = (sum >> 6).clamp(0, 128);
        final_black += act * i32::from(w);
    }

    (final_red / 2048, final_black / 2048)
}

/// Evaluates the deep FC layers (L1 -> L2 -> L3 -> Out) using SCReLU activation
/// and sparse non-zero input skipping over pre-transposed column-major matrices.
#[inline]
#[must_use]
pub fn evaluate_forward(
    active_acc: &[i16; L1_OUTPUT_SIZE],
    opp_acc: &[i16; L1_OUTPUT_SIZE],
    weights: &NnueWeights,
) -> i32 {
    let (score, _) = evaluate_dual_forward(active_acc, opp_acc, weights);
    score
}

/// Evaluates the board position from Red's perspective using the Pikafish-style
/// Dual-Head HalfKP NNUE (8-Bucket Phase PSQT Head + Deep SCReLU Positional Head).
#[inline]
#[must_use]
pub fn evaluate_nnue_with_weights(board: &Board, weights: &NnueWeights) -> i32 {
    let piece_count = board.occupied.count_ones() as usize;
    let bucket = ((piece_count.saturating_sub(2)) * PSQT_BUCKETS / 30).min(PSQT_BUCKETS - 1);

    let psqt_red = board.nnue_acc.psqt_red.get(bucket).copied().unwrap_or(0);
    let psqt_black = board.nnue_acc.psqt_black.get(bucket).copied().unwrap_or(0);
    let psqt_score = (psqt_red - psqt_black) / 16;

    let (red_score, black_score) =
        evaluate_dual_forward(&board.nnue_acc.red, &board.nnue_acc.black, weights);
    let positional_score = (red_score - black_score) / 2;

    psqt_score + positional_score
}

#[inline]
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
        assert_eq!(weights.w_psqt.len(), L1_INPUT_SIZE);
        assert_eq!(weights.b2.len(), L2_OUTPUT_SIZE);
        assert_eq!(weights.b3.len(), L3_OUTPUT_SIZE);
        assert_eq!(weights.w4.len(), L3_OUTPUT_SIZE);
    }

    #[test]
    fn test_load_standard_nnue_model() {
        if let Ok(bytes) = std::fs::read("assets/default.nnue") {
            let loaded = NnueWeights::load_from_bytes(&bytes);
            assert!(
                loaded.is_ok(),
                "Failed to load default.nnue: {:?}",
                loaded.err()
            );
        }
        if let Ok(bytes) = std::fs::read("assets/model_f32.bin") {
            let loaded_f32 = NnueWeights::load_from_bytes(&bytes);
            assert!(
                loaded_f32.is_ok(),
                "Failed to load model_f32.bin: {:?}",
                loaded_f32.err()
            );
        }
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
        for k in 0..9 {
            let feat = halfkp_feature_idx_red(k, false, PieceType::Chariot, 0).expect("Valid feat");
            assert!(feat < L1_INPUT_SIZE);

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
        assert_eq!(board.nnue_acc.psqt_red, expected.nnue_acc.psqt_red);
        assert_eq!(board.nnue_acc.psqt_black, expected.nnue_acc.psqt_black);

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
            assert_eq!(board.nnue_acc.psqt_red, fresh.nnue_acc.psqt_red);
            assert_eq!(board.nnue_acc.psqt_black, fresh.nnue_acc.psqt_black);

            turn = turn.opposite();
        }

        // 3. Undo all moves and verify back to start
        while let Some((mv, cap, t)) = history.pop() {
            board.undo_move(&mv, cap, t);
            let mut fresh = board.clone();
            fresh.refresh_nnue();
            assert_eq!(board.nnue_acc.red, fresh.nnue_acc.red);
            assert_eq!(board.nnue_acc.black, fresh.nnue_acc.black);
            assert_eq!(board.nnue_acc.psqt_red, fresh.nnue_acc.psqt_red);
            assert_eq!(board.nnue_acc.psqt_black, fresh.nnue_acc.psqt_black);
        }
    }
}
