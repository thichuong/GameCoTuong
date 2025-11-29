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
        }
        // If we exceed MAX_MOVES, we just ignore (or could panic in debug).
        // For a chess engine, silent truncation of very rare cases might be acceptable
        // if we prioritize speed, but ideally we should ensure 128 is enough.
    }

    pub fn truncate(&mut self, len: usize) {
        if len < self.count {
            self.count = len;
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
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

    // Helper for sorting
    pub fn sort_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&Move, &Move) -> std::cmp::Ordering,
    {
        if let Some(slice) = self.moves.get_mut(0..self.count) {
            slice.sort_by(|a, b| compare(a, b));
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
        #[allow(clippy::expect_used)]
        self.moves.get(index).expect("MoveList index out of bounds")
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
