use crate::engine::eval_constants::*;
use crate::engine::Evaluator;
use crate::logic::board::{Board, Color, Piece, PieceType};

pub struct SimpleEvaluator;

impl Evaluator for SimpleEvaluator {
    fn evaluate(&self, board: &Board) -> i32 {
        let mut score = 0;

        for r in 0..10 {
            for c in 0..9 {
                if let Some(piece) = board.get_piece(r, c) {
                    let value = get_piece_value(piece, r, c);
                    if piece.color == Color::Red {
                        score += value;
                    } else {
                        score -= value;
                    }
                }
            }
        }

        score
    }
}

fn get_piece_value(piece: Piece, row: usize, col: usize) -> i32 {
    let base_val = match piece.piece_type {
        PieceType::General => VAL_KING,
        PieceType::Advisor => VAL_ADVISOR,
        PieceType::Elephant => VAL_ELEPHANT,
        PieceType::Horse => VAL_HORSE,
        PieceType::Chariot => VAL_ROOK,
        PieceType::Cannon => VAL_CANNON,
        PieceType::Soldier => VAL_PAWN,
    };

    let pst_val = get_pst_value(piece, row, col);
    base_val + pst_val
}

fn get_pst_value(piece: Piece, row: usize, col: usize) -> i32 {
    // For Red, use row/col directly.
    // For Black, mirror row/col.
    let (r, _c) = if piece.color == Color::Red {
        (row, col)
    } else {
        (9 - row, col) // Mirror row only for now (symmetric PSTs)
    };

    match piece.piece_type {
        PieceType::Soldier => PST_PAWN[r][_c],
        PieceType::Horse => PST_HORSE[r][_c],
        PieceType::Chariot => PST_ROOK[r][_c],
        PieceType::Cannon => PST_CANNON[r][_c],
        _ => 0,
    }
}
