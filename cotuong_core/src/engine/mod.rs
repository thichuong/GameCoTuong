use crate::logic::board::Board;
use crate::logic::game::GameState;
use serde::{Deserialize, Serialize};

pub mod config;
pub mod eval;
pub mod move_list;
pub mod movegen;
pub mod nnue;
pub mod search;
pub mod tt;
pub mod zobrist;

pub use search::perft;

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

impl Move {
    #[inline]
    #[must_use]
    pub const fn matches(&self, fr: usize, fc: usize, tr: usize, tc: usize) -> bool {
        self.from_row as usize == fr
            && self.from_col as usize == fc
            && self.to_row as usize == tr
            && self.to_col as usize == tc
    }

    /// Converts a move to a 4-character UCI coordinate string (e.g., "b2e2", "h0g2").
    #[must_use]
    pub fn to_uci(&self) -> String {
        let from_file = (b'a' + self.from_col) as char;
        let from_rank = (b'0' + self.from_row) as char;
        let to_file = (b'a' + self.to_col) as char;
        let to_rank = (b'0' + self.to_row) as char;
        format!("{from_file}{from_rank}{to_file}{to_rank}")
    }

    /// Parses a 4-character UCI coordinate string (e.g., "b2e2") into a Move.
    pub fn from_uci(uci: &str) -> Result<Self, &'static str> {
        let bytes = uci.as_bytes();
        if bytes.len() != 4 {
            return Err("UCI move string must be exactly 4 characters");
        }
        let from_col = bytes[0].wrapping_sub(b'a');
        let from_row = bytes[1].wrapping_sub(b'0');
        let to_col = bytes[2].wrapping_sub(b'a');
        let to_row = bytes[3].wrapping_sub(b'0');

        if from_col > 8 || from_row > 9 || to_col > 8 || to_row > 9 {
            return Err("UCI move coordinates out of Xiangqi board bounds");
        }

        Ok(Self {
            from_row,
            from_col,
            to_row,
            to_col,
            score: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uci_move_conversion() {
        let mv = Move {
            from_row: 2,
            from_col: 1,
            to_row: 2,
            to_col: 4,
            score: 0,
        };
        assert_eq!(mv.to_uci(), "b2e2");

        let parsed = Move::from_uci("b2e2").expect("Valid UCI move");
        assert_eq!(parsed.from_row, 2);
        assert_eq!(parsed.from_col, 1);
        assert_eq!(parsed.to_row, 2);
        assert_eq!(parsed.to_col, 4);

        assert!(Move::from_uci("b2").is_err());
        assert!(Move::from_uci("z2e2").is_err());
        assert!(Move::from_uci("a9a:").is_err());
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum SearchLimit {
    Depth(u8),
    Time(u64), // milliseconds
    DepthOrTime { depth: u8, time_ms: u64 },
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct SearchStats {
    pub depth: u8,
    pub nodes: u32,
    pub time_ms: u64,
    #[serde(default)]
    pub score: i32,
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
