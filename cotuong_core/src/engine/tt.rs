use crate::engine::Move;

pub const MATE_THRESHOLD: i32 = 15_000;

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
            depth: 0,
            flag: TTFlag::Exact,
            age: 0,
        }
    }
}

/// A 2-tier cluster bucket to prevent hash collision thrashing:
/// - `entries[0]`: Depth-preferred slot (preserves deep search analysis long-term).
/// - `entries[1]`: Always-replace / Newest slot (keeps recent branch discoveries).
#[derive(Clone, Copy, Debug, Default)]
pub struct TTBucket {
    pub entries: [TTEntry; 2],
}

pub struct TranspositionTable {
    buckets: Vec<TTBucket>,
    mask: usize,
    generation: u8,
}

impl TranspositionTable {
    #[must_use]
    pub fn new(size_mb: usize) -> Self {
        let bucket_size = std::mem::size_of::<TTBucket>();
        let num_buckets = (size_mb * 1024 * 1024) / bucket_size;

        // Power of 2 size for efficient bitwise masking
        let mut size = 1;
        while size <= num_buckets {
            size *= 2;
        }
        size /= 2; // Keep strictly within memory limit

        if size == 0 {
            size = 512; // Minimum bucket count (1024 entries)
        }

        Self {
            buckets: vec![TTBucket::default(); size],
            mask: size - 1,
            generation: 0,
        }
    }

    /// Advances the generation counter for a new search round.
    /// This allows distinguishing fresh nodes from stale nodes from prior turns
    /// without wiping the transposition table.
    #[inline]
    pub fn new_search(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// Current generation of the table
    #[must_use]
    #[inline]
    pub fn generation(&self) -> u8 {
        self.generation
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

        for entry in &bucket.entries {
            if entry.key == key {
                let mut e = *entry;
                e.score = Self::score_from_tt(entry.score, ply);
                return Some(e);
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
    pub fn store(
        &mut self,
        key: u64,
        best_move: Option<Move>,
        score: i32,
        depth: u8,
        flag: TTFlag,
        ply: u8,
    ) {
        let idx = (key as usize) & self.mask;
        let gen = self.generation;
        let tt_score = Self::score_to_tt(score, ply);

        let Some(bucket) = self.buckets.get_mut(idx) else {
            return;
        };

        // 1. Check if key is already present in either slot
        for entry in &mut bucket.entries {
            if entry.key == key {
                let mv = best_move.or(entry.best_move);
                // Update if deeper search, same depth with exact flag, or stale generation
                if depth >= entry.depth || entry.age != gen || flag == TTFlag::Exact {
                    *entry = TTEntry {
                        key,
                        best_move: mv,
                        score: tt_score,
                        depth,
                        flag,
                        age: gen,
                    };
                }
                return;
            }
        }

        // 2. Key is new to this bucket. Choose which slot to overwrite.
        // Slot 0 is Depth-preferred; Slot 1 is Always-replace (Newest).
        let slot0_stale = bucket.entries[0].age != gen;
        let slot0_empty = bucket.entries[0].key == 0;

        if slot0_empty || slot0_stale || depth >= bucket.entries[0].depth {
            // Overwrite Depth-preferred slot 0
            bucket.entries[0] = TTEntry {
                key,
                best_move,
                score: tt_score,
                depth,
                flag,
                age: gen,
            };
        } else {
            // Overwrite Always-replace slot 1
            bucket.entries[1] = TTEntry {
                key,
                best_move,
                score: tt_score,
                depth,
                flag,
                age: gen,
            };
        }
    }

    /// Clears all entries in the transposition table and resets generation to 0.
    pub fn clear(&mut self) {
        for bucket in &mut self.buckets {
            bucket.entries[0] = TTEntry::default();
            bucket.entries[1] = TTEntry::default();
        }
        self.generation = 0;
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
        let mut tt = TranspositionTable::new(1);
        let key1 = 0x1234_5678_90AB_CDEF;
        let key2 = 0xFEDC_BA09_8765_4321;

        tt.new_search();
        tt.store(key1, None, 100, 4, TTFlag::Exact, 0);

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
        tt.store(key2, None, 200, 5, TTFlag::Exact, 0);
        assert!(tt.probe(key2, 0).is_some());
    }

    #[test]
    fn test_tt_clear() {
        let mut tt = TranspositionTable::new(1);
        let key = 0xAAAA_BBBB_CCCC_DDDD;
        tt.store(key, None, 50, 3, TTFlag::Exact, 0);
        assert!(tt.probe(key, 0).is_some());

        tt.clear();
        assert!(tt.probe(key, 0).is_none());
        assert_eq!(tt.generation(), 0);
    }
}
