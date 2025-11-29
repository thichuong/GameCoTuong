use crate::engine::config::EngineConfig;
use crate::engine::Evaluator;
use crate::logic::board::{Board, Color, Piece, PieceType};
use std::sync::Arc;

pub struct SimpleEvaluator {
    config: Arc<EngineConfig>,
}

impl SimpleEvaluator {
    pub fn new(config: Arc<EngineConfig>) -> Self {
        Self { config }
    }
}

impl Evaluator for SimpleEvaluator {
    fn evaluate(&self, board: &Board) -> i32 {
        let mut score = 0;

        for r in 0..10 {
            for c in 0..9 {
                if let Some(piece) = board.get_piece(r, c) {
                    let value = self.get_piece_value(piece, r, c);
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

impl SimpleEvaluator {
    fn get_piece_value(&self, piece: Piece, row: usize, col: usize) -> i32 {
        let base_val = match piece.piece_type {
            PieceType::General => self.config.val_king,
            PieceType::Advisor => self.config.val_advisor,
            PieceType::Elephant => self.config.val_elephant,
            PieceType::Horse => self.config.val_horse,
            PieceType::Chariot => self.config.val_rook,
            PieceType::Cannon => self.config.val_cannon,
            PieceType::Soldier => self.config.val_pawn,
        };

        let pst_val = self.get_pst_value(piece, row, col);
        base_val + pst_val
    }

    fn get_pst_value(&self, piece: Piece, row: usize, col: usize) -> i32 {
        // For Red, use row/col directly.
        // For Black, mirror row/col.
        let (r, c) = if piece.color == Color::Red {
            (row, col)
        } else {
            (9 - row, col)
        };

        let val = match piece.piece_type {
            PieceType::Soldier => self.config.pst_pawn.get(r).and_then(|row| row.get(c)),
            PieceType::Horse => self.config.pst_horse.get(r).and_then(|row| row.get(c)),
            PieceType::Chariot => self.config.pst_rook.get(r).and_then(|row| row.get(c)),
            PieceType::Cannon => self.config.pst_cannon.get(r).and_then(|row| row.get(c)),
            _ => Some(&0),
        };
        *val.unwrap_or(&0)
    }
}
