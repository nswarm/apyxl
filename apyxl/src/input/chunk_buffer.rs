use std::cell::RefCell;

use crate::input::{Data, Input};
use crate::model::Chunk;

/// Stores data across multiple in-memory chunks.
#[derive(Default)]
pub struct ChunkBuffer {
    chunks: Vec<(Chunk, Data)>,
    index: RefCell<usize>,
}

impl ChunkBuffer {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            index: RefCell::new(0),
        }
    }

    pub fn add_chunk(&mut self, chunk: Chunk, data: impl ToString) {
        self.chunks.push((chunk, data.to_string()))
    }
}

impl Input for ChunkBuffer {
    fn next_chunk(&self) -> Option<(&Chunk, &Data)> {
        let index = *self.index.borrow();
        *self.index.borrow_mut() = index + 1;
        match self.chunks.get(index) {
            None => None,
            Some((chunk, data)) => Some((chunk, data)),
        }
    }
}
