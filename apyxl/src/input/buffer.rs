use std::cell::RefCell;

use crate::input::{Data, Input};
use crate::model::Chunk;

/// Stores all data in a single in-memory chunk.
#[derive(Default)]
pub struct Buffer {
    chunk: Chunk,
    data: Data,
    read: RefCell<bool>,
}

impl Buffer {
    pub fn new(data: impl ToString) -> Self {
        Self {
            chunk: Chunk::default(),
            data: data.to_string(),
            read: RefCell::new(false),
        }
    }
}

impl Input for Buffer {
    fn next_chunk(&self) -> Option<(&Chunk, &Data)> {
        if *self.read.borrow() {
            return None;
        }
        *self.read.borrow_mut() = true;
        Some((&self.chunk, &self.data))
    }
}
