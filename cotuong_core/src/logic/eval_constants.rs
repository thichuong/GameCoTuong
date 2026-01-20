// Piece Values (Scaled up for granularity, approx Pawn=100 base)
pub const VAL_PAWN: i32 = 100;
pub const VAL_ADVISOR: i32 = 200;
pub const VAL_ELEPHANT: i32 = 200;
pub const VAL_HORSE: i32 = 450;
pub const VAL_CANNON: i32 = 500; // Cannon slightly better than Horse in general, dynamic adjustment in eval
pub const VAL_ROOK: i32 = 1000;
pub const VAL_KING: i32 = 10000;

// Piece-Square Tables (For Red side, bottom to top)
// 10 rows (0..9), 9 cols (0..8)
// 0,0 is Bottom Left.

#[rustfmt::skip]
pub const PST_PAWN: [[i32; 9]; 10] = [
    [  0,   0,   0,   0,   0,   0,   0,   0,   0], // Row 0 (Base)
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0], // Not crossed river
    [  0,   0,   0,  10,  20,  10,   0,   0,   0], // 3.5th rank? (Before river) - slightly better in center
    [ 20,  20,  20,  30,  40,  30,  20,  20,  20], // River bank (Red side) - Ready to cross
    [ 50,  60,  70,  80,  80,  80,  70,  60,  50], // Crossed River (Black side) - Stronger
    [ 70,  80,  90, 100, 100, 100,  90,  80,  70], // Approaching Palace
    [ 80,  90, 100, 110, 110, 110, 100,  90,  80], // Deep in
    [ 90, 100, 110, 120, 120, 120, 110, 100,  90], // Throat
    [  0,   0,   0,  20,  20,  20,   0,   0,   0], // Top edge (often useless unless center)
];

#[rustfmt::skip]
pub const PST_ADVISOR: [[i32; 9]; 10] = [
    [  0,   0,   0,  20,   0,  20,   0,   0,   0], // Base corners good
    [  0,   0,   0,   0,  30,   0,   0,   0,   0], // Center best
    [  0,   0,   0,  20,   0,  20,   0,   0,   0], // Top corners good
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
];

#[rustfmt::skip]
pub const PST_ELEPHANT: [[i32; 9]; 10] = [
    [  0,   0,  10,   0,   0,   0,  10,   0,   0], // Base
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,  30,   0,   0,   0,   0], // Center (Defensive hub)
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,  10,   0,   0,   0,  10,   0,   0], // River bank
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0,   0],
];

#[rustfmt::skip]
pub const PST_HORSE: [[i32; 9]; 10] = [
    [ -10, -10, -10,   0,  -5,   0, -10, -10, -10], // Base (Weak)
    [ -10,   0,   0,   5,   5,   5,   0,   0, -10],
    [   0,   0,  10,  10,  10,  10,  10,   0,   0],
    [   0,  10,  20,  30,  30,  30,  20,  10,   0], // River bank (Good)
    [   0,  10,  20,  30,  30,  30,  20,  10,   0], // River bank
    [   5,  15,  25,  35,  35,  35,  25,  15,   5], // Crossing
    [   5,  20,  30,  40,  40,  40,  30,  20,   5], // Attacking zone
    [  10,  25,  30,  40,  40,  40,  30,  25,  10], // Near palace (Palace corners at 2/6 are good)
    [   0,  10,  20,  20,  20,  20,  20,  10,   0],
    [ -10, -10,  -5,  -5,  -5,  -5,  -5, -10, -10], // Top edge (Weak)
];

#[rustfmt::skip]
pub const PST_CANNON: [[i32; 9]; 10] = [
    [   0,   0,  10,   0,   5,   0,  10,   0,   0], // Setup
    [   0,  10,   0,   0,   0,   0,   0,  10,   0],
    [   0,  20,   0,  10,   0,  10,   0,  20,   0], // Cannon row (Good)
    [   0,   0,   0,   0,   0,   0,   0,   0,   0],
    [   0,   0,   0,   0,   0,   0,   0,   0,   0], // River (Neutral)
    [   0,   0,   0,   0,   0,   0,   0,   0,   0],
    [   0,   0,   0,   0,   0,   0,   0,   0,   0],
    [  10,  20,  30,  20,  20,  20,  30,  20,  10], // Opponent's cannon row (Pressure)
    [  10,  20,  30,  20,  20,  20,  30,  20,  10], // Opponent's pawn row
    [  10,  20,  30,  20,  20,  20,  30,  20,  10], // Back rank (Bottom cannon)
];

#[rustfmt::skip]
pub const PST_ROOK: [[i32; 9]; 10] = [
    [  0,   5,   5,   5,   0,   5,   5,   5,   0], // Base
    [  0,  10,   0,   0,   0,   0,   0,  10,   0],
    [  0,  10,   0,   0,   0,   0,   0,  10,   0],
    [  0,  10,   0,   0,   0,   0,   0,  10,   0],
    [ 10,  20,  20,  20,  20,  20,  20,  20,  10], // River control
    [ 10,  30,  30,  30,  30,  30,  30,  30,  10], // Opponent River
    [ 10,  30,  30,  30,  30,  30,  30,  30,  10],
    [ 10,  30,  30,  30,  30,  30,  30,  30,  10],
    [ 20,  40,  40,  40,  40,  40,  40,  40,  20], // Deep penetration
    [ 30,  50,  50,  50,  50,  50,  50,  50,  30], // Back rank (Mate threat)
];

#[rustfmt::skip]
pub const PST_KING: [[i32; 9]; 10] = [
    [  0,   0,   0,   0,   0,   0,   0,   0,   0], // Middle is standard
    [  0,   0,   0, -10, -20, -10,   0,   0,   0], // Second rank often safer than exposed
    [  0,   0,   0, -10, -20, -10,   0,   0,   0], // Top of palace (Exposed)
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
