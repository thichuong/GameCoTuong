// Piece Values
pub const VAL_PAWN: i32 = 100;
pub const VAL_ADVISOR: i32 = 200;
pub const VAL_ELEPHANT: i32 = 200;
pub const VAL_HORSE: i32 = 400;
pub const VAL_CANNON: i32 = 450;
pub const VAL_ROOK: i32 = 900;
pub const VAL_KING: i32 = 10000;

// Piece-Square Tables (Simplified for Red, flip for Black)
// 10 rows, 9 cols.
// High values = good for Red.
// (0,0) is Red's bottom-left.

#[rustfmt::skip]
pub const PST_PAWN: [[i32; 9]; 10] = [
    [  0,   0,   0,   0,   0,   0,   0,   0,   0], // Row 0 (Bottom)
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0], // Before river
    [ 10,  10,  10,  10,  10,  10,  10,  10,  10], // River bank (Red side)
    [ 20,  20,  20,  20,  20,  20,  20,  20,  20], // River bank (Black side)
    [ 30,  30,  30,  30,  30,  30,  30,  30,  30],
    [ 40,  40,  40,  40,  40,  40,  40,  40,  40],
    [ 50,  50,  50,  50,  50,  50,  50,  50,  50],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0], // Top (Black edge)
];

// ... (We can add more detailed tables later, using flat values for now to save space/time)
// For a simple engine, just position bonuses are enough.

#[rustfmt::skip]
pub const PST_HORSE: [[i32; 9]; 10] = [
    [  0, -10,   0,   0,   0,   0,   0, -10,   0],
    [  0,   5,  15,   5,   5,   5,  15,   5,   0],
    [  5,   5,  10,  10,  10,  10,  10,   5,   5],
    [  5,  10,  15,  20,  20,  20,  15,  10,   5],
    [  5,  10,  15,  20,  20,  20,  15,  10,   5],
    [  5,  10,  20,  25,  25,  25,  20,  10,   5],
    [  5,  10,  20,  25,  25,  25,  20,  10,   5],
    [  5,  10,  10,  10,  10,  10,  10,  10,   5],
    [  0,   5,   5,   5,   5,   5,   5,   5,   0],
    [  0, -10,   0,   0,   0,   0,   0, -10,   0],
];

#[rustfmt::skip]
pub const PST_ROOK: [[i32; 9]; 10] = [
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,  10,   0,  10,   0,  10,   0,  10,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [ 10,  20,  20,  20,  20,  20,  20,  20,  10], // Control back rank
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
];

#[rustfmt::skip]
pub const PST_CANNON: [[i32; 9]; 10] = [
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,  10,   0,   0,   0,   0,   0,  10,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0], // Cannon row
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [ 10,  10,  10,  10,  10,  10,  10,  10,  10],
    [ 10,  10,  10,  10,  10,  10,  10,  10,  10],
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
        _ => Some(&0),
    };
    *val.unwrap_or(&0)
}
