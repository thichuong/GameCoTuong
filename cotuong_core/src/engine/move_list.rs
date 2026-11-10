use crate::engine::Move;
use std::ops::Index;

// Max moves in Xiangqi is typically < 100. 128 is safe.
const MAX_MOVES: usize = 128;

pub struct MoveList {
    pub moves: [Move; MAX_MOVES],
    pub count: usize,
}

impl Default for MoveList {
    fn default() -> Self {
        Self {
            moves: [Move::default(); MAX_MOVES],
            count: 0,
        }
    }
}

impl MoveList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, mv: Move) {
        if self.count < self.moves.len() {
            if let Some(slot) = self.moves.get_mut(self.count) {
                *slot = mv;
                self.count += 1;
            }
        } else {
            // In debug builds, we want to know if we are overflowing limits.
            debug_assert!(false, "MoveList overflow! Max moves: {}", MAX_MOVES);
        }
    }

    pub const fn truncate(&mut self, len: usize) {
        if len < self.count {
            self.count = len;
        }
    }

    pub const fn len(&self) -> usize {
        self.count
    }

    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        if a < self.count && b < self.count {
            self.moves.swap(a, b);
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Move> {
        self.moves.get(0..self.count).unwrap_or(&[]).iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Move> {
        self.moves
            .get_mut(0..self.count)
            .unwrap_or(&mut [])
            .iter_mut()
    }

    // Helper for sorting without heap allocation
    pub fn sort_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&Move, &Move) -> std::cmp::Ordering,
    {
        if let Some(slice) = self.moves.get_mut(0..self.count) {
            slice.sort_unstable_by(|a, b| compare(a, b));
        }
    }

    /// Incrementally selects the highest-scoring move from `start_idx..self.count`,
    /// swaps it into `start_idx`, and returns it. Avoids sorting the entire list
    /// when a beta-cutoff occurs early.
    #[inline]
    pub fn pick_next_best(&mut self, start_idx: usize) -> Option<Move> {
        let slice = self.moves.get_mut(start_idx..self.count)?;
        let first = slice.first()?;
        let mut best_offset = 0;
        let mut best_score = first.score;

        for (idx, mv) in slice.iter().enumerate().skip(1) {
            if mv.score > best_score {
                best_score = mv.score;
                best_offset = idx;
            }
        }

        if best_offset != 0 {
            slice.swap(0, best_offset);
        }

        slice.first().copied()
    }
    #[allow(clippy::indexing_slicing)]
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&Move) -> bool,
    {
        let mut i = 0;
        while i < self.count {
            if f(&self.moves[i]) {
                i += 1;
            } else {
                // Remove element at i by swapping with last element
                self.count -= 1;
                #[allow(clippy::indexing_slicing)]
                {
                    self.moves[i] = self.moves[self.count];
                }
            }
        }
    }
}

// Implement IntoIterator for &MoveList
impl<'a> IntoIterator for &'a MoveList {
    type Item = &'a Move;
    type IntoIter = std::slice::Iter<'a, Move>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// Implement Index for convenience
impl Index<usize> for MoveList {
    type Output = Move;

    fn index(&self, index: usize) -> &Self::Output {
        self.moves.get(index).unwrap_or(&self.moves[0]) // Fallback to 0th element (dummy) instead of panic
    }
}

// Implement IntoIterator for MoveList (consuming)
impl IntoIterator for MoveList {
    type Item = Move;
    type IntoIter = std::iter::Take<std::array::IntoIter<Move, MAX_MOVES>>;

    fn into_iter(self) -> Self::IntoIter {
        self.moves.into_iter().take(self.count)
    }
}
