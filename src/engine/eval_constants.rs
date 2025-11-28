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
