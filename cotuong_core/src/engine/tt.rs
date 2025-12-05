use crate::engine::Move;

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
}

impl Default for TTEntry {
    fn default() -> Self {
        Self {
            key: 0,
            best_move: None,
            score: 0,
            depth: 0,
            flag: TTFlag::Exact,
        }
    }
}

pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    mask: usize,
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let entry_size = std::mem::size_of::<TTEntry>();
        let num_entries = (size_mb * 1024 * 1024) / entry_size;

        // Power of 2 size for efficient masking
        let mut size = 1;
        while size <= num_entries {
            size *= 2;
        }
        size /= 2; // Keep it within memory limit

        if size == 0 {
            size = 1024; // Minimum size
        }

        Self {
            entries: vec![TTEntry::default(); size],
            mask: size - 1,
        }
    }

    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        let idx = (key as usize) & self.mask;
        let entry = self.entries[idx];
        if entry.key == key {
            Some(entry)
        } else {
            None
        }
    }

    pub fn get_move(&self, key: u64) -> Option<Move> {
        self.probe(key).and_then(|e| e.best_move)
    }

    pub fn store(
        &mut self,
        key: u64,
        best_move: Option<Move>,
        score: i32,
        depth: u8,
        flag: TTFlag,
    ) {
        let idx = (key as usize) & self.mask;
        let entry = &mut self.entries[idx];

        // Replacement scheme: Always replace if different key (collision) or deeper search
        // Simple "always replace" is often good enough or "replace if depth >= entry.depth"
        if entry.key != key || depth >= entry.depth {
            *entry = TTEntry {
                key,
                best_move,
                score,
                depth,
                flag,
            };
        }
    }

    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            *entry = TTEntry::default();
        }
    }
}
