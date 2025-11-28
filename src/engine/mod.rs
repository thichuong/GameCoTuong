use crate::logic::board::Board;
use crate::logic::game::GameState;

pub mod eval;
pub mod eval_constants;
pub mod search;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    pub from_row: usize,
    pub from_col: usize,
    pub to_row: usize,
    pub to_col: usize,
    pub score: i32,
}

pub trait Evaluator {
    fn evaluate(&self, board: &Board) -> i32;
}

pub trait Searcher {
    fn search(&mut self, game_state: &GameState, depth: u8) -> Option<Move>;
}
