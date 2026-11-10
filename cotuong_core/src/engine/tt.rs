use crate::engine::Move;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};

pub const MATE_THRESHOLD: i32 = 15_000;
pub const NO_EVAL: i16 = i16::MIN;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TTFlag {
    Exact,
    LowerBound, // Beta cutoff (fail-high)
    UpperBound, // Alpha cutoff (fail-low)
}

#[derive(Clone, Copy, Debug)]
pub struct TTEntry {
    pub key: u64,
    pub best_move: Option<Move>,
    pub score: i32,
    pub static_eval: i16,
    pub depth: u8,
    pub flag: TTFlag,
    pub age: u8,
}

impl Default for TTEntry {
    fn default() -> Self {
        Self {
            key: 0,
            best_move: None,
            score: 0,
            static_eval: NO_EVAL,
            depth: 0,
            flag: TTFlag::Exact,
            age: 0,
        }
    }
}

/// Packs an optional Move into 16 bits.
/// Returns 0xFFFF if None.
#[inline]
fn pack_move(mv: Option<Move>) -> u16 {
    match mv {
        Some(m) => {
            ((m.from_row as u16 & 0xF) << 12)
                | ((m.from_col as u16 & 0xF) << 8)
                | ((m.to_row as u16 & 0xF) << 4)
                | (m.to_col as u16 & 0xF)
        }
        None => 0xFFFF,
    }
}

/// Unpacks a 16-bit packed move.
#[inline]
fn unpack_move(packed: u16) -> Option<Move> {
    if packed == 0xFFFF {
        None
    } else {
        Some(Move {
            from_row: ((packed >> 12) & 0xF) as u8,
            from_col: ((packed >> 8) & 0xF) as u8,
            to_row: ((packed >> 4) & 0xF) as u8,
            to_col: (packed & 0xF) as u8,
            score: 0,
        })
    }
}

/// Packs score (32 bits), static_eval (16 bits), and best_move (16 bits) into 64 bits.
#[inline]
fn pack_data_low(score: i32, static_eval: i16, best_move: Option<Move>) -> u64 {
    #[allow(clippy::cast_sign_loss)]
    let s_u32 = score as u32;
    #[allow(clippy::cast_sign_loss)]
    let ev_u16 = static_eval as u16;
    let mv_u16 = pack_move(best_move);
    ((mv_u16 as u64) << 48) | ((ev_u16 as u64) << 32) | (s_u32 as u64)
}

/// Unpacks score, static_eval, and best_move from 64 bits.
#[inline]
fn unpack_data_low(val: u64) -> (i32, i16, Option<Move>) {
    #[allow(clippy::cast_possible_wrap)]
    let score = (val & 0xFFFF_FFFF) as u32 as i32;
    #[allow(clippy::cast_possible_wrap)]
    let static_eval = ((val >> 32) & 0xFFFF) as u16 as i16;
    let best_move = unpack_move(((val >> 48) & 0xFFFF) as u16);
    (score, static_eval, best_move)
}

/// Packs depth (8 bits), flag (8 bits), and age (8 bits) into 32 bits.
#[inline]
fn pack_data_high(depth: u8, flag: TTFlag, age: u8) -> u32 {
    let f = match flag {
        TTFlag::Exact => 0u32,
        TTFlag::LowerBound => 1u32,
        TTFlag::UpperBound => 2u32,
    };
    ((age as u32) << 16) | (f << 8) | (depth as u32)
}

/// Unpacks depth, flag, and age from 32 bits.
#[inline]
fn unpack_data_high(val: u32) -> (u8, TTFlag, u8) {
    let depth = (val & 0xFF) as u8;
    let flag = match (val >> 8) & 0xFF {
        1 => TTFlag::LowerBound,
        2 => TTFlag::UpperBound,
        _ => TTFlag::Exact,
    };
    let age = ((val >> 16) & 0xFF) as u8;
    (depth, flag, age)
}

/// Lockless atomic transposition table entry.
pub struct AtomicTTEntry {
    pub key: AtomicU64,
    pub data_low: AtomicU64,
    pub data_high: AtomicU32,
}

impl Default for AtomicTTEntry {
    fn default() -> Self {
        Self {
            key: AtomicU64::new(0),
            data_low: AtomicU64::new(0),
            data_high: AtomicU32::new(0),
        }
    }
}

impl AtomicTTEntry {
    #[inline]
    pub fn read(&self, expected_key: u64) -> Option<TTEntry> {
        let key = self.key.load(Ordering::Acquire);
        if key != expected_key || key == 0 {
            return None;
        }

        let low = self.data_low.load(Ordering::Relaxed);
        let high = self.data_high.load(Ordering::Relaxed);

        // Verify key again to protect against concurrent torn-write
        let verify_key = self.key.load(Ordering::Acquire);
        if verify_key != expected_key {
            return None;
        }

        let (score, static_eval, best_move) = unpack_data_low(low);
        let (depth, flag, age) = unpack_data_high(high);

        Some(TTEntry {
            key,
            best_move,
            score,
            static_eval,
            depth,
            flag,
            age,
        })
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn write(
        &self,
        key: u64,
        best_move: Option<Move>,
        score: i32,
        static_eval: i16,
        depth: u8,
        flag: TTFlag,
        age: u8,
    ) {
        let low = pack_data_low(score, static_eval, best_move);
        let high = pack_data_high(depth, flag, age);

        // Store data first, then publish the key with Release ordering
        self.data_low.store(low, Ordering::Relaxed);
        self.data_high.store(high, Ordering::Relaxed);
        self.key.store(key, Ordering::Release);
    }
}

/// A 2-tier cluster bucket to prevent hash collision thrashing:
/// - `entries[0]`: Depth-preferred slot (preserves deep search analysis long-term).
/// - `entries[1]`: Always-replace / Newest slot (keeps recent branch discoveries).
#[derive(Default)]
pub struct TTBucket {
    pub entries: [AtomicTTEntry; 2],
}

pub struct TranspositionTable {
    buckets: Vec<TTBucket>,
    mask: usize,
    generation: AtomicU8,
}

impl TranspositionTable {
    #[must_use]
    pub fn new(size_mb: usize) -> Self {
        // Cap size_mb to maximum 2048MB (2GB)
        let safe_mb = size_mb.min(2048);
        let bucket_size = std::mem::size_of::<TTBucket>();
        let num_buckets = ((safe_mb as u64 * 1024 * 1024) / bucket_size as u64) as usize;

        // Power of 2 size for efficient bitwise masking
        let mut size = 1;
        while size <= num_buckets {
            size *= 2;
        }
        size /= 2; // Keep strictly within memory limit

        if size == 0 {
            size = 512; // Minimum bucket count (1024 entries)
        }

        let mut buckets = Vec::with_capacity(size);
        for _ in 0..size {
            buckets.push(TTBucket::default());
        }

        Self {
            buckets,
            mask: size - 1,
            generation: AtomicU8::new(0),
        }
    }

    /// Advances the generation counter for a new search round.
    /// This allows distinguishing fresh nodes from stale nodes from prior turns
    /// without wiping the transposition table.
    #[inline]
    pub fn new_search(&self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
    }

    /// Current generation of the table
    #[must_use]
    #[inline]
    pub fn generation(&self) -> u8 {
        self.generation.load(Ordering::Relaxed)
    }

    /// Converts a search score to a ply-independent TT score for mate distance preservation.
    #[must_use]
    #[inline]
    pub fn score_to_tt(score: i32, ply: u8) -> i32 {
        if score >= MATE_THRESHOLD {
            score + i32::from(ply)
        } else if score <= -MATE_THRESHOLD {
            score - i32::from(ply)
        } else {
            score
        }
    }

    /// Converts a ply-independent TT score back to a current-search ply score.
    #[must_use]
    #[inline]
    pub fn score_from_tt(score: i32, ply: u8) -> i32 {
        if score >= MATE_THRESHOLD {
            score - i32::from(ply)
        } else if score <= -MATE_THRESHOLD {
            score + i32::from(ply)
        } else {
            score
        }
    }

    /// Probes the transposition table for an existing entry matching the given 64-bit key.
    /// Returns the entry with its score converted to the given ply distance.
    pub fn probe(&self, key: u64, ply: u8) -> Option<TTEntry> {
        let idx = (key as usize) & self.mask;
        let bucket = self.buckets.get(idx)?;

        for entry_atomic in &bucket.entries {
            if let Some(mut entry) = entry_atomic.read(key) {
                entry.score = Self::score_from_tt(entry.score, ply);
                return Some(entry);
            }
        }
        None
    }

    /// Quick lookup for the best move associated with a position key.
    pub fn get_move(&self, key: u64) -> Option<Move> {
        self.probe(key, 0).and_then(|e| e.best_move)
    }

    /// Stores a position analysis result in the transposition table.
    /// Uses a 2-tier replacement scheme:
    /// - If matching key already exists: updates in place (preserving best_move if not provided).
    /// - Otherwise: updates Depth-preferred slot if incoming depth is deeper or current slot is stale;
    ///   or replaces Always-replace slot.
    #[allow(clippy::too_many_arguments)]
    pub fn store(
        &self,
        key: u64,
        best_move: Option<Move>,
        score: i32,
        static_eval: i16,
        depth: u8,
        flag: TTFlag,
        ply: u8,
    ) {
        let idx = (key as usize) & self.mask;
        let gen = self.generation();
        let tt_score = Self::score_to_tt(score, ply);

        let Some(bucket) = self.buckets.get(idx) else {
            return;
        };

        // 1. Check if key is already present in either slot
        for (i, entry_atomic) in bucket.entries.iter().enumerate() {
            if let Some(existing) = entry_atomic.read(key) {
                let mv = best_move.or(existing.best_move);
                let eval = if static_eval != NO_EVAL {
                    static_eval
                } else {
                    existing.static_eval
                };

                if depth >= existing.depth || existing.age != gen || flag == TTFlag::Exact {
                    if let Some(target) = bucket.entries.get(i) {
                        target.write(key, mv, tt_score, eval, depth, flag, gen);
                    }
                } else if static_eval != NO_EVAL && existing.static_eval == NO_EVAL {
                    if let Some(target) = bucket.entries.get(i) {
                        target.write(
                            key,
                            existing.best_move,
                            existing.score,
                            static_eval,
                            existing.depth,
                            existing.flag,
                            existing.age,
                        );
                    }
                }
                return;
            }
        }

        // 2. Key is new to this bucket. Choose which slot to overwrite.
        // Slot 0 is Depth-preferred; Slot 1 is Always-replace (Newest).
        let slot0_opt = bucket.entries.first().and_then(|e| {
            let k = e.key.load(Ordering::Acquire);
            if k == 0 {
                None
            } else {
                let high = e.data_high.load(Ordering::Relaxed);
                let (d, _, a) = unpack_data_high(high);
                Some((k, d, a))
            }
        });

        let overwrite_slot0 = match slot0_opt {
            None => true, // Slot 0 is empty
            Some((_, d, a)) => a != gen || depth >= d,
        };

        if overwrite_slot0 {
            if let Some(slot0) = bucket.entries.first() {
                slot0.write(key, best_move, tt_score, static_eval, depth, flag, gen);
            }
        } else if let Some(slot1) = bucket.entries.get(1) {
            slot1.write(key, best_move, tt_score, static_eval, depth, flag, gen);
        }
    }

    /// Clears all entries in the transposition table and resets generation to 0.
    pub fn clear(&self) {
        for bucket in &self.buckets {
            for entry in &bucket.entries {
                entry.key.store(0, Ordering::Relaxed);
                entry.data_low.store(0, Ordering::Relaxed);
                entry.data_high.store(0, Ordering::Relaxed);
            }
        }
        self.generation.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tt_score_mate_normalization() {
        let ply = 3;
        let mate_score = 300_000 - 3; // Mate in 2 from root
        let tt_val = TranspositionTable::score_to_tt(mate_score, ply);
        assert_eq!(tt_val, 300_000);

        // Probe next turn at root ply 0
        let restored_root = TranspositionTable::score_from_tt(tt_val, 0);
        assert_eq!(restored_root, 300_000);

        // Probe next turn at ply 1
        let restored_ply1 = TranspositionTable::score_from_tt(tt_val, 1);
        assert_eq!(restored_ply1, 299_999);
    }

    #[test]
    fn test_tt_generation_and_replacement() {
        let tt = TranspositionTable::new(1);
        let key1 = 0x1234_5678_90AB_CDEF;
        let key2 = 0xFEDC_BA09_8765_4321;

        tt.new_search();
        tt.store(key1, None, 100, NO_EVAL, 4, TTFlag::Exact, 0);

        let probed = tt.probe(key1, 0);
        assert!(probed.is_some());
        assert_eq!(probed.unwrap().score, 100);
        assert_eq!(probed.unwrap().depth, 4);

        // Advance generation: entry from generation 1 should be retained
        tt.new_search();
        let probed_after_gen = tt.probe(key1, 0);
        assert!(probed_after_gen.is_some());
        assert_eq!(probed_after_gen.unwrap().score, 100);

        // Store a second key mapping to the same bucket
        tt.store(key2, None, 200, NO_EVAL, 5, TTFlag::Exact, 0);
        assert!(tt.probe(key2, 0).is_some());
    }

    #[test]
    fn test_tt_clear() {
        let tt = TranspositionTable::new(1);
        let key = 0xAAAA_BBBB_CCCC_DDDD;
        tt.store(key, None, 50, NO_EVAL, 3, TTFlag::Exact, 0);
        assert!(tt.probe(key, 0).is_some());

        tt.clear();
        assert!(tt.probe(key, 0).is_none());
        assert_eq!(tt.generation(), 0);
    }

    #[test]
    fn test_tt_concurrent_read_write() {
        use std::sync::Arc;
        use std::thread;

        let tt = Arc::new(TranspositionTable::new(2));
        let mut handles = Vec::new();

        // Spawn multiple concurrent threads storing and probing
        for t in 0..4 {
            let tt_clone = tt.clone();
            handles.push(thread::spawn(move || {
                for i in 0..1000 {
                    let key = 0x1000_0000 + (i % 64) + (t * 100);
                    tt_clone.store(
                        key,
                        Some(Move {
                            from_row: 0,
                            from_col: (i % 9) as u8,
                            to_row: 1,
                            to_col: (i % 9) as u8,
                            score: 0,
                        }),
                        (i as i32) * 10,
                        100,
                        (i % 16) as u8,
                        TTFlag::Exact,
                        0,
                    );
                    let _ = tt_clone.probe(key, 0);
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }
    }
}
