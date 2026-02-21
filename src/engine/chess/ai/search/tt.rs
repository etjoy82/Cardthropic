use crate::game::{encode_fen, ChessMove, ChessPosition};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Debug, Clone, Copy)]
pub struct Entry {
    pub depth: u8,
    pub score: i32,
    pub best_move: Option<ChessMove>,
    pub bound: Bound,
}

pub struct TranspositionTable {
    entries: HashMap<String, Entry>,
    capacity: usize,
}

impl TranspositionTable {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            capacity: capacity.max(1024),
        }
    }

    pub fn probe(&self, position: &ChessPosition) -> Option<Entry> {
        self.entries.get(&encode_fen(position)).copied()
    }

    pub fn store(&mut self, position: &ChessPosition, entry: Entry) {
        if self.entries.len() >= self.capacity {
            self.entries.clear();
        }
        let _ = self.entries.insert(encode_fen(position), entry);
    }
}
