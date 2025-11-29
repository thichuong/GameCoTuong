use crate::engine::eval_constants::{
    PST_CANNON, PST_HORSE, PST_PAWN, PST_ROOK, VAL_ADVISOR, VAL_CANNON, VAL_ELEPHANT, VAL_HORSE,
    VAL_KING, VAL_PAWN, VAL_ROOK,
};
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
    let (r, c) = if piece.color == Color::Red {
        (row, col)
    } else {
        (9 - row, col) // Mirror row for Black, col is same for PST (symmetric?)
                       // Actually PSTs are usually symmetric or defined for one side.
                       // If PST is for Red, then Black at (9, c) is like Red at (0, c).
    };

    let val = match piece.piece_type {
        PieceType::Soldier => PST_PAWN.get(r).and_then(|row| row.get(c)),
        PieceType::Horse => PST_HORSE.get(r).and_then(|row| row.get(c)),
        PieceType::Chariot => PST_ROOK.get(r).and_then(|row| row.get(c)),
        PieceType::Cannon => PST_CANNON.get(r).and_then(|row| row.get(c)),
        _ => Some(&0),
    };
    *val.unwrap_or(&0)
}
