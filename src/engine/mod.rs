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

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum SearchLimit {
    Depth(u8),
    Time(u64), // milliseconds
}

pub trait Evaluator {
    fn evaluate(&self, board: &Board) -> i32;
}

pub trait Searcher {
    fn search(&mut self, game_state: &GameState, limit: SearchLimit) -> Option<Move>;
}
