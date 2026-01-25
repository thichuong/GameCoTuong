// Piece Values
pub const VAL_PAWN: i32 = 30;
pub const VAL_ADVISOR: i32 = 120;
pub const VAL_ELEPHANT: i32 = 120;
pub const VAL_HORSE: i32 = 270;
pub const VAL_CANNON: i32 = 285;
pub const VAL_ROOK: i32 = 600;
pub const VAL_KING: i32 = 6000;

// Evaluation Weights
pub const WEIGHT_MOBILITY_ROOK: i32 = 4;
pub const WEIGHT_MOBILITY_HORSE: i32 = 6; // Horses blocked are bad
pub const WEIGHT_MOBILITY_CANNON: i32 = 3;
pub const WEIGHT_MOBILITY_PAWN: i32 = 2; // Only valid for river checkers maybe?

// King Safety
pub const WEIGHT_KING_DANGER_CANNON_MOUNT: i32 = 40; // Penalty for cannon aiming at king with mount
pub const WEIGHT_KING_EXPOSED: i32 = 50; // Penalty for king on open file/rank

// Structure
pub const BONUS_CONNECTED_ELEPHANTS: i32 = 30;
pub const BONUS_CONNECTED_ADVISORS: i32 = 30;

// Piece-Square Tables (Red, flip for Black)
// 10 rows, 9 cols.
// (0,0) is Red's bottom-left.

#[rustfmt::skip]
pub const PST_PAWN: [[i32; 9]; 10] = [
    [  0,   0,   0,   0,   0,   0,   0,   0,   0], // Row 0
    [  0,   0,   0, -10,   0, -10,   0,   0,   0],
    [ -2,   0,  -2,   0,   6,   0,  -2,   0, -2],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0], // Before river
    [ 10,  10,  10,  10,  10,  10,  10,  10,  10], // River bank (Red side)
    [ 30,  40,  40,  50,  60,  50,  40,  40,  30], // Across river
    [ 40,  50,  60,  70,  80,  70,  60,  50,  40],
    [ 50,  60,  70,  80,  80,  80,  70,  60,  50],
    [ 60,  70,  80,  90, 100,  90,  80,  70,  60],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0], // Back rank? Usually redundant but ok
];

#[rustfmt::skip]
pub const PST_ADVISOR: [[i32; 9]; 10] = [
    [  0,   0,   0,  10,   0,  10,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
];

// Elephants need to stay home usually, mostly defensive
#[rustfmt::skip]
pub const PST_ELEPHANT: [[i32; 9]; 10] = [
    [  0,   0,  10,   0,   0,   0,  10,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,  10,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0], // River
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
];

#[rustfmt::skip]
pub const PST_HORSE: [[i32; 9]; 10] = [
    [ -4,  -8,  -4, -10, -10, -10,  -4,  -8,  -4],
    [ -4,   8,   4,   0,   0,   0,   4,   8,  -4],
    [  4,   4,  12,  15,  15,  15,  12,   4,  -4],
    [  5,   8,  15,  20,  20,  20,  15,   8,   5],
    [ 10,  20,  20,  25,  25,  25,  20,  20,  10], // River bank
    [ 10,  25,  20,  30,  30,  30,  20,  25,  10], // Across river
    [ 10,  20,  25,  35,  35,  35,  25,  20,  10],
    [ 10,  20,  20,  10,  10,  10,  20,  20,  10],
    [  5,  10,  10,  10,  10,  10,  10,  10,   5], // Opponent palace
    [  5,   5,   5,   5,   5,   5,   5,   5,   5],
];

#[rustfmt::skip]
pub const PST_ROOK: [[i32; 9]; 10] = [
    [ 10,  10,  10,  10,  10,  10,  10,  10,  10], // Good to have on bottom?
    [ 20,  30,  20,  30,  20,  30,  20,  30,  20],
    [ 10,  20,  20,  20,  20,  20,  20,  20,  10],
    [ 10,  20,  20,  20,  20,  20,  20,  20,  10],
    [ 10,  30,  20,  20,  20,  20,  20,  30,  10],
    [ 10,  40,  30,  30,  30,  30,  30,  40,  10], // Control river
    [ 20,  40,  40,  40,  40,  40,  40,  40,  20],
    [ 20,  40,  40,  40,  45,  40,  40,  40,  20],
    [ 20,  40,  40,  40,  50,  40,  40,  40,  20], // Deep in opponent territory
    [ 10,  20,  20,  20,  20,  20,  20,  20,  10],
];

#[rustfmt::skip]
pub const PST_CANNON: [[i32; 9]; 10] = [
    [  0,   0,   2,   4,   4,   4,   2,   0,   0],
    [  0,  10,   0,   0,   0,   0,   0,  10,   0],
    [  0,   0,  10,   0,  20,   0,  10,   0,   0], // Standard cannon opening spots
    [  0,   0,  10,   0,   0,   0,  10,   0,   0],
    [  0,  10,  10,  10,  10,  10,  10,  10,   0], // River bank
    [  0,   5,   5,   5,   5,   5,   5,   5,   0],
    [  0,   5,   5,   5,   5,   5,   5,   5,   0],
    [ 10,  10,  10,  10,  10,  10,  10,  10,  10], // 2nd rank is strong for cannon
    [ 20,  20,  20,  20,  20,  20,  20,  20,  20], // Back rank pressure
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
];

#[rustfmt::skip]
pub const PST_KING: [[i32; 9]; 10] = [
    [  0,   0,   0,   0,   0,   0,   0,   0,   0], // King only lives in palace 0-2
    [  0,   0,   0,   0,  10,   0,   0,   0,   0],
    [  0,   0,   0,   0, -10,   0,   0,   0,   0], // Don't expose unless endgame
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
];

use crate::logic::board::{Color, PieceType};

pub const fn get_piece_value(pt: PieceType) -> i32 {
    match pt {
        PieceType::General => VAL_KING,
        PieceType::Advisor => VAL_ADVISOR,
        PieceType::Elephant => VAL_ELEPHANT,
        PieceType::Horse => VAL_HORSE,
        PieceType::Chariot => VAL_ROOK,
        PieceType::Cannon => VAL_CANNON,
        PieceType::Soldier => VAL_PAWN,
    }
}

pub fn get_pst_value(pt: PieceType, color: Color, row: usize, col: usize) -> i32 {
    let (r, c) = if color == Color::Red {
        (row, col)
    } else {
        (9 - row, col)
    };

    let val = match pt {
        PieceType::Soldier => PST_PAWN.get(r).and_then(|row| row.get(c)),
        PieceType::Horse => PST_HORSE.get(r).and_then(|row| row.get(c)),
        PieceType::Chariot => PST_ROOK.get(r).and_then(|row| row.get(c)),
        PieceType::Cannon => PST_CANNON.get(r).and_then(|row| row.get(c)),
        PieceType::Advisor => PST_ADVISOR.get(r).and_then(|row| row.get(c)),
        PieceType::Elephant => PST_ELEPHANT.get(r).and_then(|row| row.get(c)),
        PieceType::General => PST_KING.get(r).and_then(|row| row.get(c)),
    };
    *val.unwrap_or(&0)
}

#[rustfmt::skip]
pub const SQ_TO_COORD: [(usize, usize); 90] = [
    (0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5), (0, 6), (0, 7), (0, 8),
    (1, 0), (1, 1), (1, 2), (1, 3), (1, 4), (1, 5), (1, 6), (1, 7), (1, 8),
    (2, 0), (2, 1), (2, 2), (2, 3), (2, 4), (2, 5), (2, 6), (2, 7), (2, 8),
    (3, 0), (3, 1), (3, 2), (3, 3), (3, 4), (3, 5), (3, 6), (3, 7), (3, 8),
    (4, 0), (4, 1), (4, 2), (4, 3), (4, 4), (4, 5), (4, 6), (4, 7), (4, 8),
    (5, 0), (5, 1), (5, 2), (5, 3), (5, 4), (5, 5), (5, 6), (5, 7), (5, 8),
    (6, 0), (6, 1), (6, 2), (6, 3), (6, 4), (6, 5), (6, 6), (6, 7), (6, 8),
    (7, 0), (7, 1), (7, 2), (7, 3), (7, 4), (7, 5), (7, 6), (7, 7), (7, 8),
    (8, 0), (8, 1), (8, 2), (8, 3), (8, 4), (8, 5), (8, 6), (8, 7), (8, 8),
    (9, 0), (9, 1), (9, 2), (9, 3), (9, 4), (9, 5), (9, 6), (9, 7), (9, 8),
];
