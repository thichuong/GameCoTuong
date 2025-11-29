use crate::engine::Move;
use crate::logic::board::{Color, PieceType};

// Constants for Zobrist Hashing
// 7 Piece Types * 2 Colors * 10 Rows * 9 Cols
const NUM_PIECE_TYPES: usize = 7;
const NUM_COLORS: usize = 2;
const NUM_ROWS: usize = 10;
const NUM_COLS: usize = 9;
const TABLE_SIZE: usize = NUM_PIECE_TYPES * NUM_COLORS * NUM_ROWS * NUM_COLS;

pub struct ZobristKeys {
    pub piece_keys: [u64; TABLE_SIZE],
    pub side_key: u64,
}

// Simple XorShift RNG for deterministic keys without dependencies
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

// Global instance of ZobristKeys (lazy_static or just a function to create it)
// Since we want to avoid lazy_static dependency, we can just instantiate it in the Engine.
impl ZobristKeys {
    pub fn new() -> Self {
        let mut rng = XorShift64::new(123_456_789);
        let mut piece_keys = [0; TABLE_SIZE];
        for key in &mut piece_keys {
            *key = rng.next();
        }
        let side_key = rng.next();

        Self {
            piece_keys,
            side_key,
        }
    }

    pub fn get_piece_key(
        &self,
        piece_type: PieceType,
        color: Color,
        row: usize,
        col: usize,
    ) -> u64 {
        let pt_idx = match piece_type {
            PieceType::General => 0,
            PieceType::Advisor => 1,
            PieceType::Elephant => 2,
            PieceType::Horse => 3,
            PieceType::Chariot => 4,
            PieceType::Cannon => 5,
            PieceType::Soldier => 6,
        };
        let c_idx = match color {
            Color::Red => 0,
            Color::Black => 1,
        };

        // Index calculation:
        // ((pt_idx * 2 + c_idx) * 10 + row) * 9 + col
        // Index calculation is guaranteed to be within bounds
        let idx = ((pt_idx * NUM_COLORS + c_idx) * NUM_ROWS + row) * NUM_COLS + col;
        #[allow(clippy::indexing_slicing)]
        self.piece_keys[idx]
    }
}

// Transposition Table
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TTFlag {
    Exact,
    LowerBound,
    UpperBound,
}

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub key: u64,
    pub depth: u8,
    pub score: i32,
    pub flag: TTFlag,
    pub best_move: Option<Move>,
}

pub struct TranspositionTable {
    entries: Vec<Option<TTEntry>>,
    size: usize,
}

impl TranspositionTable {
    pub fn new(_size_mb: usize) -> Self {
        // Calculate number of entries based on size_mb
        // Size of TTEntry is roughly 8 + 1 + 4 + 1 + (size of Option<Move>) bytes
        // Move is 5 bytes (u8*4 + i32), Option adds tag/padding. Let's say ~24-32 bytes.
        // 1MB ~ 32,000 entries.
        // User suggested 65536 entries, which is a power of 2, good for indexing.
        let num_entries = 65536;

        Self {
            entries: vec![None; num_entries],
            size: num_entries,
        }
    }

    pub fn probe(&self, key: u64, depth: u8, alpha: i32, beta: i32) -> Option<i32> {
        #[allow(clippy::cast_possible_truncation)]
        let idx = (key % (self.size as u64)) as usize;
        if let Some(entry) = self.entries.get(idx).and_then(|e| e.as_ref()) {
            if entry.key == key && entry.depth >= depth {
                match entry.flag {
                    TTFlag::Exact => return Some(entry.score),
                    TTFlag::LowerBound => {
                        if entry.score >= beta {
                            return Some(entry.score);
                        }
                    }
                    TTFlag::UpperBound => {
                        if entry.score <= alpha {
                            return Some(entry.score);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_move(&self, key: u64) -> Option<Move> {
        #[allow(clippy::cast_possible_truncation)]
        let idx = (key % (self.size as u64)) as usize;
        #[allow(clippy::indexing_slicing)]
        if let Some(entry) = &self.entries[idx] {
            if entry.key == key {
                return entry.best_move;
            }
        }
        None
    }

    pub fn store(
        &mut self,
        key: u64,
        depth: u8,
        score: i32,
        flag: TTFlag,
        best_move: Option<Move>,
    ) {
        #[allow(clippy::cast_possible_truncation)]
        let idx = (key % (self.size as u64)) as usize;

        // Replacement scheme: Always replace if new depth is greater or equal,
        // or if the current entry is from a different position (collision).
        // For simplicity, we'll just always replace for now, or maybe prefer deeper searches.
        // Common strategy: Replace if entry.depth <= depth

        let replace = match self.entries.get(idx).and_then(|e| e.as_ref()) {
            None => true,
            Some(entry) => {
                // If keys are different (collision), we might want to keep the deeper one?
                // Or just overwrite. Overwriting is standard for simple replacement.
                // If keys are same, overwrite if new depth is better.
                if entry.key == key {
                    depth >= entry.depth
                } else {
                    true // Collision, overwrite (or use buckets later)
                }
            }
        };

        if replace {
            if let Some(slot) = self.entries.get_mut(idx) {
                *slot = Some(TTEntry {
                    key,
                    depth,
                    score,
                    flag,
                    best_move,
                });
            }
        }
    }
}
