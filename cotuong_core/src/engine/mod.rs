use crate::logic::board::Board;
use crate::logic::game::GameState;
use serde::{Deserialize, Serialize};

pub mod config;
pub mod eval;
pub mod move_list;
pub mod search;
pub mod tt;
pub mod zobrist;

#[cfg(test)]
mod bench_test;
mod mate_test;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Move {
    pub from_row: u8,
    pub from_col: u8,
    pub to_row: u8,
    pub to_col: u8,
    pub score: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum SearchLimit {
    Depth(u8),
    Time(u64), // milliseconds
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct SearchStats {
    pub depth: u8,
    pub nodes: u32,
    pub time_ms: u64,
}

pub trait Evaluator {
    fn evaluate(&self, board: &Board) -> i32;
}

pub trait Searcher {
    fn search(
        &mut self,
        game_state: &GameState,
        limit: SearchLimit,
        excluded_moves: &[Move],
    ) -> Option<(Move, SearchStats)>;
}
